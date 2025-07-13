use crate::{blocklist::BlockList, events::EventChannels, server::extractors::AxumSigned}; // Use the new AxumSigned type
use axum::{
    routing::{get, post},
    Router,
};
use deadpool_diesel::postgres::{Manager, Pool};
use dotenvy::dotenv;
use std::net::SocketAddr;
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
}

// The entry point for our Axum server task.
pub async fn start() {
    dotenv().ok();
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
        // Inbox routes
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
        // Image routes
        .route(
            "/api/user/{username}/media",
            post(routes::image::upload_media),
        )
        .route("/api/cache", get(routes::image::cached_image))
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
        // Admin routes
        .route("/api/user/create", post(routes::admin::create_user))
        .route("/api/system/relay", post(routes::admin::relay_post))
        .route(
            "/api/user/{username}/muted-terms",
            get(routes::admin::get_muted_terms).post(routes::admin::manage_muted_terms),
        )
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
        )
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

// Update the test handler to use the new extractor.
async fn hello_axum(signed: AxumSigned) -> &'static str {
    // The log will still work due to Deref and Debug on the inner type
    log::info!("Request received with signature status: {signed:?}");
    "Hello from the Axum side!"
}
