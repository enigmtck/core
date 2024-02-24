use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;

use crate::activity_pub::{ApActivity, ApObject, Inbox};
use crate::db::Db;
use crate::fairings::faktory::FaktoryConnection;
use crate::fairings::signatures::Signed;
use crate::inbox;
use crate::models::profiles::get_profile_by_username;
//use crate::models::remote_activities::create_remote_activity;
use crate::signing::VerificationType;

#[get("/user/<username>/inbox?<offset>&<limit>")]
pub async fn inbox_get(
    signed: Signed,
    conn: Db,
    username: String,
    offset: u16,
    limit: u8,
) -> Result<Json<ApObject>, Status> {
    if let (Some(profile), Signed(true, VerificationType::Local)) = (
        get_profile_by_username((&conn).into(), username).await,
        signed,
    ) {
        let inbox = inbox::retrieve::inbox(&conn, limit.into(), offset.into(), profile).await;
        Ok(Json(inbox))
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/timeline?<offset>&<limit>")]
pub async fn timeline(conn: Db, offset: u16, limit: u8) -> Result<Json<ApObject>, Status> {
    Ok(Json(
        inbox::retrieve::timeline(&conn, limit.into(), offset.into()).await,
    ))
}

#[post("/user/<_>/inbox", data = "<activity>")]
pub async fn inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    activity: String,
) -> Result<Status, Status> {
    shared_inbox_post(signed, conn, faktory, activity).await
}

#[post("/inbox", data = "<activity>")]
pub async fn shared_inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    activity: String,
) -> Result<Status, Status> {
    if let Ok(raw) = serde_json::from_str::<Value>(&activity) {
        if let Ok(activity) = serde_json::from_str::<ApActivity>(&activity) {
            log::debug!("POSTING TO INBOX\n{activity:#?}");

            if let Signed(true, _) = signed {
                activity.inbox(conn, faktory, raw).await
            } else {
                log::debug!("REQUEST WAS UNSIGNED OR MALFORMED");
                Err(Status::NoContent)
            }
        } else {
            log::debug!("UNKNOWN OR MALFORMED ACTIVITY\n{raw:#?}");
            Err(Status::BadRequest)
        }
    } else {
        log::debug!("FAILED TO DECODE JSON\n{activity:#?}");
        Err(Status::BadRequest)
    }
}

#[get("/api/user/<username>/conversation?<conversation>&<offset>&<limit>")]
pub async fn conversation_get(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    username: String,
    offset: u16,
    limit: u8,
    conversation: String,
) -> Result<Json<ApObject>, Status> {
    if let (Some(_profile), Signed(true, VerificationType::Local)) = (
        get_profile_by_username((&conn).into(), username).await,
        signed,
    ) {
        if let Ok(conversation) = urlencoding::decode(&conversation.clone()) {
            let inbox = inbox::retrieve::conversation(
                &conn,
                faktory,
                conversation.to_string(),
                limit.into(),
                offset.into(),
            )
            .await;
            Ok(Json(inbox))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/conversation/<uuid>")]
pub async fn conversation_get_local(
    conn: Db,
    faktory: FaktoryConnection,
    uuid: String,
) -> Result<Json<ApObject>, Status> {
    let conversation = format!("{}/conversation/{}", *crate::SERVER_URL, uuid);

    Ok(Json(
        inbox::retrieve::conversation(&conn, faktory, conversation.to_string(), 40, 0).await,
    ))
}
