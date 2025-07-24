use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::future::Future;
use std::pin::Pin;
use tokio::sync::RwLock;
use tokio::time::interval;

/// Trait that all tasks must implement
pub trait Task: Send + Sync {
    /// The name of the task (for logging)
    fn name(&self) -> &'static str;
    
    /// How often this task should run
    fn interval(&self) -> Duration;
    
    /// Execute the task - returns a boxed future to make it dyn-compatible
    fn execute(&self) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + '_>>;
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
        log::info!("Registering task: {}", name);
        self.tasks.write().await.insert(name, task);
    }

    /// Start the task scheduler
    pub async fn start(&self) {
        *self.running.write().await = true;
        log::info!("Starting task scheduler...");

        let tasks = self.tasks.read().await;
        let mut handles = Vec::new();

        for (name, task) in tasks.iter() {
            let task_name = name.clone();
            let task_interval = task.interval();
            let running = self.running.clone();
            
            // We need to move the task into the spawned task
            // Since we can't clone Box<dyn Task>, we'll use a different approach
            let task_name_for_spawn = task_name.clone();
            let tasks_clone = self.tasks.clone();
            
            let handle = tokio::spawn(async move {
                let mut interval_timer = interval(task_interval);
                
                while *running.read().await {
                    interval_timer.tick().await;
                    
                    if !*running.read().await {
                        break;
                    }

                    let start_time = Instant::now();
                    log::debug!("Executing task: {}", task_name_for_spawn);
                    
                    // Get the task from the shared map
                    let tasks_guard = tasks_clone.read().await;
                    if let Some(task) = tasks_guard.get(&task_name_for_spawn) {
                        match task.execute().await {
                            Ok(()) => {
                                let duration = start_time.elapsed();
                                log::debug!("Task {} completed in {:?}", task_name_for_spawn, duration);
                            }
                            Err(e) => {
                                log::error!("Task {} failed: {}", task_name_for_spawn, e);
                            }
                        }
                    }
                }
                
                log::info!("Task {} stopped", task_name_for_spawn);
            });
            
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Stop the task scheduler
    pub async fn stop(&self) {
        log::info!("Stopping task scheduler...");
        *self.running.write().await = false;
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

    fn execute(&self) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + '_>> {
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

    fn execute(&self) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + '_>> {
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

    fn execute(&self) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + '_>> {
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

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();
    
    log::info!("Starting Enigmatick Task Manager");

    let scheduler = TaskScheduler::new();

    // Register tasks
    scheduler.register_task(Box::new(DatabaseCleanupTask)).await;
    scheduler.register_task(Box::new(FederationHealthCheckTask)).await;
    scheduler.register_task(Box::new(ActivityDeliveryRetryTask)).await;

    // Set up graceful shutdown
    let scheduler_clone = scheduler.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl+c");
        log::info!("Shutdown signal received");
        scheduler_clone.stop().await;
    });

    // Start the scheduler (this will block until stopped)
    scheduler.start().await;
    
    log::info!("Task Manager shutdown complete");
}
