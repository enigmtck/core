use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;

use anyhow::Result;
use chrono::Utc;

#[cfg(feature = "vendored-openssl")]
use openssl as _;

#[cfg(feature = "bundled-postgres")]
use pq_sys as _;

type TaskResult<'a> =
    Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>>;

/// Trait that all tasks must implement
pub trait Task: Send + Sync {
    /// The name of the task (for logging)
    fn name(&self) -> &'static str;

    /// How often this task should run
    fn interval(&self) -> Duration;

    /// Execute the task - returns a boxed future to make it dyn-compatible
    fn execute(&self) -> TaskResult;
}

/// Task scheduler that manages and runs tasks
pub struct TaskScheduler {
    tasks: Arc<RwLock<HashMap<String, Box<dyn Task>>>>,
    running: Arc<RwLock<bool>>,
}

impl Clone for TaskScheduler {
    fn clone(&self) -> Self {
        Self {
            tasks: self.tasks.clone(),
            running: self.running.clone(),
        }
    }
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Register a new task
    pub async fn register_task(&self, task: Box<dyn Task>) {
        let name = task.name().to_string();
        log::info!("Registering task: {name}");
        self.tasks.write().await.insert(name, task);
    }

    /// Start the task scheduler
    pub async fn start(&self) {
        *self.running.write().await = true;
        log::info!("Starting task scheduler...");

        let tasks = self.tasks.read().await;
        let mut handles = Vec::new();

        log::info!("Found {} tasks to schedule", tasks.len());

        for (name, task) in tasks.iter() {
            let task_name = name.clone();
            let task_interval = task.interval();
            let running = self.running.clone();

            log::info!("Scheduling task '{task_name}' with interval {task_interval:?}");

            // We need to move the task into the spawned task
            // Since we can't clone Box<dyn Task>, we'll use a different approach
            let task_name_for_spawn = task_name.clone();
            let tasks_clone = self.tasks.clone();

            let handle = tokio::spawn(async move {
                log::info!("Task '{task_name_for_spawn}' started");
                let mut interval_timer = interval(task_interval);
                // Skip missed ticks if task takes longer than interval
                // This prevents back-to-back execution and waits for next scheduled tick
                interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

                while *running.read().await {
                    interval_timer.tick().await;

                    if !*running.read().await {
                        log::info!("Task '{task_name_for_spawn}' received stop signal");
                        break;
                    }

                    let start_time = Instant::now();
                    log::debug!("Executing task: {task_name_for_spawn}");

                    // Get the task from the shared map
                    let tasks_guard = tasks_clone.read().await;
                    if let Some(task) = tasks_guard.get(&task_name_for_spawn) {
                        match task.execute().await {
                            Ok(()) => {
                                let duration = start_time.elapsed();
                                log::debug!("Task {task_name_for_spawn} completed in {duration:?}");
                            }
                            Err(e) => {
                                log::error!("Task {task_name_for_spawn} failed: {e}");
                            }
                        }
                    } else {
                        log::error!("Task '{task_name_for_spawn}' not found in task map");
                        break;
                    }
                }

                log::info!("Task {task_name_for_spawn} stopped");
            });

            handles.push(handle);
        }

        log::info!(
            "All {} tasks spawned, waiting for completion...",
            handles.len()
        );

        // Wait for all tasks to complete
        for (i, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(()) => log::info!("Task handle {i} completed normally"),
                Err(e) => log::error!("Task handle {i} failed: {e}"),
            }
        }

        log::info!("Task scheduler stopped");
    }

    /// Stop the task scheduler
    pub async fn stop(&self) {
        log::info!("Stopping task scheduler...");
        *self.running.write().await = false;
    }
}

impl Default for TaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Example task: Database cleanup
pub struct DatabaseCleanupTask;

impl Task for DatabaseCleanupTask {
    fn name(&self) -> &'static str {
        "database_cleanup"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(3600) // Run every hour
    }

    fn execute(&self) -> TaskResult {
        Box::pin(async move {
            log::info!("Running database cleanup...");
            // TODO: Implement actual database cleanup logic
            // - Remove old unprocessable entries
            // - Clean up orphaned media files
            // - Vacuum database tables
            Ok(())
        })
    }
}

/// Example task: Federation health check
pub struct FederationHealthCheckTask;

impl Task for FederationHealthCheckTask {
    fn name(&self) -> &'static str {
        "federation_health_check"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(300) // Run every 5 minutes
    }

    fn execute(&self) -> TaskResult {
        Box::pin(async move {
            log::info!("Checking federation health...");
            // TODO: Implement federation health checks
            // - Check connectivity to known instances
            // - Update instance status
            // - Retry failed deliveries
            Ok(())
        })
    }
}

/// Example task: Activity delivery retry
pub struct ActivityDeliveryRetryTask;

impl Task for ActivityDeliveryRetryTask {
    fn name(&self) -> &'static str {
        "activity_delivery_retry"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(600) // Run every 10 minutes
    }

    fn execute(&self) -> TaskResult {
        Box::pin(async move {
            log::info!("Retrying failed activity deliveries...");
            // TODO: Implement delivery retry logic
            // - Find failed deliveries in database
            // - Attempt redelivery with exponential backoff
            // - Mark permanently failed after max attempts
            Ok(())
        })
    }
}

/// Cache cleanup task: Remove cache items older than 30 days
pub struct CacheCleanupTask;

impl Task for CacheCleanupTask {
    fn name(&self) -> &'static str {
        "cache_cleanup"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(86400) // Run once a day (24 hours)
    }

    fn execute(&self) -> TaskResult {
        Box::pin(async move {
            log::info!("Running cache cleanup (removing items older than 30 days)...");

            // Get database connection
            let conn = match enigmatick::db::POOL.get().await {
                Ok(conn) => conn,
                Err(e) => {
                    log::error!("Failed to get database connection for cache cleanup: {e}");
                    return Err(e.into());
                }
            };

            // Calculate cutoff date (30 days ago)
            let cutoff = Utc::now() - chrono::Duration::days(30);

            // Use the existing prune_cache_items function which handles both file and DB deletion
            match enigmatick::models::cache::prune_cache_items(&conn, cutoff).await {
                Ok(deleted_count) => {
                    log::info!("Cache cleanup completed successfully - removed {deleted_count} items older than 30 days");
                    Ok(())
                }
                Err(e) => {
                    log::error!("Error during cache cleanup: {e}");
                    Err(e.into())
                }
            }
        })
    }
}

/// Search index update task: Incrementally index new/updated content
pub struct SearchIndexTask;

impl Task for SearchIndexTask {
    fn name(&self) -> &'static str {
        "search_index_update"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(3600) // Run every hour
    }

    fn execute(&self) -> TaskResult {
        Box::pin(async move {
            log::info!("Running search index update...");

            // Get database connection
            let pool = enigmatick::db::POOL.clone();

            // Call the periodic task function
            match enigmatick::runner::search_index::periodic_search_index_task(pool, None, vec![]).await {
                Ok(()) => {
                    log::info!("Search index update completed successfully");
                    Ok(())
                }
                Err(e) => {
                    log::error!("Search index update failed: {e:?}");
                    Err(format!("Search index update failed: {e:?}").into())
                }
            }
        })
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();

    log::info!("Starting Enigmatick Task Manager");

    let scheduler = TaskScheduler::new();

    // Register tasks
    log::info!("Registering tasks...");
    scheduler.register_task(Box::new(DatabaseCleanupTask)).await;
    scheduler
        .register_task(Box::new(FederationHealthCheckTask))
        .await;
    scheduler
        .register_task(Box::new(ActivityDeliveryRetryTask))
        .await;
    scheduler.register_task(Box::new(CacheCleanupTask)).await;
    scheduler.register_task(Box::new(SearchIndexTask)).await;
    log::info!("All tasks registered successfully");

    // Set up graceful shutdown
    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                log::info!("Shutdown signal received");
                scheduler_clone.stop().await;
            }
            Err(err) => {
                log::error!("Unable to listen for shutdown signal: {err}");
            }
        }
    });

    log::info!("Starting task scheduler...");
    // Start the scheduler (this will block until stopped)
    scheduler.start().await;

    log::info!("Task Manager shutdown complete");
}
