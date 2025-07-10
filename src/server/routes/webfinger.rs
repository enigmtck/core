use crate::{models::actors::get_actor_by_username, server::AppState, webfinger::WebFinger};
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use super::{AbstractResponse, ActivityJson, JrdJson, XrdXml};

#[derive(Deserialize)]
pub struct WebfingerQuery {
    resource: String,
}

pub async fn axum_webfinger(
    State(state): State<AppState>,
    Query(query): Query<WebfingerQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let resource = query.resource;

    if !resource.starts_with("acct:") {
        return Err(StatusCode::BAD_REQUEST);
    }

    let parts: Vec<&str> = resource.split(':').collect();
    if parts.len() < 2 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let handle: Vec<&str> = parts[1].split('@').collect();
    let username = handle[0];

    let profile = get_actor_by_username(&conn, username.to_string())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let accept = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if accept.contains("application/xrd+xml") {
        let server_url = format!("https://{}", *crate::SERVER_NAME);
        let xrd_response = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Subject>{resource}</Subject><Alias>{server_url}/user/{username}</Alias><Link href="{server_url}/@{username}" rel="http://webfinger.net/rel/profile-page" type="text/html" /><Link href="{server_url}/user/{username}" rel="self" type="application/activity+json" /><Link href="{server_url}/user/{username}" rel="self" type="application/ld+json; profile=&quot;https://www.w3.org/ns/activitystreams&quot;" /></XRD>"#,
        );
        return Ok(AbstractResponse::XrdXml(XrdXml(xrd_response)));
    }

    let webfinger_data = WebFinger::from(profile);

    if accept.contains("application/jrd+json") {
        return Ok(AbstractResponse::JrdJson(JrdJson(webfinger_data)));
    }

    if accept.contains("application/activity+json") {
        return Ok(AbstractResponse::ActivityJson(ActivityJson(webfinger_data)));
    }

    // Default to plain JSON
    Ok(AbstractResponse::Json(Json(webfinger_data)))
}
