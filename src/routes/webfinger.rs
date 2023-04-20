use crate::{db::Db, models::profiles::get_profile_by_username, webfinger::WebFinger};
use rocket::{get, http::Status, serde::json::Json};

#[get(
    "/.well-known/webfinger?<resource>",
    format = "application/xrd+xml",
    rank = 2
)]
pub async fn webfinger_xml(conn: Db, resource: String) -> Result<String, Status> {
    if resource.starts_with("acct:") {
        let parts = resource.split(':').collect::<Vec<&str>>();
        let handle = parts[1].split('@').collect::<Vec<&str>>();
        let username = handle[0];

        let server_url = (*crate::SERVER_URL).clone();

        if get_profile_by_username(&conn, username.to_string())
            .await
            .is_some()
        {
            Ok(format!(
                r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Subject>{resource}</Subject><Alias>{server_url}/user/{username}</Alias><Link href="{server_url}/@{username}" rel="http://webfinger.net/rel/profile-page" type="text/html" /><Link href="{server_url}/user/{username}" rel="self" type="application/activity+json" /><Link href="{server_url}/user/{username}" rel="self" type="application/ld+json; profile=&quot;https://www.w3.org/ns/activitystreams&quot;" /></XRD>"#
            ))
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
    rank = 1
)]
pub async fn webfinger_json(conn: Db, resource: String) -> Result<Json<WebFinger>, Status> {
    if resource.starts_with("acct:") {
        let parts = resource.split(':').collect::<Vec<&str>>();
        let handle = parts[1].split('@').collect::<Vec<&str>>();
        let username = handle[0];

        match get_profile_by_username(&conn, username.to_string()).await {
            Some(profile) => Ok(Json(WebFinger::from(profile))),
            None => Err(Status::NoContent),
        }
    } else {
        Err(Status::NoContent)
    }
}
