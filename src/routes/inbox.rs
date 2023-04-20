use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post};
use serde_json::Value;

use crate::activity_pub::{ApActivity, ApObject};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::fairings::faktory::FaktoryConnection;
use crate::fairings::signatures::Signed;
use crate::inbox;
use crate::models::profiles::get_profile_by_username;
use crate::models::remote_activities::create_remote_activity;
use crate::signing::VerificationType;

#[get("/user/<username>/inbox?<offset>&<limit>")]
pub async fn inbox_get(
    signed: Signed,
    conn: Db,
    username: String,
    offset: u16,
    limit: u8,
) -> Result<Json<ApObject>, Status> {
    if let (Some(profile), Signed(true, VerificationType::Local)) =
        (get_profile_by_username(&conn, username).await, signed)
    {
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

#[post("/user/<_username>/inbox", data = "<activity>")]
pub async fn inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    _username: String,
    activity: String,
) -> Result<Status, Status> {
    shared_inbox_post(signed, conn, faktory, events, activity).await
}

#[post("/inbox", data = "<activity>")]
pub async fn shared_inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    activity: String,
) -> Result<Status, Status> {
    let v: Value = serde_json::from_str(&activity).unwrap();
    log::debug!("POSTING TO INBOX\n{v:#?}");

    let activity: ApActivity = serde_json::from_str(&activity).unwrap();

    if let Signed(true, _) = signed {
        let activity = activity.clone();

        if create_remote_activity(&conn, activity.clone().into())
            .await
            .is_some()
        {
            log::debug!("ACTIVITY CREATED");
            match activity {
                ApActivity::Delete(activity) => inbox::activity::delete(conn, *activity).await,
                ApActivity::Create(activity) => {
                    inbox::activity::create(conn, faktory, activity).await
                }
                ApActivity::Follow(activity) => inbox::activity::follow(faktory, activity).await,
                ApActivity::Undo(activity) => {
                    inbox::activity::undo(conn, events, faktory, *activity).await
                }
                ApActivity::Accept(activity) => inbox::activity::accept(faktory, *activity).await,
                ApActivity::Invite(activity) => {
                    inbox::activity::invite(conn, faktory, activity).await
                }
                ApActivity::Join(activity) => inbox::activity::join(conn, faktory, activity).await,
                ApActivity::Announce(activity) => {
                    inbox::activity::announce(conn, faktory, activity).await
                }
                ApActivity::Update(activity) => {
                    inbox::activity::update(conn, faktory, activity).await
                }
                ApActivity::Like(activity) => inbox::activity::like(conn, faktory, *activity).await,
                ApActivity::Block(activity) => {
                    inbox::activity::block(conn, faktory, activity).await
                }
                ApActivity::Add(activity) => inbox::activity::add(conn, faktory, activity).await,
            }
        } else {
            log::debug!("FAILED TO CREATE REMOTE ACTIVITY");
            Err(Status::NoContent)
        }
    } else {
        log::debug!("REQUEST WAS UNSIGNED OR MALFORMED");
        Err(Status::NoContent)
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
    if let (Some(_profile), Signed(true, VerificationType::Local)) =
        (get_profile_by_username(&conn, username).await, signed)
    {
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
