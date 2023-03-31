#[macro_use]
extern crate rocket;

use std::collections::HashMap;

use enigmatick::{
    activity_pub::{
        retriever::{self, get_note, get_remote_webfinger},
        ActivityPub, ApActivity, ApActivityType, ApActor, ApCollection, ApNote, ApNoteType,
        ApObject, ApSession, FollowersPage, IdentifiedVaultItems, LeadersPage,
    },
    admin::{self, verify_and_generate_password, NewUser},
    api::{instance::InstanceInformation, processing_queue},
    db::{
        get_leaders_by_profile_id, update_avatar_by_username, update_banner_by_username,
        update_password_by_username, update_summary_by_username, Db,
    },
    fairings::{events::EventChannels, faktory::FaktoryConnection, signatures::Signed},
    helper::{get_local_username_from_ap_id, is_local},
    inbox,
    models::{
        encrypted_sessions::{
            get_encrypted_session_by_profile_id_and_ap_to, get_encrypted_sessions_by_profile_id,
            EncryptedSession,
        },
        followers::get_followers_by_profile_id,
        notes::get_note_by_uuid,
        olm_one_time_keys::create_olm_one_time_key,
        olm_sessions::{
            get_olm_session_by_uuid, update_olm_session_by_encrypted_session_id, OlmSession,
        },
        processing_queue::resolve_processed_item_by_ap_id_and_profile_id,
        profiles::{get_profile_by_username, update_olm_account_by_username, Profile},
        remote_activities::create_remote_activity,
        vault::{create_vault_item, get_vault_items_by_profile_id_and_remote_actor, VaultItem},
    },
    outbox,
    signing::VerificationType,
    webfinger::WebFinger,
};

use rocket::{
    data::{Data, ToByteUnit},
    http::RawStr,
    http::Status,
    request::FromParam,
    response::{
        stream::{Event, EventStream},
        Redirect,
    },
    serde::json::{Error, Json},
    tokio::select,
    tokio::time::{self, Duration},
    Shutdown,
};

use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use serde_json::Value;

// // This handle functionality is unused because it is currently managed by Svelte directly

// #[derive(Debug)]
// pub struct Handle<'r> {
//     name: &'r str,
//     local: bool,
// }

// impl<'r> FromParam<'r> for Handle<'r> {
//     type Error = &'r RawStr;

//     fn from_param(param: &'r str) -> Result<Self, Self::Error> {
//         log::debug!("EVALUATING PARAM: {param}");

//         let remote_re =
//             regex::Regex::new(r#"^@([a-zA-Z0-9_]+)@((?:[a-zA-Z0-9\-_]+\.)+[a-zA-Z0-9\-_]+)$"#)
//                 .unwrap();
//         let local_re = regex::Regex::new(r#"^@([a-zA-Z0-9_]+)$"#).unwrap();

//         if remote_re.is_match(param) {
//             Ok(Handle {
//                 name: param,
//                 local: false,
//             })
//         } else if local_re.is_match(param) {
//             Ok(Handle {
//                 name: param,
//                 local: true,
//             })
//         } else {
//             Err(param.into())
//         }
//     }
// }

// #[get("/<handle>")]
// pub async fn profile(conn: Db, handle: Handle<'_>) -> Result<Json<ApActor>, Status> {
//     let mut username = handle.name.to_string();

//     if handle.local {
//         username.remove(0);
//         match get_profile_by_username(&conn, username).await {
//             Some(profile) => Ok(Json(ApActor::from(profile))),
//             None => Err(Status::NoContent),
//         }
//     } else {
//         log::debug!("REMOTE LOOKUP\n{handle:#?}");
//         Err(Status::NoContent)
//     }
// }

pub struct ApiVersion<'r> {
    _version: &'r str,
}

impl<'r> FromParam<'r> for ApiVersion<'r> {
    type Error = &'r RawStr;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if param == "v1" || param == "v2" {
            Ok(ApiVersion { _version: param })
        } else {
            Err(param.into())
        }
    }
}

struct DropGuard {
    events: EventChannels,
    uuid: String,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        log::debug!("REMOVING EVENT LISTENER {}", self.uuid.clone());
        self.events.remove(self.uuid.clone());
    }
}

#[get("/api/user/<username>/events")]
fn stream(mut shutdown: Shutdown, mut events: EventChannels, username: String) -> EventStream![] {
    EventStream! {
        let (uuid, rx) = events.subscribe(username);
        let _drop_guard = DropGuard { events: events.clone(), uuid: uuid.clone() };
        let mut i = 1;

        #[derive(Serialize)]
        struct StreamConnect {
            uuid: String
        }

        let message = serde_json::to_string(&StreamConnect { uuid: uuid.clone() }).unwrap();

        yield Event::data(message).event("message").id(i.to_string());
        i += 1;

        let mut interval = time::interval(Duration::from_secs(1));

        loop {
            select! {
                _ = interval.tick() => {
                    if let Ok(message) = rx.try_recv() {
                        log::debug!("SENDING MESSAGE TO {uuid}");
                        i += 1;
                        yield Event::data(message).event("message").id(i.to_string())
                    }
                },
                _ = &mut shutdown => {
                    log::debug!("SHUTDOWN {uuid}");
                    yield Event::data("goodbye").event("message");
                    break;
                }
            };
        }
    }
}

#[get("/user/<_username>/test")]
pub async fn test(
    _conn: Db,
    _faktory: FaktoryConnection,
    _username: String,
) -> Result<Status, Status> {
    // match faktory.producer.try_lock() {
    //     Ok(mut x) => {
    //         if x.enqueue(Job::new("test_job", vec!["arg"])).is_err() {
    //             log::error!("failed to enqueue job");
    //         }
    //     }
    //     Err(e) => log::debug!("failed to lock mutex: {}", e),
    // }

    // match get_profile_by_username(&conn, username).await {
    //     Some(profile) => {
    //         retriever::get_followers(
    //             &conn,
    //             profile.clone(),
    //             "https://ser.endipito.us/users/justin".to_string(),
    //             Option::from(1),
    //         )
    //         .await;

    //         Ok(Json(ApActor::from(profile)))
    //     }
    //     None => Err(Status::NoContent),
    // }

    Ok(Status::Accepted)
}

#[get("/user/<username>", format = "text/html", rank = 1)]
pub async fn person_redirect(username: String) -> Redirect {
    log::debug!("REDIRECTING {username}");
    Redirect::to(format!("/@{username}"))
}

#[get("/user/<username>", rank = 2)]
pub async fn person(conn: Db, username: String) -> Result<Json<ApActor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => Ok(Json(ApActor::from(profile))),
        None => Err(Status::NoContent),
    }
}

#[get("/.well-known/host-meta", format = "application/xrd+xml")]
pub async fn host_meta() -> Result<String, Status> {
    Ok(r#"<?xml version="1.0" encoding="UTF-8"?><XRD xmlns="http://docs.oasis-open.org/ns/xri/xrd-1.0"><Link rel="lrdd" template="https://enigmatick.jdt.dev/.well-known/webfinger?resource={uri}" type="application/json" /></XRD>"#.to_string())
}

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

        let server_url = (*enigmatick::SERVER_URL).clone();

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

#[get("/user/<username>/followers")]
pub async fn get_followers(conn: Db, username: String) -> Result<Json<ApCollection>, Status> {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let followers = get_followers_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApCollection::from(FollowersPage {
            page: 0,
            profile,
            followers,
        })))
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/following")]
pub async fn get_leaders(conn: Db, username: String) -> Result<Json<ApCollection>, Status> {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let leaders = get_leaders_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApCollection::from(LeadersPage {
            page: 0,
            profile,
            leaders,
        })))
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SessionUpdate {
    pub session_uuid: String,
    pub encrypted_session: String,
    pub session_hash: String,
    pub mutation_of: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VaultStorageRequest {
    pub data: String,
    pub remote_actor: String,
    pub session: SessionUpdate,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VaultStorageResponse {
    pub uuid: Option<String>,
}

impl From<Option<VaultItem>> for VaultStorageResponse {
    fn from(item: Option<VaultItem>) -> Self {
        VaultStorageResponse {
            uuid: {
                if let Some(item) = item {
                    Option::from(item.uuid)
                } else {
                    Option::None
                }
            },
        }
    }
}

#[post("/api/user/<username>/vault", data = "<data>")]
pub async fn store_vault_item(
    signed: Signed,
    conn: Db,
    username: String,
    data: Json<VaultStorageRequest>,
) -> Result<Json<VaultStorageResponse>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        log::debug!("STORE VAULT REQUEST\n{data:#?}");

        if let Some(profile) = get_profile_by_username(&conn, username).await {
            let data = data.0;
            let session_update = data.clone().session;

            if let Some((olm_session, Some(encrypted_session))) =
                get_olm_session_by_uuid(&conn, session_update.session_uuid).await
            {
                if encrypted_session.profile_id == profile.id {
                    if update_olm_session_by_encrypted_session_id(
                        &conn,
                        olm_session.encrypted_session_id,
                        session_update.encrypted_session,
                        session_update.session_hash,
                    )
                    .await
                    .is_some()
                    {
                        Ok(Json(
                            create_vault_item(
                                &conn,
                                (data.clone().data, profile.id, data.clone().remote_actor).into(),
                            )
                            .await
                            .into(),
                        ))
                    } else {
                        Err(Status::Unauthorized)
                    }
                } else {
                    Err(Status::Unauthorized)
                }
            } else {
                Err(Status::Unauthorized)
            }
        } else {
            Err(Status::Unauthorized)
        }
    } else {
        Err(Status::Unauthorized)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VaultRetrievalItem {
    pub created_at: String,
    pub updated_at: String,
    pub uuid: String,
    pub remote_actor: String,
    pub data: String,
}

impl From<VaultItem> for VaultRetrievalItem {
    fn from(item: VaultItem) -> Self {
        VaultRetrievalItem {
            created_at: item.created_at.to_rfc2822(),
            updated_at: item.updated_at.to_rfc2822(),
            uuid: item.uuid,
            remote_actor: item.remote_actor,
            data: item.encrypted_data,
        }
    }
}

#[get("/api/user/<username>/vault?<offset>&<limit>&<actor>")]
pub async fn vault_get(
    signed: Signed,
    conn: Db,
    username: String,
    offset: u16,
    limit: u8,
    actor: String,
) -> Result<Json<ApObject>, Status> {
    if let (Some(profile), Signed(true, VerificationType::Local)) =
        (get_profile_by_username(&conn, username).await, signed)
    {
        if let Ok(actor) = base64::decode(actor) {
            let items: Vec<VaultItem> = get_vault_items_by_profile_id_and_remote_actor(
                &conn,
                profile.id,
                limit.into(),
                offset.into(),
                String::from_utf8(actor).unwrap(),
            )
            .await;

            Ok(Json(ApObject::Collection(ApCollection::from(
                (items, profile) as IdentifiedVaultItems,
            ))))
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/api/user/<username>/avatar?<extension>", data = "<media>")]
pub async fn upload_avatar(
    signed: Signed,
    conn: Db,
    username: String,
    extension: String,
    media: Data<'_>,
) -> Result<Status, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        let filename = format!(
            "{}.{}",
            Alphanumeric.sample_string(&mut rand::thread_rng(), 16),
            extension
        );

        if let Ok(file) = media
            .open(2.mebibytes())
            .into_file(&format!("{}/{}", *enigmatick::MEDIA_DIR, filename))
            .await
        {
            if file.is_complete() {
                if update_avatar_by_username(&conn, username, filename)
                    .await
                    .is_some()
                {
                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/api/user/<username>/banner?<extension>", data = "<media>")]
pub async fn upload_banner(
    signed: Signed,
    conn: Db,
    username: String,
    extension: String,
    media: Data<'_>,
) -> Result<Status, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        let filename = format!(
            "{}.{}",
            Alphanumeric.sample_string(&mut rand::thread_rng(), 16),
            extension
        );

        if let Ok(file) = media
            .open(2.mebibytes())
            .into_file(&format!("{}/{}", *enigmatick::MEDIA_DIR, filename))
            .await
        {
            if file.is_complete() {
                if update_banner_by_username(&conn, username, filename)
                    .await
                    .is_some()
                {
                    Ok(Status::Accepted)
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/api/<_version>/instance")]
pub async fn instance_information(
    _version: ApiVersion<'_>,
) -> Result<Json<InstanceInformation>, Status> {
    Ok(Json(InstanceInformation::default()))
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StreamAuthorization {
    uuid: String,
}

#[post(
    "/api/user/<username>/events/authorize",
    format = "application/activity+json",
    data = "<authorization>"
)]
pub async fn authorize_stream(
    signed: Signed,
    mut events: EventChannels,
    username: String,
    authorization: Result<Json<StreamAuthorization>, Error<'_>>,
) -> Result<Json<StreamAuthorization>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(authorization) = authorization {
            events.authorize(authorization.uuid.clone(), username);
            Ok(authorization)
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::Forbidden)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Lookup {
    id: String,
}

#[post("/api/user/<username>/remote/note", format = "json", data = "<note>")]
pub async fn remote_note_lookup(
    signed: Signed,
    conn: Db,
    username: String,
    note: Result<Json<Lookup>, Error<'_>>,
) -> Result<Json<ApNote>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(note) = note {
            if let Some(profile) = get_profile_by_username(&conn, username).await {
                if let Some(note) = get_note(&conn, profile, note.id.clone()).await {
                    Ok(Json(note))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/api/user/<username>/remote/actor", format = "json", data = "<actor>")]
pub async fn remote_actor_lookup(
    signed: Signed,
    conn: Db,
    username: String,
    actor: Result<Json<Lookup>, Error<'_>>,
) -> Result<Json<ApActor>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(actor) = actor {
            if let Some(profile) = get_profile_by_username(&conn, username.clone()).await {
                let ap_id = {
                    let webfinger_re = regex::Regex::new(r#"@(.+?)@(.+)"#).unwrap();
                    let http_url_re =
                        regex::Regex::new(r#"https://(.+?)/@([a-zA-Z0-9_]+)"#).unwrap();
                    let http_id_re = regex::Regex::new(r#"https://.+"#).unwrap();

                    let webfinger = {
                        if http_url_re.is_match(&actor.id.clone()) {
                            let id = &actor.id.clone();
                            let captures = http_url_re.captures(id);
                            if let Some(captures) = captures {
                                if captures.len() == 3 {
                                    if let (Some(_whole), Some(server), Some(user)) =
                                        (captures.get(0), captures.get(1), captures.get(2))
                                    {
                                        Option::from(format!(
                                            "@{}@{}",
                                            user.as_str(),
                                            server.as_str()
                                        ))
                                    } else {
                                        Option::None
                                    }
                                } else {
                                    Option::None
                                }
                            } else {
                                Option::None
                            }
                        } else if webfinger_re.is_match(&actor.id.clone()) {
                            Option::from(actor.id.clone())
                        } else {
                            Option::None
                        }
                    };

                    if let Some(webfinger) = webfinger {
                        if let Some(webfinger) = get_remote_webfinger(webfinger).await {
                            let mut ap_id_int = Option::<String>::None;
                            for link in webfinger.links {
                                if let (Some(kind), Some(href)) = (link.kind, link.href) {
                                    if kind == "application/activity+json" || kind == "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"" {
                                        ap_id_int = Option::from(href);
                                    }
                                }
                            }
                            ap_id_int
                        } else {
                            Option::None
                        }
                    } else if http_id_re.is_match(&actor.id.clone()) {
                        Option::from(actor.id.clone())
                    } else {
                        Option::None
                    }
                };

                if let Some(ap_id) = ap_id {
                    if is_local(ap_id.clone()) {
                        if let Some(username) = get_local_username_from_ap_id(ap_id.clone()) {
                            Ok(Json(
                                get_profile_by_username(&conn, username)
                                    .await
                                    .unwrap()
                                    .into(),
                            ))
                        } else {
                            Err(Status::NoContent)
                        }
                    } else if let Some(actor) =
                        retriever::get_actor(&conn, ap_id, Some(profile)).await
                    {
                        Ok(Json(actor.into()))
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct UpdatePassword {
    pub current: String,
    pub updated: String,
}

#[post("/api/user/<username>/password", format = "json", data = "<password>")]
pub async fn change_password(
    signed: Signed,
    conn: Db,
    username: String,
    password: Result<Json<UpdatePassword>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(password) = password {
            if let Some(password) = verify_and_generate_password(
                &conn,
                username.clone(),
                password.current.clone(),
                password.updated.clone(),
            )
            .await
            {
                Ok(Json(
                    update_password_by_username(&conn, username, password)
                        .await
                        .unwrap_or_default(),
                ))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::BadRequest)
    }
}

#[post("/api/user/create", format = "json", data = "<user>")]
pub async fn create_user(
    conn: Db,
    user: Result<Json<NewUser>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    if let Ok(Json(user)) = user {
        log::debug!("CREATING USER\n{user:#?}");

        if let Some(profile) = admin::create_user(&conn, user).await {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/api/user/<username>/sessions", format = "json")]
pub async fn get_sessions(
    //    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApObject>, Status> {
    //  if let Signed(true, VerificationType::Local) = signed {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let sessions: Vec<(EncryptedSession, Option<OlmSession>)> =
            get_encrypted_sessions_by_profile_id(&conn, profile.id).await;

        // this converts EncryptedSession to ApSession and (ApSession, Option<OlmSession>)
        // into merged Vec<ApObject::Session> in one shot - see types/session.rs for details
        let normalized: Vec<ApObject> = sessions
            .iter()
            .map(|(x, y)| ApObject::Session(((*x).clone().into(), (*y).clone()).into()))
            .collect();

        Ok(Json(ApObject::Collection(ApCollection::from(normalized))))
    } else {
        Err(Status::NoContent)
    }
    // } else {
    //     Err(Status::NoContent)
    // }
}

#[get("/api/user/<username>/session/<encoded>")]
pub async fn get_olm_session(
    //    signed: Signed,
    conn: Db,
    username: String,
    encoded: String,
) -> Result<Json<ApSession>, Status> {
    //  if let Signed(true, VerificationType::Local) = signed {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        if let Ok(id) = base64::decode(encoded) {
            if let Ok(id) = String::from_utf8(id) {
                if let Some((encrypted_session, olm_session)) =
                    get_encrypted_session_by_profile_id_and_ap_to(&conn, profile.id, id).await
                {
                    Ok(Json((encrypted_session.into(), olm_session).into()))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
    // } else {
    //     Err(Status::NoContent)
    // }
}
#[get("/api/user/<username>/queue")]
pub async fn get_processing_queue(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApObject>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username(&conn, username).await {
            let l = processing_queue::retrieve(&conn, profile).await;

            Ok(Json(ApObject::Collection(ApCollection::from(l))))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub enum QueueTask {
    Resolve,
    #[default]
    Unknown,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct QueueAction {
    id: String,
    action: QueueTask,
}

#[post("/api/user/<username>/queue", format = "json", data = "<item>")]
pub async fn update_processing_queue_item(
    signed: Signed,
    conn: Db,
    username: String,
    item: Result<Json<QueueAction>, Error<'_>>,
) -> Result<Status, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(Json(item)) = item {
            if let Some(profile) = get_profile_by_username(&conn, username).await {
                if item.action == QueueTask::Resolve {
                    if resolve_processed_item_by_ap_id_and_profile_id(&conn, profile.id, item.id)
                        .await
                        .is_some()
                    {
                        Ok(Status::Accepted)
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OtkUpdateParams {
    pub keys: HashMap<String, String>,
    pub account: String,
    pub mutation_of: String,
    pub account_hash: String,
}

#[post("/api/user/<username>/otk", format = "json", data = "<params>")]
pub async fn add_one_time_keys(
    signed: Signed,
    conn: Db,
    username: String,
    params: Result<Json<OtkUpdateParams>, Error<'_>>,
) -> Result<Status, Status> {
    debug!("ADDING ONE-TIME-KEYS\n{params:#?}");

    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username(&conn, username.clone()).await {
            if let Ok(Json(params)) = params {
                if profile.olm_pickled_account_hash == params.mutation_of.into() {
                    if update_olm_account_by_username(
                        &conn,
                        username,
                        params.account,
                        params.account_hash,
                    )
                    .await
                    .is_some()
                    {
                        for (key, otk) in params.keys {
                            create_olm_one_time_key(&conn, (profile.id, key, otk).into()).await;
                        }

                        Ok(Status::Accepted)
                    } else {
                        Err(Status::NoContent)
                    }
                } else {
                    error!("UNEXPECTED MUTATION");
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct SummaryUpdate {
    content: String,
}

#[post(
    "/api/user/<username>/update/summary",
    format = "json",
    data = "<summary>"
)]
pub async fn update_summary(
    signed: Signed,
    conn: Db,
    username: String,
    summary: Result<Json<SummaryUpdate>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    log::debug!("UPDATING SUMMARY\n{summary:#?}");

    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(Json(summary)) = summary {
            if let Some(profile) =
                update_summary_by_username(&conn, username, summary.content).await
            {
                Ok(Json(profile))
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct AuthenticationData {
    pub username: String,
    pub password: String,
}

#[post("/api/user/authenticate", format = "json", data = "<user>")]
pub async fn authenticate_user(
    conn: Db,
    user: Result<Json<AuthenticationData>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    log::debug!("AUTHENTICATING\n{user:#?}");

    if let Ok(user) = user {
        if let Some(profile) =
            admin::authenticate(&conn, user.username.clone(), user.password.clone()).await
        {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
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
    let conversation = format!("{}/conversation/{}", *enigmatick::SERVER_URL, uuid);

    Ok(Json(
        inbox::retrieve::conversation(&conn, faktory, conversation.to_string(), 40, 0).await,
    ))
}

#[get("/user/<username>/liked")]
pub async fn liked_get(conn: Db, username: String) -> Result<Json<ApCollection>, Status> {
    if let Some(_profile) = get_profile_by_username(&conn, username).await {
        Ok(Json(ApCollection::default()))
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/outbox")]
pub async fn outbox_get(conn: Db, username: String) -> Result<Json<ApCollection>, Status> {
    if let Some(_profile) = get_profile_by_username(&conn, username).await {
        Ok(Json(ApCollection::default()))
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/outbox", data = "<object>")]
pub async fn outbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    username: String,
    object: Result<Json<ActivityPub>, Error<'_>>,
) -> Result<Status, Status> {
    log::debug!("POSTING TO OUTBOX\n{object:#?}");

    if let Signed(true, VerificationType::Local) = signed {
        match get_profile_by_username(&conn, username).await {
            Some(profile) => match object {
                Ok(object) => match object {
                    Json(ActivityPub::Activity(activity)) => match activity.kind {
                        ApActivityType::Undo => {
                            outbox::activity::undo(conn, events, activity, profile).await
                        }
                        ApActivityType::Follow => {
                            outbox::activity::follow(conn, events, activity, profile).await
                        }
                        ApActivityType::Like => {
                            outbox::activity::like(conn, faktory, events, activity, profile).await
                        }
                        _ => Err(Status::NoContent),
                    },
                    Json(ActivityPub::Object(ApObject::Note(note))) => {
                        // EncryptedNotes need to be handled differently, but use the ApNote struct
                        match note.kind {
                            ApNoteType::Note => {
                                outbox::object::note(conn, faktory, events, note, profile).await
                            }
                            ApNoteType::EncryptedNote => {
                                outbox::object::encrypted_note(conn, faktory, events, note, profile)
                                    .await
                            }
                            _ => Err(Status::NoContent),
                        }
                    }
                    Json(ActivityPub::Object(ApObject::Session(session))) => {
                        outbox::object::session(conn, faktory, session, profile).await
                    }
                    _ => Err(Status::NoContent),
                },
                Err(_) => Err(Status::NoContent),
            },
            None => Err(Status::NoContent),
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/notes/<uuid>")]
pub async fn note_get(conn: Db, uuid: String) -> Result<Json<ApNote>, Status> {
    if let Some(x) = get_note_by_uuid(&conn, uuid).await {
        Ok(Json(x.into()))
    } else {
        Err(Status::NoContent)
    }
}

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

        if retriever::get_actor(&conn, activity.actor.clone(), Option::None)
            .await
            .is_some()
        {
            if create_remote_activity(&conn, activity.clone().into())
                .await
                .is_some()
            {
                log::debug!("ACTIVITY CREATED");
                match activity.kind {
                    ApActivityType::Delete => inbox::activity::delete(conn, activity).await,
                    ApActivityType::Create => {
                        inbox::activity::create(conn, faktory, events, activity).await
                    }
                    ApActivityType::Follow => {
                        log::debug!("LOOKS LIKE A FOLLOW ACTIVITY");
                        inbox::activity::follow(conn, faktory, events, activity).await
                    }
                    ApActivityType::Undo => inbox::activity::undo(conn, events, activity).await,
                    ApActivityType::Accept => inbox::activity::accept(conn, events, activity).await,
                    ApActivityType::Invite => {
                        inbox::activity::invite(conn, faktory, activity).await
                    }
                    ApActivityType::Join => inbox::activity::join(conn, faktory, activity).await,
                    ApActivityType::Announce => {
                        inbox::activity::announce(conn, faktory, events, activity).await
                    }
                    ApActivityType::Update => {
                        inbox::activity::update(conn, faktory, activity).await
                    }
                    ApActivityType::Like => inbox::activity::like(conn, faktory, activity).await,
                    _ => {
                        log::warn!("UNIMPLEMENTED ACTIVITY\n{activity:#?}");
                        Err(Status::NoContent)
                    }
                }
            } else {
                log::debug!("FAILED TO CREATE REMOTE ACTIVITY");
                Err(Status::NoContent)
            }
        } else {
            log::debug!("FAILED TO RETRIEVE ACTOR");
            Err(Status::NoContent)
        }
    } else {
        log::debug!("REQUEST WAS UNSIGNED OR MALFORMED");
        Err(Status::NoContent)
    }
}

#[launch]
fn rocket() -> _ {
    if let Ok(profile) = std::env::var("PROFILE") {
        match profile.as_str() {
            "debug" => log4rs::init_file("log4rs.yml", Default::default()).unwrap(),
            "release" => env_logger::init(),
            _ => (),
        }
    } else {
        env_logger::init();
    }

    rocket::build()
        .attach(FaktoryConnection::fairing())
        .attach(EventChannels::fairing())
        .attach(Db::fairing())
        .mount(
            "/",
            routes![
                person_redirect,
                person,
                //profile,
                webfinger_json,
                webfinger_xml,
                outbox_post,
                outbox_get,
                inbox_post,
                shared_inbox_post,
                timeline,
                inbox_get,
                liked_get,
                get_followers,
                get_leaders,
                create_user,
                authenticate_user,
                add_one_time_keys,
                get_processing_queue,
                update_processing_queue_item,
                get_olm_session,
                get_sessions,
                store_vault_item,
                vault_get,
                test,
                stream,
                instance_information,
                remote_actor_lookup,
                remote_note_lookup,
                update_summary,
                upload_avatar,
                upload_banner,
                change_password,
                note_get,
                conversation_get,
                conversation_get_local,
                authorize_stream,
                host_meta
            ],
        )
}
