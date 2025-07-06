use crate::{db::Db, models::actors::get_actor_by_username, webfinger::WebFinger};
use rocket::{get, http::Status, serde::json::Json};

use super::{ActivityJson, JrdJson, XrdXml};

#[get(
    "/.well-known/webfinger?<resource>",
    format = "application/xrd+xml",
    rank = 4
)]
pub async fn webfinger_xml(conn: Db, resource: String) -> Result<XrdXml, Status> {
    if resource.starts_with("acct:") {
        let parts = resource.split(':').collect::<Vec<&str>>();
        let handle = parts[1].split('@').collect::<Vec<&str>>();
        let username = handle[0];

        let server_url = format!("https://{}", *crate::SERVER_NAME);

        if get_actor_by_username(&conn, username.to_string())
            .await
            .is_ok()
        {
            Ok(XrdXml(format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Subject>{resource}</Subject><Alias>{server_url}/user/{username}</Alias><Link href="{server_url}/@{username}" rel="http://webfinger.net/rel/profile-page" type="text/html" /><Link href="{server_url}/user/{username}" rel="self" type="application/activity+json" /><Link href="{server_url}/user/{username}" rel="self" type="application/ld+json; profile=&quot;https://www.w3.org/ns/activitystreams&quot;" /></XRD>"#
            )))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get(
    "/.well-known/webfinger?<resource>",
    format = "application/jrd+json",
    rank = 3
)]
pub async fn webfinger_jrd_json(conn: Db, resource: String) -> Result<JrdJson<WebFinger>, Status> {
    webfinger(conn, resource).await.map(|x| JrdJson(Json(x)))
}

#[get(
    "/.well-known/webfinger?<resource>",
    format = "application/activity+json",
    rank = 2
)]
pub async fn webfinger_activity_json(
    conn: Db,
    resource: String,
) -> Result<ActivityJson<WebFinger>, Status> {
    webfinger(conn, resource)
        .await
        .map(|x| ActivityJson(Json(x)))
}

#[get("/.well-known/webfinger?<resource>", format = "json", rank = 1)]
pub async fn webfinger_json(conn: Db, resource: String) -> Result<Json<WebFinger>, Status> {
    webfinger(conn, resource).await.map(Json)
}

async fn webfinger(conn: Db, resource: String) -> Result<WebFinger, Status> {
    if resource.starts_with("acct:") {
        let parts = resource.split(':').collect::<Vec<&str>>();
        let handle = parts[1].split('@').collect::<Vec<&str>>();
        let username = handle[0];

        if let Ok(profile) = get_actor_by_username(&conn, username.to_string()).await {
            Ok(WebFinger::from(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}
