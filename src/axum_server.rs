use crate::fairings::{access_control::BlockList, events::EventChannels};
use axum::{routing::get, Router};
use deadpool_diesel::postgres::{Manager, Pool};
use std::net::SocketAddr;
//use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// This struct will hold all shared state for the Axum part of the application.
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool,
    pub block_list: BlockList,
    pub event_channels: EventChannels,
}

// The entry point for our Axum server task.
pub async fn start() {
    // Initialize logging (tracing is the standard in the Axum ecosystem)
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| "enigmatick=debug,tower_http=debug".into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

    // --- Database Pool Setup for Axum ---
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager)
        .build()
        .expect("Failed to create Axum database pool.");

    // --- State Initialization for Axum ---
    let block_list = BlockList::new_axum(&pool)
        .await
        .expect("Failed to initialize BlockList for Axum");
    let event_channels = EventChannels::new();

    // Create the application state.
    let app_state = AppState {
        db_pool: pool,
        block_list,
        event_channels,
    };

    // Build the Axum router. We will add migrated routes here.
    // For now, a simple test route proves it's working.
    let app = Router::new()
        .route("/hello", get(hello_axum))
        .with_state(app_state);

    // Run the Axum server on an internal-only port.
    let addr = SocketAddr::from(([127, 0, 0, 1], 8001));
    tracing::debug!("Axum server listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// A temporary handler to prove the Axum server is reachable.
async fn hello_axum() -> &'static str {
    "Hello from the Axum side!"
}
