use axum::{
    extract::{Path, Query},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "client/"]
pub struct Client;

fn serve_200_html() -> Response {
    match Client::get("200.html") {
        Some(asset) => Html(asset.data).into_response(),
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

async fn serve_static_file(path: PathBuf) -> Response {
    let path_str = path.to_string_lossy();
    log::debug!("Serving static file from axum: {path_str}");

    match Client::get(&path_str) {
        Some(asset) => {
            let mime = mime_guess::from_path(&*path_str).first_or_octet_stream();
            let mut headers = axum::http::HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, mime.to_string().parse().unwrap());
            (headers, asset.data).into_response()
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

pub async fn client_profile(Path(handle): Path<String>) -> Response {
    if handle.starts_with('@') {
        serve_200_html()
    } else {
        (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}

#[derive(Deserialize)]
pub struct UuidQuery {
    _uuid: String,
}

pub async fn client_notes(Query(_query): Query<UuidQuery>) -> Response {
    serve_200_html()
}

pub async fn client_timeline() -> Response {
    serve_200_html()
}

pub async fn client_signup() -> Response {
    serve_200_html()
}

pub async fn client_login() -> Response {
    serve_200_html()
}

pub async fn client_index() -> Response {
    serve_200_html()
}

pub async fn client_app_file(Path(file): Path<String>) -> Response {
    let path = PathBuf::from("_app").join(&file);
    serve_static_file(path).await
}

pub async fn client_assets_file(Path(file): Path<String>) -> Response {
    let path = PathBuf::from("assets").join(&file);
    serve_static_file(path).await
}

pub async fn client_fontawesome_file(Path(file): Path<String>) -> Response {
    let path = PathBuf::from("fontawesome").join(&file);
    serve_static_file(path).await
}

pub async fn client_fonts_file(Path(file): Path<String>) -> Response {
    let path = PathBuf::from("fonts").join(&file);
    serve_static_file(path).await
}

pub async fn client_highlight_file(Path(file): Path<String>) -> Response {
    let path = PathBuf::from("highlight").join(&file);
    serve_static_file(path).await
}

pub async fn client_icons_file(Path(file): Path<String>) -> Response {
    let path = PathBuf::from("icons").join(&file);
    serve_static_file(path).await
}
