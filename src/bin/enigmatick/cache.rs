use anyhow::Result;
use chrono::Utc;
use clap::{Parser, Subcommand};
use enigmatick::db::runner::DbRunner;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

#[derive(Parser)]
pub struct CacheArgs {
    #[command(subcommand)]
    pub command: CacheCommands,
}

#[derive(Subcommand)]
pub enum CacheCommands {
    /// Prune cached files older than specified duration (e.g., 30d, 2m, 1y)
    Prune { duration: String },
    /// Delete cached item by URL
    Delete { url: String },
    /// Delete cached items from server/domain pattern
    DeleteServer {
        /// Server pattern to match (e.g., "domain.name")
        pattern: String,
    },
}

fn parse_duration(duration_str: &str) -> Result<chrono::Duration> {
    let duration_str = duration_str.trim();
    let numeric_part = duration_str
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>();
    let unit_part = duration_str
        .chars()
        .skip_while(|c| c.is_ascii_digit())
        .collect::<String>();

    let value = numeric_part.parse::<i64>()?;

    match unit_part.as_str() {
        "d" => Ok(chrono::Duration::days(value)),
        "m" => Ok(chrono::Duration::days(value * 30)), // Approximate months
        "y" => Ok(chrono::Duration::days(value * 365)), // Approximate years
        _ => Err(anyhow::anyhow!(
            "Invalid duration unit: '{}'. Use 'd' for days, 'm' for months, 'y' for years.",
            unit_part
        )),
    }
}

pub async fn handle_cache_command(args: CacheArgs) -> Result<()> {
    let conn = enigmatick::db::POOL.get().await.map_err(|e| {
        eprintln!("Failed to get DB connection: {e}");
        e
    })?;

    match args.command {
        CacheCommands::Prune { duration } => {
            println!("Pruning cache items older than {duration}...");
            let duration = parse_duration(&duration)?;
            let cutoff = Utc::now() - duration;

            // Use the existing prune_cache_items function which handles both file and DB deletion
            match enigmatick::models::cache::prune_cache_items(&conn, cutoff).await {
                Ok(count) => println!("Successfully pruned {count} cache items."),
                Err(e) => eprintln!("Error pruning cache: {e}"),
            }
        }
        CacheCommands::Delete { url } => {
            println!("Attempting to delete cache item with URL: {url}...");

            // First fetch the item to get its path
            let item_to_delete: Option<enigmatick::models::cache::CacheItem> = match conn
                .run({
                    let url = url.clone();
                    move |c: &mut diesel::PgConnection| {
                        use diesel::prelude::*;
                        use enigmatick::schema::cache;
                        cache::table
                            .filter(cache::url.eq(url))
                            .first::<enigmatick::models::cache::CacheItem>(c)
                            .optional()
                    }
                })
                .await
            {
                Ok(item) => item,
                Err(e) => {
                    eprintln!("Error fetching cache item for URL {url}: {e}");
                    return Ok(());
                }
            };

            if let Some(item) = item_to_delete {
                // Delete the file if it exists
                if let Some(ref path) = item.path {
                    let full_path = std::path::Path::new(enigmatick::MEDIA_DIR.as_str()).join(path);
                    if let Err(e) = tokio::fs::remove_file(&full_path).await {
                        eprintln!("Warning: failed to delete file {}: {e} (database record will still be deleted)", full_path.display());
                    } else {
                        println!("Deleted file: {}", full_path.display());
                    }
                }

                // Delete the database record
                match enigmatick::models::cache::delete_cache_item_by_url(&conn, url.clone()).await
                {
                    Ok(_) => println!("Successfully deleted cache item for URL: {url}."),
                    Err(e) => {
                        eprintln!("Error deleting cache item from database for URL {url}: {e}")
                    }
                }
            } else {
                println!("No cache item found for URL: {url}");
            }
        }
        CacheCommands::DeleteServer { pattern } => {
            println!("Attempting to delete cache items from server pattern: {pattern}...");

            // 1. Get items to delete
            let server_pattern_for_query = format!("%{pattern}%");
            let items_to_delete_result: Result<Vec<enigmatick::models::cache::CacheItem>, _> = conn
                .run({
                    let server_pattern = server_pattern_for_query.clone();
                    move |c: &mut diesel::PgConnection| {
                        use diesel::prelude::*;
                        use enigmatick::schema::cache;
                        cache::table
                            .filter(cache::url.like(server_pattern))
                            .load::<enigmatick::models::cache::CacheItem>(c)
                    }
                })
                .await;

            let items_to_delete = match items_to_delete_result {
                Ok(items) => items,
                Err(e) => {
                    eprintln!("Error fetching cache items for server pattern '{pattern}': {e}");
                    return Ok(());
                }
            };

            if items_to_delete.is_empty() {
                println!("No cache items found matching server pattern: {pattern}");
                return Ok(());
            }

            // 2. Delete the files
            let mut files_deleted = 0;
            let mut files_failed = 0;

            if atty::is(atty::Stream::Stdout) {
                let mp = MultiProgress::new();
                let pb = mp.add(ProgressBar::new(items_to_delete.len() as u64));
                pb.set_style(ProgressStyle::default_bar()
                        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({percent}%)")
                        .expect("Failed to create progress bar template")
                        .progress_chars("=> "));
                let msg_pb = mp.add(ProgressBar::new(1));
                msg_pb.set_style(
                    ProgressStyle::default_bar()
                        .template("{wide_msg}")
                        .expect("Failed to create message progress bar template"),
                );
                msg_pb.enable_steady_tick(Duration::from_millis(100));

                for item in &items_to_delete {
                    if let Some(ref path) = item.path {
                        let full_path =
                            std::path::Path::new(enigmatick::MEDIA_DIR.as_str()).join(path);
                        msg_pb.set_message(format!("Deleting file: {}", full_path.display()));
                        match tokio::fs::remove_file(&full_path).await {
                            Ok(()) => files_deleted += 1,
                            Err(e) => {
                                eprintln!("Failed to delete file {}: {e}", full_path.display());
                                files_failed += 1;
                            }
                        }
                    } else {
                        msg_pb.set_message(format!("Processing item ID: {}", item.id));
                    }
                    pb.inc(1);
                }
                msg_pb.finish_and_clear();
                pb.finish_with_message("File deletion scan complete.");
            } else {
                for item in &items_to_delete {
                    if let Some(ref path) = item.path {
                        let full_path =
                            std::path::Path::new(enigmatick::MEDIA_DIR.as_str()).join(path);
                        match tokio::fs::remove_file(&full_path).await {
                            Ok(()) => files_deleted += 1,
                            Err(e) => {
                                eprintln!("Failed to delete file {}: {e}", full_path.display());
                                files_failed += 1;
                            }
                        }
                    }
                }
            }
            println!(
                "File deletion summary: {files_deleted} files deleted, {files_failed} files failed"
            );

            // 3. Delete the database records
            match enigmatick::models::cache::delete_cache_items_by_server_pattern(
                &conn,
                pattern.clone(),
            )
            .await
            {
                Ok(deleted_items) => {
                    println!(
                        "Successfully deleted {} cache items from database for server pattern: {}",
                        deleted_items.len(),
                        pattern
                    );
                }
                Err(e) => {
                    eprintln!("Error deleting cache items from database for server pattern '{pattern}': {e}")
                }
            }
        }
    }
    Ok(())
}
