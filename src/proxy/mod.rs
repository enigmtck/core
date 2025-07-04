use std::path::PathBuf;

use crate::rocket::futures::StreamExt;
use anyhow::{anyhow, Result};
use axum::{routing::any, Router};
use axum_reverse_proxy::ReverseProxy;
use rustls_acme::{caches::DirCache, AcmeConfig};
use tower::ServiceExt;

pub async fn start() -> Result<()> {
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

    // --- NEW ROUTING LOGIC ---
    // Proxy for the new Axum server on its internal port
    let axum_proxy = ReverseProxy::new("/", "http://127.0.0.1:8001");

    // Proxy for the existing Rocket server
    let rocket_proxy = ReverseProxy::new("/", &format!("http://{rocket_address}:{rocket_port}"));

    // The main router that decides where to send requests.
    // We will add migrated routes here.
    let app: Router = Router::new()
        // Any request starting with `/axum` will be forwarded to the Axum server.
        // `nest_service` is the correct way to delegate a block of routes to another service.
        .nest_service("/axum", axum_proxy)
        // Fallback: Any request that doesn't match the routes above goes to the Rocket server.
        .fallback(any(|req| async { rocket_proxy.oneshot(req).await }));

    let listener = std::net::TcpListener::bind(format!("[::]:{acme_port}")).unwrap();
    log::info!("Proxy server running on https://[::]:{acme_port} (IPv4 and IPv6) with ACME/TLS");

    axum_server::from_tcp(listener)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await
        .unwrap();

    Ok(())
}
