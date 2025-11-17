use anyhow::Result;
use clap::{Parser, Subcommand};
use enigmatick::db::runner::DbRunner;
use enigmatick::search::SearchIndex;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
pub struct SearchArgs {
    #[command(subcommand)]
    pub command: SearchCommands,
}

#[derive(Subcommand)]
pub enum SearchCommands {
    /// Rebuild search index from database (full reindex)
    Index,
    /// Update search index incrementally (only new/updated content)
    Update,
    /// Show search index statistics
    Status,
    /// Optimize search index (merge segments)
    Optimize,
}

pub async fn handle_search_command(args: SearchArgs) -> Result<()> {
    match args.command {
        SearchCommands::Index => {
            println!("Rebuilding search index from database...");

            // Initialize search index
            let index_path = format!("{}/search_index", *enigmatick::MEDIA_DIR);
            let search_index = SearchIndex::new(&index_path)?;

            let conn = enigmatick::db::POOL.get().await.map_err(|e| {
                eprintln!("Failed to get DB connection: {e}");
                e
            })?;

            // Index objects (posts/notes/articles/questions)
            println!("Indexing objects...");
            use std::io::{self, Write};
            io::stdout().flush().unwrap();

            // Count total objects for progress bar
            let total_objects: i64 = conn
                .run(|c| {
                    use diesel::prelude::*;
                    use enigmatick::schema::objects;

                    objects::table
                        .filter(objects::as_deleted.is_null())
                        .count()
                        .get_result(c)
                })
                .await?;

            println!("Found {total_objects} objects to index");

            // Use shared indexing logic with progress tracking
            let total_indexed = if atty::is(atty::Stream::Stdout) {
                let pb = ProgressBar::new(total_objects as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%)")
                        .expect("Failed to create progress bar template")
                        .progress_chars("=> "),
                );

                search_index.index_all_objects(&conn, 100000, |count| {
                    pb.set_position(count as u64);
                }).await?;

                pb.finish_with_message("Objects indexed");
                pb.position() as usize
            } else {
                search_index.index_all_objects(&conn, 100000, |count| {
                    if count % 100000 == 0 {
                        println!("Indexed {} objects...", count);
                    }
                }).await?
            };

            println!("Total objects indexed: {}", total_indexed);

            // Index actors (users/profiles)
            println!("\nIndexing actors...");
            io::stdout().flush().unwrap();

            // Count total actors for progress bar
            let total_actors: i64 = conn
                .run(|c| {
                    use diesel::prelude::*;
                    use enigmatick::schema::actors;

                    actors::table
                        .filter(actors::as_discoverable.eq(true))
                        .count()
                        .get_result(c)
                })
                .await?;

            println!("Found {total_actors} discoverable actors to index");

            // Use shared indexing logic with progress tracking
            let total_indexed_actors = if atty::is(atty::Stream::Stdout) {
                let pb = ProgressBar::new(total_actors as u64);
                pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%)")
                        .expect("Failed to create progress bar template")
                        .progress_chars("=> "),
                );

                search_index.index_all_actors(&conn, 100000, |count| {
                    pb.set_position(count as u64);
                }).await?;

                pb.finish_with_message("Actors indexed");
                pb.position() as usize
            } else {
                search_index.index_all_actors(&conn, 100000, |count| {
                    if count % 100000 == 0 {
                        println!("Indexed {} actors...", count);
                    }
                }).await?
            };

            println!("Total actors indexed: {}", total_indexed_actors);

            println!("\nSearch index rebuild complete!");
            println!("  Objects processed: {}", total_indexed);
            println!("  Actors processed: {}", total_indexed_actors);

            // Verify by reading stats
            println!("\nVerifying index...");
            let stats = search_index.get_stats()?;
            println!("  Objects in index: {}", stats.objects_count);
            println!("  Actors in index: {}", stats.actors_count);

            if stats.objects_count != total_indexed as usize || stats.actors_count != total_indexed_actors as usize {
                eprintln!("\nWarning: Index count mismatch detected!");
                eprintln!("  Expected objects: {}, Got: {}", total_indexed, stats.objects_count);
                eprintln!("  Expected actors: {}, Got: {}", total_indexed_actors, stats.actors_count);
            } else {
                println!("✓ Index verification successful!");
            }
        }

        SearchCommands::Update => {
            println!("Updating search index incrementally...");

            // Initialize search index
            let index_path = format!("{}/search_index", *enigmatick::MEDIA_DIR);
            let search_index = SearchIndex::new(&index_path)?;

            let conn = enigmatick::db::POOL.get().await.map_err(|e| {
                eprintln!("Failed to get DB connection: {e}");
                e
            })?;

            // Perform incremental update using shared logic (handles checkpoint internally)
            let (objects_indexed, actors_indexed) = search_index
                .incremental_update(&conn, 1000)
                .await?;

            println!("\nSearch index update complete!");
            println!("  Objects indexed: {}", objects_indexed);
            println!("  Actors indexed: {}", actors_indexed);
        }

        SearchCommands::Status => {
            println!("Checking search index status...");

            let index_path = format!("{}/search_index", *enigmatick::MEDIA_DIR);
            let search_index = SearchIndex::new(&index_path)?;

            let stats = search_index.get_stats()?;

            println!("\nSearch Index Statistics");
            println!("=======================");
            println!("Index path: {}", stats.index_path.display());
            println!("Objects indexed: {}", stats.objects_count);
            println!("Actors indexed: {}", stats.actors_count);
            println!("Total documents: {}", stats.objects_count + stats.actors_count);
        }

        SearchCommands::Optimize => {
            println!("Optimizing search index...");

            let index_path = format!("{}/search_index", *enigmatick::MEDIA_DIR);
            let search_index = SearchIndex::new(&index_path)?;

            search_index.optimize()?;

            println!("Search index optimized successfully!");
        }
    }

    Ok(())
}
