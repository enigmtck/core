use anyhow::{anyhow, Result};
use axum::{routing::any, Router};
use axum_reverse_proxy::ReverseProxy;
use dotenvy::dotenv;
use futures::StreamExt;
use lazy_static::lazy_static;
use rustls_acme::{caches::DirCache, AcmeConfig};
use std::env;
use std::path::PathBuf;
use tower::ServiceExt;

#[cfg(feature = "vendored-openssl")]
use openssl as _;

lazy_static! {
    pub static ref ACME_PROXY: bool = {
        dotenv().ok();
        env::var("ACME_PROXY")
            .is_ok_and(|x| x.parse().expect("ACME_PROXY must be \"true\" or \"false\""))
    };
    pub static ref ACME_EMAILS: Option<Vec<String>> = {
        dotenv().ok();
        if let Ok(emails) = env::var("ACME_EMAIL") {
            serde_json::from_str(&emails).ok()
        } else {
            None
        }
    };
    pub static ref ACME_PORT: String = {
        dotenv().ok();
        env::var("ACME_PORT").unwrap_or("443".to_string())
    };
    pub static ref ROCKET_PORT: String = {
        dotenv().ok();
        env::var("ROCKET_PORT").unwrap_or("8000".to_string())
    };
    pub static ref ROCKET_ADDRESS: String = {
        dotenv().ok();
        env::var("ROCKET_ADDRESS").unwrap_or("0.0.0.0".to_string())
    };
    pub static ref SERVER_NAME: String = {
        dotenv().ok();
        env::var("SERVER_NAME").expect("SERVER_NAME must be set")
    };
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let rocket_port = crate::ROCKET_PORT.as_str();
    let rocket_address = crate::ROCKET_ADDRESS.as_str();
    let acme_port = crate::ACME_PORT.as_str();
    let server_name = crate::SERVER_NAME.as_str();
    let acme_emails = (*crate::ACME_EMAILS)
        .clone()
        .ok_or(anyhow!("ACME_EMAILS must be set"))?;

    let mut state = AcmeConfig::new(vec![server_name])
        .contact(acme_emails.iter().map(|e| format!("mailto:{e}")))
        .cache_option(Some(DirCache::new(PathBuf::from("acme/"))))
        .directory_lets_encrypt(true)
        .state();

    let acceptor = state.axum_acceptor(state.default_rustls_config());

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => log::info!("ACME event: {ok:?}"),
                Err(err) => log::error!("ACME error: {err:?}"),
            }
        }
    });

    let axum_proxy = ReverseProxy::new("/", "http://127.0.0.1:8001");
    let rocket_proxy = ReverseProxy::new("/", &format!("http://{rocket_address}:{rocket_port}"));

    let app: Router = Router::new()
        // Any request starting with `/axum` will be forwarded to the Axum server.
        // `nest_service` is the correct way to delegate a block of routes to another service.
        .nest_service("/rocket", rocket_proxy)
        // Fallback: Any request that doesn't match the routes above goes to the Rocket server.
        .fallback(any(|req| async { axum_proxy.oneshot(req).await }));

    //let app: Router = axum_proxy.into().nest_service("/axum", axum_proxy);

    let listener = std::net::TcpListener::bind(format!("[::]:{acme_port}")).unwrap();
    log::info!("Proxy server running on https://[::]:{acme_port} (IPv4 and IPv6) with ACME/TLS");

    // Run the server with the ACME acceptor for automatic TLS
    axum_server::from_tcp(listener)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
