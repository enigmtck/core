use std::path::PathBuf;

use crate::rocket::futures::StreamExt;
use anyhow::{anyhow, Result};
use axum::Router;
use axum_reverse_proxy::ReverseProxy;
use rustls_acme::{caches::DirCache, AcmeConfig};

pub async fn start() -> Result<()> {
    let server_port = crate::ROCKET_PORT.as_str();
    let server_address = crate::ROCKET_ADDRESS.as_str();
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

    let proxy = ReverseProxy::new("/", &format!("http://{server_address}:{server_port}"));
    let app: Router = proxy.into();

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
