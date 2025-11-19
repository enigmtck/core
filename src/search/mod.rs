mod indexer;
mod query;
mod schema;

pub use query::{
    ActorSearchResult, ObjectSearchResult, SearchContext, SearchFilters,
    SearchResults, SortOrder,
};

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tantivy::Index;

use crate::models::actors::Actor;
use crate::models::objects::Object;

/// Maximum number of retry attempts for lock contention (real-time indexing)
const MAX_RETRIES: u32 = 20;
/// Maximum retries for bulk operations (more patient)
const MAX_RETRIES_BULK: u32 = 50;
/// Initial backoff delay in milliseconds
const INITIAL_BACKOFF_MS: u64 = 50;
/// Initial backoff for bulk operations (more patient)
const INITIAL_BACKOFF_MS_BULK: u64 = 50;

/// Check if an error is due to lock contention
fn is_lock_contention_error(err: &anyhow::Error) -> bool {
    err.to_string().contains("LockBusy") || err.to_string().contains("Failed to acquire index lock")
}

/// Retry a function with exponential backoff on lock contention
fn retry_on_lock_contention<F, T>(f: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    retry_with_config(f, MAX_RETRIES, INITIAL_BACKOFF_MS)
}

/// Retry a function with exponential backoff on lock contention (bulk operations - more patient)
fn retry_on_lock_contention_bulk<F, T>(f: F) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    retry_with_config(f, MAX_RETRIES_BULK, INITIAL_BACKOFF_MS_BULK)
}

/// Generic retry implementation with configurable parameters
fn retry_with_config<F, T>(mut f: F, max_retries: u32, initial_backoff_ms: u64) -> Result<T>
where
    F: FnMut() -> Result<T>,
{
    let mut attempt = 0;
    loop {
        match f() {
            Ok(result) => return Ok(result),
            Err(e) if is_lock_contention_error(&e) && attempt < max_retries => {
                attempt += 1;
                // Cap exponential backoff at 5 seconds to avoid extremely long waits
                let backoff_ms = (initial_backoff_ms * 2u64.pow(attempt - 1)).min(5000);
                log::debug!(
                    "Index lock contention detected, retrying in {}ms (attempt {}/{})",
                    backoff_ms,
                    attempt,
                    max_retries
                );
                thread::sleep(Duration::from_millis(backoff_ms));
            }
            Err(e) => return Err(e),
        }
    }
}

/// Main search index manager
pub struct SearchIndex {
    objects_index: Index,
    actors_index: Index,
    index_dir: PathBuf,
}

impl SearchIndex {
    /// Create or open search indexes at the specified path
    pub fn new<P: AsRef<Path>>(index_path: P) -> Result<Self> {
        let index_dir = index_path.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&index_dir)
            .with_context(|| format!("Failed to create index directory: {:?}", index_dir))?;

        // Create subdirectories for each index
        let objects_dir = index_dir.join("objects");
        let actors_dir = index_dir.join("actors");

        std::fs::create_dir_all(&objects_dir)?;
        std::fs::create_dir_all(&actors_dir)?;

        // Create or open indexes
        let objects_schema = schema::create_objects_schema();
        let objects_index = Index::open_or_create(
            tantivy::directory::MmapDirectory::open(&objects_dir)?,
            objects_schema,
        )?;

        let actors_schema = schema::create_actors_schema();
        let actors_index = Index::open_or_create(
            tantivy::directory::MmapDirectory::open(&actors_dir)?,
            actors_schema,
        )?;

        Ok(Self {
            objects_index,
            actors_index,
            index_dir,
        })
    }

    /// Index an object (post/note/article/question)
    pub fn index_object(&self, object: &Object) -> Result<()> {
        retry_on_lock_contention(|| {
            let schema = self.objects_index.schema();
            let mut writer = self.objects_index.writer(15_000_000)?;  // Tantivy minimum
            indexer::index_object(&mut writer, object, &schema)?;
            writer.commit()?;
            Ok(())
        })
    }

    /// Index an actor (user/profile)
    pub fn index_actor(&self, actor: &Actor) -> Result<()> {
        retry_on_lock_contention(|| {
            let schema = self.actors_index.schema();
            let mut writer = self.actors_index.writer(15_000_000)?;  // 3 MB instead of 15 MB
            indexer::index_actor(&mut writer, actor, &schema)?;
            writer.commit()?;
            Ok(())
        })
    }

    /// Delete an object from the index
    pub fn delete_object(&self, object_id: &str) -> Result<()> {
        let object_id = object_id.to_string(); // Clone for closure
        retry_on_lock_contention(|| {
            let schema = self.objects_index.schema();
            let mut writer = self.objects_index.writer(15_000_000)?;  // 3 MB instead of 15 MB
            indexer::delete_object(&mut writer, &object_id, &schema)?;
            writer.commit()?;
            Ok(())
        })
    }

    /// Delete an actor from the index
    pub fn delete_actor(&self, actor_id: &str) -> Result<()> {
        let actor_id = actor_id.to_string(); // Clone for closure
        retry_on_lock_contention(|| {
            let schema = self.actors_index.schema();
            let mut writer = self.actors_index.writer(15_000_000)?;  // 3 MB instead of 15 MB
            indexer::delete_actor(&mut writer, &actor_id, &schema)?;
            writer.commit()?;
            Ok(())
        })
    }

    /// Perform a unified search across all indexes
    pub fn search(
        &self,
        query: &str,
        context: &SearchContext,
        filters: &SearchFilters,
        limit: usize,
        offset: usize,
    ) -> Result<SearchResults> {
        let objects = query::search_objects(&self.objects_index, query, context, filters, limit, offset)
            .unwrap_or_else(|e| {
                log::error!("Error searching objects: {:#?}", e);
                Vec::new()
            });

        let actors = query::search_actors(&self.actors_index, query, context, filters, limit, offset)
            .unwrap_or_else(|e| {
                log::error!("Error searching actors: {:#?}", e);
                Vec::new()
            });

        Ok(SearchResults {
            objects,
            actors,
        })
    }

    /// Search only objects
    pub fn search_objects(
        &self,
        query: &str,
        context: &SearchContext,
        filters: &SearchFilters,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ObjectSearchResult>> {
        query::search_objects(&self.objects_index, query, context, filters, limit, offset)
    }

    /// Search only actors
    pub fn search_actors(
        &self,
        query: &str,
        context: &SearchContext,
        filters: &SearchFilters,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<ActorSearchResult>> {
        query::search_actors(&self.actors_index, query, context, filters, limit, offset)
    }

    /// Get index statistics
    pub fn get_stats(&self) -> Result<IndexStats> {
        let objects_reader = self.objects_index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Force reload to pick up recent commits
        objects_reader.reload()?;
        let objects_count = objects_reader.searcher().num_docs() as usize;

        let actors_reader = self.actors_index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        // Force reload to pick up recent commits
        actors_reader.reload()?;
        let actors_count = actors_reader.searcher().num_docs() as usize;

        Ok(IndexStats {
            objects_count,
            actors_count,
            index_path: self.index_dir.clone(),
        })
    }

    /// Optimize all indexes (merge segments)
    /// Note: Tantivy automatically handles segment merging in the background
    pub fn optimize(&self) -> Result<()> {
        log::info!("Search index optimization is handled automatically by Tantivy");
        Ok(())
    }

    /// Commit any pending changes (normally done automatically per operation)
    /// Note: With on-demand writers, commits happen automatically per operation
    pub fn commit(&self) -> Result<()> {
        // No-op since we commit on each operation with on-demand writers
        Ok(())
    }

    /// Bulk index objects (for initial indexing or reindexing)
    pub fn bulk_index_objects(&self, objects: &[Object]) -> Result<()> {
        log::debug!("Bulk indexing {} objects", objects.len());
        retry_on_lock_contention_bulk(|| {
            let schema = self.objects_index.schema();
            let mut writer = self.objects_index.writer(15_000_000)?;  // 3 MB instead of 15 MB

            let mut indexed = 0;
            for object in objects {
                if let Err(e) = indexer::index_object(&mut writer, object, &schema) {
                    log::warn!("Failed to index object {}: {:#?}", object.id, e);
                } else {
                    indexed += 1;
                }
            }

            writer.commit()?;
            log::debug!("Committed batch of {} objects (indexed {} successfully)", objects.len(), indexed);
            Ok(())
        })
    }

    /// Bulk delete objects from index
    pub fn bulk_delete_objects(&self, object_ids: &[String]) -> Result<()> {
        log::debug!("Bulk deleting {} objects", object_ids.len());
        retry_on_lock_contention_bulk(|| {
            let schema = self.objects_index.schema();
            let mut writer = self.objects_index.writer(15_000_000)?;  // 3 MB instead of 15 MB

            let mut deleted = 0;
            for object_id in object_ids {
                if let Err(e) = indexer::delete_object(&mut writer, object_id, &schema) {
                    log::warn!("Failed to delete object {}: {:#?}", object_id, e);
                } else {
                    deleted += 1;
                }
            }

            writer.commit()?;
            log::debug!("Committed batch deletion of {} objects ({} successfully deleted)", object_ids.len(), deleted);
            Ok(())
        })
    }

    /// Bulk index actors (for initial indexing or reindexing)
    pub fn bulk_index_actors(&self, actors: &[Actor]) -> Result<()> {
        log::debug!("Bulk indexing {} actors", actors.len());
        retry_on_lock_contention_bulk(|| {
            let schema = self.actors_index.schema();
            let mut writer = self.actors_index.writer(15_000_000)?;  // 3 MB instead of 15 MB

            let mut indexed = 0;
            for actor in actors {
                if let Err(e) = indexer::index_actor(&mut writer, actor, &schema) {
                    log::warn!("Failed to index actor {}: {:#?}", actor.id, e);
                } else {
                    indexed += 1;
                }
            }

            writer.commit()?;
            log::debug!("Committed batch of {} actors (indexed {} successfully)", actors.len(), indexed);
            Ok(())
        })
    }

    /// Index all objects from database (for full reindex)
    /// Returns total number of objects indexed
    pub async fn index_all_objects<C: crate::db::runner::DbRunner>(
        &self,
        conn: &C,
        batch_size: i64,
        mut progress_callback: impl FnMut(usize),
    ) -> Result<usize> {
        use diesel::prelude::*;
        use crate::schema::objects;

        let mut last_id = 0i32;
        let mut total_indexed = 0usize;

        loop {
            let current_last_id = last_id;
            let batch: Vec<Object> = conn
                .run(move |c| {
                    use crate::models::objects::ObjectType;
                    objects::table
                        .filter(objects::as_deleted.is_null())
                        .filter(objects::as_type.ne(ObjectType::Tombstone))
                        .filter(objects::id.gt(current_last_id))
                        .order(objects::id)
                        .limit(batch_size)
                        .load::<Object>(c)
                })
                .await?;

            if batch.is_empty() {
                break;
            }

            // Update last_id for next iteration
            if let Some(last_obj) = batch.last() {
                last_id = last_obj.id;
            }

            let batch_count = batch.len();
            self.bulk_index_objects(&batch)?;
            total_indexed += batch_count;
            progress_callback(total_indexed);

            // If we got fewer than batch_size, we're done
            if batch_count < batch_size as usize {
                break;
            }
        }

        Ok(total_indexed)
    }

    /// Index all actors from database (for full reindex)
    /// Returns total number of actors indexed
    pub async fn index_all_actors<C: crate::db::runner::DbRunner>(
        &self,
        conn: &C,
        batch_size: i64,
        mut progress_callback: impl FnMut(usize),
    ) -> Result<usize> {
        use diesel::prelude::*;
        use crate::schema::actors;

        let mut last_id = 0i32;
        let mut total_indexed = 0usize;

        loop {
            let current_last_id = last_id;
            let batch: Vec<Actor> = conn
                .run(move |c| {
                    actors::table
                        .filter(actors::as_discoverable.eq(true))
                        .filter(actors::id.gt(current_last_id))
                        .order(actors::id)
                        .limit(batch_size)
                        .load::<Actor>(c)
                })
                .await?;

            if batch.is_empty() {
                break;
            }

            // Update last_id for next iteration
            if let Some(last_actor) = batch.last() {
                last_id = last_actor.id;
            }

            let batch_count = batch.len();
            self.bulk_index_actors(&batch)?;
            total_indexed += batch_count;
            progress_callback(total_indexed);

            // If we got fewer than batch_size, we're done
            if batch_count < batch_size as usize {
                break;
            }
        }

        Ok(total_indexed)
    }

    /// Index objects updated since a specific time (for incremental indexing)
    /// Returns (total_indexed, last_updated_at)
    /// The last_updated_at can be used to update checkpoint after each batch
    pub async fn index_objects_updated_since<C: crate::db::runner::DbRunner>(
        &self,
        conn: &C,
        since: chrono::DateTime<chrono::Utc>,
        batch_size: i64,
        mut checkpoint_callback: impl FnMut(chrono::DateTime<chrono::Utc>),
    ) -> Result<usize> {
        use diesel::prelude::*;
        use crate::schema::objects;

        let mut last_updated_at = since;
        let mut total_indexed = 0usize;

        // Create ONE writer for the entire update session (not per batch!)
        // This eliminates lock contention and reduces memory allocations
        let schema = self.objects_index.schema();
        let mut writer = retry_on_lock_contention(|| {
            self.objects_index.writer(50_000_000)  // Larger buffer since it's long-lived
                .context("Failed to create index writer")
        })?;

        loop {
            let current_updated_at = last_updated_at;
            let batch: Vec<Object> = conn
                .run(move |c| {
                    use crate::models::objects::ObjectType;
                    objects::table
                        .filter(objects::as_deleted.is_null())
                        .filter(objects::as_type.ne(ObjectType::Tombstone))
                        .filter(objects::updated_at.gt(current_updated_at))
                        .order(objects::updated_at.asc())
                        .limit(batch_size)
                        .load::<Object>(c)
                })
                .await?;

            if batch.is_empty() {
                break;
            }

            // Update last_updated_at for next iteration (keyset pagination)
            if let Some(last_obj) = batch.last() {
                last_updated_at = last_obj.updated_at;
            }

            let batch_count = batch.len();

            // Index batch using existing writer (no new allocation!)
            for object in &batch {
                if let Err(e) = indexer::index_object(&mut writer, object, &schema) {
                    log::warn!("Failed to index object {}: {:#?}", object.id, e);
                }
            }

            total_indexed += batch_count;

            log::debug!("Indexed batch of {} objects (total: {})", batch_count, total_indexed);

            // Update checkpoint after successful batch
            checkpoint_callback(last_updated_at);

            // If we got fewer than batch_size, we're done
            if batch_count < batch_size as usize {
                break;
            }
        }

        // Commit once at the end of the entire update session
        writer.commit()?;
        log::debug!("Committed all {} indexed objects", total_indexed);

        Ok(total_indexed)
    }

    /// Index actors updated since a specific time (for incremental indexing)
    /// Returns total number of actors indexed
    pub async fn index_actors_updated_since<C: crate::db::runner::DbRunner>(
        &self,
        conn: &C,
        since: chrono::DateTime<chrono::Utc>,
        batch_size: i64,
        mut checkpoint_callback: impl FnMut(chrono::DateTime<chrono::Utc>),
    ) -> Result<usize> {
        use diesel::prelude::*;
        use crate::schema::actors;

        let mut last_updated_at = since;
        let mut total_indexed = 0usize;

        // Create ONE writer for the entire update session (not per batch!)
        // This eliminates lock contention and reduces memory allocations
        let schema = self.actors_index.schema();
        let mut writer = retry_on_lock_contention(|| {
            self.actors_index.writer(50_000_000)  // Larger buffer since it's long-lived
                .context("Failed to create index writer")
        })?;

        loop {
            let current_updated_at = last_updated_at;
            let batch: Vec<Actor> = conn
                .run(move |c| {
                    actors::table
                        .filter(actors::as_discoverable.eq(true))
                        .filter(actors::updated_at.gt(current_updated_at))
                        .order(actors::updated_at.asc())
                        .limit(batch_size)
                        .load::<Actor>(c)
                })
                .await?;

            if batch.is_empty() {
                break;
            }

            // Update last_updated_at for next iteration (keyset pagination)
            if let Some(last_actor) = batch.last() {
                last_updated_at = last_actor.updated_at;
            }

            let batch_count = batch.len();

            // Index batch using existing writer (no new allocation!)
            for actor in &batch {
                if let Err(e) = indexer::index_actor(&mut writer, actor, &schema) {
                    log::warn!("Failed to index actor {}: {:#?}", actor.id, e);
                }
            }

            total_indexed += batch_count;

            log::debug!("Indexed batch of {} actors (total: {})", batch_count, total_indexed);

            // Update checkpoint after successful batch
            checkpoint_callback(last_updated_at);

            // If we got fewer than batch_size, we're done
            if batch_count < batch_size as usize {
                break;
            }
        }

        // Commit once at the end of the entire update session
        writer.commit()?;
        log::debug!("Committed all {} indexed actors", total_indexed);

        Ok(total_indexed)
    }

    /// Remove objects that have transitioned to Tombstone status
    /// Returns total number of objects removed from index
    async fn remove_tombstoned_objects<C: crate::db::runner::DbRunner>(
        &self,
        conn: &C,
        since: chrono::DateTime<chrono::Utc>,
        batch_size: i64,
    ) -> Result<usize> {
        use diesel::prelude::*;
        use crate::schema::objects;

        log::debug!("Starting tombstone removal process for objects updated since {}", since);

        let mut last_updated_at = since;
        let mut total_removed = 0usize;
        let mut batch_count = 0usize;

        // Create ONE writer for the entire removal session (not per batch!)
        let schema = self.objects_index.schema();
        let mut writer = retry_on_lock_contention(|| {
            self.objects_index.writer(50_000_000)  // Larger buffer since it's long-lived
                .context("Failed to create index writer")
        })?;

        loop {
            let current_updated_at = last_updated_at;
            log::debug!("Querying for tombstones updated since {}", current_updated_at);

            // Query for Tombstone objects that were updated since checkpoint
            let batch: Vec<Object> = conn
                .run(move |c| {
                    use crate::models::objects::ObjectType;
                    objects::table
                        .filter(objects::as_type.eq(ObjectType::Tombstone))
                        .filter(objects::updated_at.gt(current_updated_at))
                        .order(objects::updated_at.asc())
                        .limit(batch_size)
                        .load::<Object>(c)
                })
                .await?;

            if batch.is_empty() {
                log::debug!("No more tombstones found, finishing removal process");
                break;
            }

            batch_count += 1;
            log::debug!("Found {} tombstones in batch {}", batch.len(), batch_count);

            // Update last_updated_at for next iteration (keyset pagination)
            if let Some(last_obj) = batch.last() {
                last_updated_at = last_obj.updated_at;
            }

            // Delete tombstones using existing writer (no new allocation!)
            for object in &batch {
                let object_id = object.id.to_string();
                if let Err(e) = indexer::delete_object(&mut writer, &object_id, &schema) {
                    log::warn!("Failed to delete object {}: {:#?}", object_id, e);
                } else {
                    total_removed += 1;
                }
            }

            log::info!(
                "Removed {} tombstoned objects from index in batch {} (total removed: {})",
                batch.len(),
                batch_count,
                total_removed
            );

            // If we got fewer than batch_size, we're done
            if batch.len() < batch_size as usize {
                log::debug!("Batch size ({}) < limit ({}), finishing removal process", batch.len(), batch_size);
                break;
            }
        }

        // Commit all deletions at once
        writer.commit()?;
        log::info!("Tombstone removal complete: {} objects removed in {} batches", total_removed, batch_count);
        Ok(total_removed)
    }

    /// Perform incremental update with checkpoint management
    /// Returns (objects_indexed, actors_indexed)
    pub async fn incremental_update<C: crate::db::runner::DbRunner>(
        &self,
        conn: &C,
        batch_size: i64,
    ) -> Result<(usize, usize)> {
        use chrono::{DateTime, Utc};

        // Check if index is empty - if so, use larger batch size for better performance
        let stats = self.get_stats()?;
        let is_empty = stats.objects_count == 0 && stats.actors_count == 0;
        let effective_batch_size = if is_empty {
            log::info!("Index is empty, using large batch size (100000) for better performance");
            100000
        } else {
            batch_size
        };

        // Separate checkpoint file paths for objects and actors
        let objects_checkpoint_path = format!(
            "{}/search_index_objects_checkpoint.txt",
            *crate::MEDIA_DIR
        );
        let actors_checkpoint_path = format!(
            "{}/search_index_actors_checkpoint.txt",
            *crate::MEDIA_DIR
        );

        // Read last checkpoint time for objects (or use epoch if first run)
        let objects_last_checkpoint: DateTime<Utc> = if std::path::Path::new(&objects_checkpoint_path).exists() {
            if let Ok(checkpoint_str) = std::fs::read_to_string(&objects_checkpoint_path) {
                if let Ok(timestamp) = checkpoint_str.trim().parse::<i64>() {
                    DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now)
                } else {
                    log::warn!("Failed to parse objects checkpoint timestamp, using epoch");
                    DateTime::from_timestamp(0, 0).unwrap()
                }
            } else {
                log::warn!("Failed to read objects checkpoint file, using epoch");
                DateTime::from_timestamp(0, 0).unwrap()
            }
        } else {
            log::info!("No objects checkpoint file found, starting from epoch");
            DateTime::from_timestamp(0, 0).unwrap()
        };

        // Read last checkpoint time for actors (or use epoch if first run)
        let actors_last_checkpoint: DateTime<Utc> = if std::path::Path::new(&actors_checkpoint_path).exists() {
            if let Ok(checkpoint_str) = std::fs::read_to_string(&actors_checkpoint_path) {
                if let Ok(timestamp) = checkpoint_str.trim().parse::<i64>() {
                    DateTime::from_timestamp(timestamp, 0).unwrap_or_else(Utc::now)
                } else {
                    log::warn!("Failed to parse actors checkpoint timestamp, using epoch");
                    DateTime::from_timestamp(0, 0).unwrap()
                }
            } else {
                log::warn!("Failed to read actors checkpoint file, using epoch");
                DateTime::from_timestamp(0, 0).unwrap()
            }
        } else {
            log::info!("No actors checkpoint file found, starting from epoch");
            DateTime::from_timestamp(0, 0).unwrap()
        };

        log::info!(
            "Indexing objects updated since: {}, actors updated since: {}",
            objects_last_checkpoint,
            actors_last_checkpoint
        );

        // Remove objects that have transitioned to Tombstone status since last checkpoint
        // Skip this if index is empty (nothing to remove)
        if !is_empty {
            let tombstones_removed = self
                .remove_tombstoned_objects(conn, objects_last_checkpoint, effective_batch_size)
                .await?;

            if tombstones_removed > 0 {
                log::info!("Removed {} tombstoned objects from search index", tombstones_removed);
            }
        }

        // Index objects with checkpoint updates after each batch
        let objects_indexed = self
            .index_objects_updated_since(conn, objects_last_checkpoint, effective_batch_size, |updated_at| {
                // Update checkpoint after each successful batch
                if let Err(e) = std::fs::write(&objects_checkpoint_path, updated_at.timestamp().to_string()) {
                    log::warn!("Failed to update objects checkpoint: {}", e);
                } else {
                    log::debug!("Updated objects checkpoint to {}", updated_at);
                }
            })
            .await?;

        // Index actors with checkpoint updates after each batch
        let actors_indexed = self
            .index_actors_updated_since(conn, actors_last_checkpoint, effective_batch_size, |updated_at| {
                // Update checkpoint after each successful batch
                if let Err(e) = std::fs::write(&actors_checkpoint_path, updated_at.timestamp().to_string()) {
                    log::warn!("Failed to update actors checkpoint: {}", e);
                } else {
                    log::debug!("Updated actors checkpoint to {}", updated_at);
                }
            })
            .await?;

        log::info!(
            "Incremental update complete: {} objects, {} actors",
            objects_indexed,
            actors_indexed
        );

        // Merge segments to prevent accumulation and reduce memory usage
        self.merge_segments()?;

        Ok((objects_indexed, actors_indexed))
    }

    /// Wait for pending segment merges to complete and free memory
    /// Should be called periodically after bulk indexing operations
    pub fn merge_segments(&self) -> Result<()> {
        use tantivy::TantivyDocument;

        log::debug!("Waiting for pending segment merges to consolidate and free memory");

        // Wait for objects index merges to complete
        retry_on_lock_contention(|| {
            let writer: tantivy::IndexWriter<TantivyDocument> = self.objects_index.writer(15_000_000)?;  // 3 MB instead of 15 MB
            log::debug!("Waiting for objects index merge threads...");
            writer.wait_merging_threads()?;
            log::debug!("Objects index merges complete");
            Ok(())
        })?;

        // Wait for actors index merges to complete
        retry_on_lock_contention(|| {
            let writer: tantivy::IndexWriter<TantivyDocument> = self.actors_index.writer(15_000_000)?;  // 3 MB instead of 15 MB
            log::debug!("Waiting for actors index merge threads...");
            writer.wait_merging_threads()?;
            log::debug!("Actors index merges complete");
            Ok(())
        })?;

        Ok(())
    }
}

/// Statistics about the search indexes
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub objects_count: usize,
    pub actors_count: usize,
    pub index_path: PathBuf,
}
