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

            // Variables for progress reporting
            let show_progress = atty::is(atty::Stream::Stdout);
            let _multi_progress_holder: Option<MultiProgress> = if show_progress {
                Some(MultiProgress::new())
            } else {
                None
            };

            // First, get the items that would be deleted to show progress
            let old_items_result: Result<Vec<enigmatick::models::cache::CacheItem>, _> = conn
                .run(move |c: &mut diesel::PgConnection| {
                    use diesel::prelude::*;
                    use enigmatick::schema::cache;
                    cache::table
                        .filter(cache::created_at.lt(cutoff))
                        .load::<enigmatick::models::cache::CacheItem>(c)
                })
                .await;

            match old_items_result {
                Ok(old_items) => {
                    if old_items.is_empty() {
                        println!("No cache items found older than {duration}.");
                        return Ok(());
                    }

                    // Show progress if we're in a TTY
                    if show_progress {
                        let mp = MultiProgress::new();
                        let pb = mp.add(ProgressBar::new(old_items.len() as u64));
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

                        for item in &old_items {
                            if let Some(ref path) = item.path {
                                msg_pb.set_message(format!("Processing: {path}"));
                            } else {
                                msg_pb.set_message(format!("Processing item ID: {}", item.id));
                            }
                            pb.inc(1);
                            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                        }
                        msg_pb.finish_and_clear();
                        pb.finish_with_message(format!("Processed {} items", old_items.len()));
                    }

                    match enigmatick::models::cache::prune_cache_items(&conn, cutoff).await {
                        Ok(count) => println!("Successfully pruned {count} cache items."),
                        Err(e) => eprintln!("Error pruning cache: {e}"),
                    }
                }
                Err(e) => eprintln!("Error fetching cache items for progress display: {e}"),
            };
        }
        CacheCommands::Delete { url } => {
            println!("Attempting to delete cache item with URL: {url}...");

            match enigmatick::models::cache::delete_cache_item_by_url(&conn, url.clone()).await {
                Ok(_) => println!("Successfully deleted cache item for URL: {url}."),
                Err(e) => eprintln!("Error deleting cache item for URL {url}: {e}"),
            }
        }
        CacheCommands::DeleteServer { pattern } => {
            println!("Attempting to delete cache items from server pattern: {pattern}...");

            // Variables for progress reporting
            let show_progress = atty::is(atty::Stream::Stdout);
            let _multi_progress_holder: Option<MultiProgress> = if show_progress {
                Some(MultiProgress::new())
            } else {
                None
            };

            match enigmatick::models::cache::delete_cache_items_by_server_pattern(
                &conn,
                pattern.clone(),
            )
            .await
            {
                Ok(deleted_items) => {
                    if deleted_items.is_empty() {
                        println!("No cache items found matching server pattern: {pattern}");
                    } else {
                        // Show progress if we're in a TTY
                        if show_progress {
                            let mp = MultiProgress::new();
                            let pb = mp.add(ProgressBar::new(deleted_items.len() as u64));
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

                            for item in &deleted_items {
                                if let Some(ref path) = item.path {
                                    msg_pb.set_message(format!("Processed: {path}"));
                                } else {
                                    msg_pb.set_message(format!("Processed item ID: {}", item.id));
                                }
                                pb.inc(1);
                                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                            }
                            msg_pb.finish_and_clear();
                            pb.finish_with_message(format!(
                                "Deleted {} items from server pattern: {}",
                                deleted_items.len(),
                                pattern
                            ));
                        }
                        println!("Successfully deleted {} cache items matching server pattern: {pattern}", deleted_items.len());
                    }
                }
                Err(e) => eprintln!("Error deleting cache items for server pattern {pattern}: {e}"),
            }
        }
    }
    Ok(())
}
