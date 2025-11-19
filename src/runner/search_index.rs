use anyhow::Result;
use deadpool_diesel::postgres::Pool;

use crate::events::EventChannels;
use crate::runner::TaskError;

/// Periodic search indexing task
/// Indexes new/updated objects and actors since the last run
pub async fn periodic_search_index_task(
    pool: Pool,
    _channels: Option<EventChannels>,
    _params: Vec<String>,
) -> Result<(), TaskError> {
    log::info!("Starting periodic search indexing task");

    // Get database connection
    let conn = pool.get().await.map_err(|e| {
        log::error!("Failed to get DB connection: {e}");
        TaskError::TaskFailed
    })?;

    // Use the global search index (shared across server and tasks)
    // This prevents creating new mmap'd index files every hour
    let search_index = &*crate::SEARCH_INDEX;

    // Perform incremental update using shared logic (handles checkpoint internally)
    let (objects_indexed, actors_indexed) = search_index
        .incremental_update(&conn, 1000)
        .await
        .map_err(|e| {
            log::error!("Failed to perform incremental update: {e:#?}");
            TaskError::TaskFailed
        })?;

    log::info!(
        "Periodic search indexing complete: {} objects, {} actors",
        objects_indexed,
        actors_indexed
    );

    Ok(())
}
