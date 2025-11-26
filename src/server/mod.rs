use crate::{blocklist::BlockList, events::EventChannels, search::SearchIndex};
use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use deadpool_diesel::postgres::{Manager, Pool};
use dotenvy::dotenv;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;

pub mod extractors;
mod retriever;
mod routes;

pub use routes::inbox::sanitize_json_fields;
pub use routes::inbox::InboxView;

// This struct will hold all shared state for the Axum part of the application.
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool,
    pub block_list: BlockList,
    pub event_channels: EventChannels,
    pub search_index: Arc<SearchIndex>,
}

// The entry point for our Axum server task.
pub async fn start() {
    dotenv().ok();
    // --- Database Pool Setup for Axum ---
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let manager = Manager::new(database_url, deadpool_diesel::Runtime::Tokio1);
    let pool = Pool::builder(manager)
        // Limit pool size to reduce memory from libpq buffers (uses glibc malloc)
        .max_size(8)
        .build()
        .expect("Failed to create Axum database pool.");

    // --- State Initialization for Axum ---
    let block_list = BlockList::new_axum(&pool)
        .await
        .expect("Failed to initialize BlockList for Axum");
    let event_channels = EventChannels::new();

    // Use the global search index (shared across server and tasks)
    let search_index = crate::SEARCH_INDEX.clone();
    log::info!("Using global search index");

    // Create the application state.
    let app_state = AppState {
        db_pool: pool,
        block_list,
        event_channels,
        search_index,
    };

    // Build the Axum router. We will add migrated routes here.
    // For now, a simple test route proves it's working.

    let mut app = Router::new()
        .route(
            "/api/user/{username}/media",
            post(routes::image::upload_media),
        )
        .route("/api/cache", get(routes::image::cached_image))
        // Media file servers
        .nest_service(
            "/media/avatars",
            ServeDir::new(format!("{}/avatars", *crate::MEDIA_DIR)),
        )
        .nest_service(
            "/media/banners",
            ServeDir::new(format!("{}/banners", *crate::MEDIA_DIR)),
        )
        .nest_service(
            "/media/uploads",
            ServeDir::new(format!("{}/uploads", *crate::MEDIA_DIR)),
        );

    // Add custom static directory if configured
    if let Some(custom_dir) = crate::CUSTOM_STATIC_DIR.as_ref() {
        log::info!("Serving custom static files from {}", custom_dir.display());
        app = app.nest_service("/custom", ServeDir::new(custom_dir));
    }

    let app = app
        .layer(DefaultBodyLimit::max(1024 * 1024 * 100))
        .route(
            "/inbox",
            get(routes::inbox::axum_shared_inbox_get).post(routes::inbox::axum_shared_inbox_post),
        )
        .route(
            "/user/{username}/inbox",
            get(routes::inbox::axum_shared_inbox_get).post(routes::inbox::axum_shared_inbox_post),
        )
        // Authentication routes
        .route(
            "/api/user/authenticate",
            post(routes::authentication::authenticate_user),
        )
        .route(
            "/api/user/{username}/password",
            post(routes::authentication::change_password),
        )
        // Instance routes
        .route("/.well-known/host-meta", get(routes::instance::host_meta))
        .route(
            "/.well-known/webfinger",
            get(routes::webfinger::axum_webfinger),
        )
        .route("/install", get(routes::instance::install_script))
        .route(
            "/api/{version}/instance",
            get(routes::instance::instance_information),
        )
        // Encryption routes
        .route(
            "/api/instruments",
            post(routes::encryption::update_instruments),
        )
        .route("/user/{username}/keys", get(routes::encryption::keys))
        // User routes
        .route(
            "/user/{username}",
            get(routes::user::person_get).post(routes::user::person_post),
        )
        .route("/user/{username}/liked", get(routes::user::liked_get))
        .route(
            "/user/{username}/followers",
            get(routes::user::get_followers),
        )
        .route("/user/{username}/following", get(routes::user::get_leaders))
        .route(
            "/user/{username}/outbox",
            get(routes::outbox::axum_outbox_get).post(routes::outbox::axum_outbox_post),
        )
        .route("/api/user/{username}", get(routes::user::user_get_api))
        .route(
            "/api/user/{username}/update/summary",
            post(routes::user::update_summary),
        )
        .route(
            "/api/user/{username}/avatar",
            post(routes::user::upload_avatar),
        )
        .route(
            "/api/user/{username}/banner",
            post(routes::user::upload_banner),
        )
        .route("/api/announcers", get(routes::inbox::axum_announcers_get))
        .route(
            "/api/conversation",
            get(routes::inbox::axum_conversation_get),
        )
        .route("/objects/{uuid}", get(routes::objects::object_get))
        // Remote routes
        .route(
            "/api/remote/webfinger",
            get(routes::remote::remote_webfinger_by_id),
        )
        .route(
            "/api/user/{username}/remote/webfinger",
            get(routes::remote::remote_webfinger_by_id),
        )
        .route("/api/remote/actor", get(routes::remote::remote_actor))
        .route(
            "/api/user/{username}/remote/actor",
            get(routes::remote::remote_actor),
        )
        .route(
            "/api/remote/followers",
            get(routes::remote::remote_followers),
        )
        .route(
            "/api/user/{username}/remote/followers",
            get(routes::remote::remote_followers),
        )
        .route(
            "/api/remote/following",
            get(routes::remote::remote_following),
        )
        .route(
            "/api/user/{username}/remote/following",
            get(routes::remote::remote_following),
        )
        .route("/api/remote/outbox", get(routes::remote::remote_outbox))
        .route(
            "/api/user/{username}/remote/outbox",
            get(routes::remote::remote_outbox),
        )
        .route(
            "/api/user/{username}/remote/keys",
            get(routes::remote::remote_keys),
        )
        .route("/api/remote/object", get(routes::remote::remote_object))
        // Search routes
        .route("/api/search", get(routes::search::search))
        // Admin routes
        .route("/api/user/create", post(routes::admin::create_user))
        .route("/api/system/relay", post(routes::admin::relay_post))
        .route(
            "/api/user/{username}/muted-terms",
            get(routes::admin::get_muted_terms).post(routes::admin::manage_muted_terms),
        )
        .route("/api/admin/memory", get(routes::admin::memory_stats))
        // Client routes
        .route("/login", get(routes::client::client_login))
        .route("/signup", get(routes::client::client_signup))
        .route("/timeline", get(routes::client::client_timeline))
        .route("/notes", get(routes::client::client_notes))
        .route("/_app/{*path}", get(routes::client::client_app_file))
        .route("/assets/{*path}", get(routes::client::client_assets_file))
        .route(
            "/fontawesome/{*path}",
            get(routes::client::client_fontawesome_file),
        )
        .route("/fonts/{*path}", get(routes::client::client_fonts_file))
        .route(
            "/highlight/{*path}",
            get(routes::client::client_highlight_file),
        )
        .route("/icons/{*path}", get(routes::client::client_icons_file))
        .route("/{handle}", get(routes::client::client_profile))
        .route("/", get(routes::client::client_index))
        .with_state(app_state);

    let server_addr_str = crate::SERVER_ADDRESS.as_str();
    let server: SocketAddr = server_addr_str
        .parse()
        .expect("Unable to parse socket address");

    log::info!("Axum server listening on {server}");

    let listener = tokio::net::TcpListener::bind(server).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
