#[macro_use]
extern crate rocket;

use enigmatick::{
    activity_pub::{
        retriever::{self, get_note, get_remote_webfinger},
        ApActivity, ApActivityType, ApActor, ApBaseObjectSuper, ApCollection, ApNote, ApObject,
        ApObjectType, ApOrderedCollection, FollowersPage, LeadersPage,
    },
    admin::{self, verify_and_generate_password},
    api::{instance::InstanceInformation, processing_queue},
    db::{
        create_remote_activity, get_followers_by_profile_id, get_leaders_by_profile_id,
        get_profile_by_username, update_avatar_by_username, update_banner_by_username,
        update_password_by_username, update_summary_by_username, Db,
    },
    fairings::{events::EventChannels, faktory::FaktoryConnection, signatures::Signed},
    helper::{get_local_username_from_ap_id, is_local},
    inbox,
    models::{
        notes::get_note_by_uuid,
        profiles::{
            update_olm_external_identity_keys_by_username, update_olm_sessions_by_username,
            KeyStore, Profile,
        },
    },
    outbox,
    signing::VerificationType,
    webfinger::WebFinger,
};

use faktory::Job;

use rocket::{
    data::{Data, ToByteUnit},
    http::RawStr,
    http::Status,
    request::FromParam,
    response::stream::{Event, EventStream},
    serde::json::{Error, Json},
    tokio::select,
    tokio::time::{self, Duration},
    Shutdown,
};

use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct Handle<'r> {
    name: &'r str,
}

impl<'r> FromParam<'r> for Handle<'r> {
    type Error = &'r RawStr;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if param.starts_with('@') {
            Ok(Handle { name: param })
        } else {
            Err(param.into())
        }
    }
}

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

#[get("/<handle>")]
pub async fn profile(conn: Db, handle: Handle<'_>) -> Result<Json<ApActor>, Status> {
    let mut username = handle.name.to_string();
    username.remove(0);
    match get_profile_by_username(&conn, username).await {
        Some(profile) => Ok(Json(ApActor::from(profile))),
        None => Err(Status::NoContent),
    }
}

#[get("/api/user/<username>/events")]
async fn stream(
    mut shutdown: Shutdown,
    mut events: EventChannels,
    username: String,
) -> EventStream![] {
    EventStream! {
        let (uuid, rx) = events.subscribe(username);
        let mut i = 1;
        let mut k = 0;

        #[derive(Serialize)]
        struct StreamConnect {
            uuid: String
        }

        let message = serde_json::to_string(&StreamConnect { uuid: uuid.clone() }).unwrap();

        yield Event::data(message).event("message").id(i.to_string());
        i += 1;

        let mut interval = time::interval(Duration::from_secs(1));

        loop {
            //debug!("looping");

            select! {
                _ = interval.tick() => {
                    if let Ok(message) = rx.try_recv() {
                        i += 1;
                        yield Event::data(message).event("message").id(i.to_string());
                    }
                },
                _ = &mut shutdown => {
                    yield Event::data("goodbye").event("message");
                    break;
                }
            };

            if k >= 300 {
                //debug!("breaking loop to force reconnection");
                break;
            } else {
                k += 1;
            }
        }

        events.remove(uuid);
    }
}

#[get("/user/<username>/test")]
pub async fn test(
    conn: Db,
    faktory: FaktoryConnection,
    username: String,
) -> Result<Json<ApActor>, Status> {
    match faktory.producer.try_lock() {
        Ok(mut x) => {
            if x.enqueue(Job::new("test_job", vec!["arg"])).is_err() {
                log::error!("failed to enqueue job");
            }
        }
        Err(e) => log::debug!("failed to lock mutex: {}", e),
    }

    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            retriever::get_followers(
                &conn,
                profile.clone(),
                "https://ser.endipito.us/users/justin".to_string(),
                Option::from(1),
            )
            .await;

            Ok(Json(ApActor::from(profile)))
        }
        None => Err(Status::NoContent),
    }
}

#[get("/user/<username>")]
pub async fn person(conn: Db, username: String) -> Result<Json<ApActor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            //debug!("profile\n{:#?}", profile);
            Ok(Json(ApActor::from(profile)))
        }
        None => Err(Status::NoContent),
    }
}

#[get("/.well-known/webfinger?<resource>")]
pub async fn webfinger(conn: Db, resource: String) -> Result<Json<WebFinger>, Status> {
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
pub async fn get_followers(
    conn: Db,
    username: String,
) -> Result<Json<ApOrderedCollection>, Status> {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let followers = get_followers_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApOrderedCollection::from(FollowersPage {
            page: 0,
            profile,
            followers,
        })))

        //log::debug!("followers: {:#?}", followers);
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/following")]
pub async fn get_leaders(conn: Db, username: String) -> Result<Json<ApOrderedCollection>, Status> {
    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let leaders = get_leaders_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApOrderedCollection::from(LeadersPage {
            page: 0,
            profile,
            leaders,
        })))

        //log::debug!("followers: {:#?}", followers);
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
    //debug!("raw\n{:#?}", actor);

    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(note) = note {
            if let Some(profile) = get_profile_by_username(&conn, username).await {
                if let Some(note) = get_note(&conn, profile, note.id.clone()).await {
                    //debug!("here's the note\n{note:#?}");
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
    //debug!("raw\n{:#?}", actor);

    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(actor) = actor {
            if let Some(profile) = get_profile_by_username(&conn, username).await {
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
                        //log::debug!("matching to webfinger");
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
                        log::debug!("matching to https id");
                        Option::from(actor.id.clone())
                    } else {
                        //log::debug!("matched to none");
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
                    } else if let Some(actor) = retriever::get_actor(&conn, profile, ap_id).await {
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
    //debug!("raw\n{:#?}", password);

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

#[derive(Deserialize, Debug, Clone)]
pub struct NewUser {
    pub username: String,
    pub password: String,
    pub display_name: String,
    pub client_public_key: String,
    pub keystore: String,
}

#[post("/api/user/create", format = "json", data = "<user>")]
pub async fn create_user(
    conn: Db,
    user: Result<Json<NewUser>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    //debug!("raw\n{:#?}", user);

    if let Ok(user) = user {
        let keystore_value: serde_json::Value =
            serde_json::from_str(&user.keystore.clone()).unwrap();

        if let Some(profile) = admin::create_user(
            &conn,
            user.username.clone(),
            user.display_name.clone(),
            user.password.clone(),
            Some(user.client_public_key.clone()),
            Some(keystore_value),
        )
        .await
        {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post(
    "/api/user/<username>/update_olm_sessions",
    format = "json",
    data = "<keystore>"
)]
pub async fn update_olm_sessions(
    signed: Signed,
    conn: Db,
    username: String,
    keystore: Result<Json<KeyStore>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    //debug!("raw\n{:#?}", keystore);

    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(Json(keystore)) = keystore {
            if let Some(profile) = update_olm_sessions_by_username(&conn, username, keystore).await
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

#[get("/api/user/<username>/processing_queue")]
pub async fn get_processing_queue(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApObject>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Some(profile) = get_profile_by_username(&conn, username).await {
            let l = processing_queue::retrieve(&conn, profile).await;

            //debug!("queue\n{:#?}", l);
            Ok(Json(ApObject::Collection(ApCollection::from(l))))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post(
    "/api/user/<username>/update_identity_cache",
    format = "json",
    data = "<keystore>"
)]
pub async fn update_identity_cache(
    signed: Signed,
    conn: Db,
    username: String,
    keystore: Result<Json<KeyStore>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    //debug!("raw\n{:#?}", keystore);

    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(Json(keystore)) = keystore {
            if let Some(profile) =
                update_olm_external_identity_keys_by_username(&conn, username, keystore).await
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
    //debug!("raw\n{:#?}", summary);

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
    //debug!("raw\n{:#?}", user);

    if let Ok(user) = user {
        if let Some(profile) =
            admin::authenticate(&conn, user.username.clone(), user.password.clone()).await
        {
            //debug!("sending profile\n{:#?}", profile);
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
    username: String,
    offset: u16,
    limit: u8,
    conversation: String,
) -> Result<Json<ApObject>, Status> {
    //debug!("inbox get request received");

    if let (Some(profile), Signed(true, VerificationType::Local)) =
        (get_profile_by_username(&conn, username).await, signed)
    {
        if let Ok(conversation) = urlencoding::decode(&conversation.clone()) {
            let inbox = inbox::retrieve::conversation(
                &conn,
                conversation.to_string(),
                limit.into(),
                offset.into(),
            )
            .await;
            //debug!("inbox\n{:#?}", inbox);
            Ok(Json(inbox))
        } else {
            Err(Status::NoContent)
        }
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
    object: Result<Json<ApBaseObjectSuper>, Error<'_>>,
) -> Result<Status, Status> {
    debug!("raw\n{:#?}", object);

    if let Signed(true, VerificationType::Local) = signed {
        match get_profile_by_username(&conn, username).await {
            Some(profile) => match object {
                Ok(object) => match object {
                    Json(ApBaseObjectSuper::Activity(activity)) => match activity.kind {
                        ApActivityType::Undo => {
                            outbox::activity::undo(conn, events, activity, profile).await
                        }
                        ApActivityType::Follow => {
                            outbox::activity::follow(conn, events, activity, profile).await
                        }
                        _ => Err(Status::NoContent),
                    },
                    Json(ApBaseObjectSuper::Object(ApObject::Note(note))) => {
                        // EncryptedNotes need to be handled differently, but use the ApNote struct
                        match note.kind {
                            ApObjectType::Note => {
                                outbox::object::note(conn, faktory, events, note, profile).await
                            }
                            ApObjectType::EncryptedNote => {
                                outbox::object::encrypted_note(conn, faktory, events, note, profile)
                                    .await
                            }
                            _ => Err(Status::NoContent),
                        }
                    }
                    Json(ApBaseObjectSuper::Object(ApObject::Session(session))) => {
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
    //debug!("inbox get request received");

    if let (Some(profile), Signed(true, VerificationType::Local)) =
        (get_profile_by_username(&conn, username).await, signed)
    {
        let inbox = inbox::retrieve::timeline(&conn, limit.into(), offset.into()).await;
        //debug!("inbox\n{:#?}", inbox);
        Ok(Json(inbox))
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/inbox", data = "<activity>")]
pub async fn inbox_post(
    signed: Signed,
    conn: Db,
    faktory: FaktoryConnection,
    events: EventChannels,
    username: String,
    activity: String,
    //activity: Result<Json<ApActivity>, Error<'_>>,
) -> Result<Status, Status> {
    //debug!("inbox: {:#?}", activity);
    // Err(Status::NoContent)

    let v: Value = serde_json::from_str(&activity).unwrap();
    debug!("inbox\n{v:#?}");

    let activity: ApActivity = serde_json::from_str(&activity).unwrap();

    if let (Some(profile), Signed(true, _)) =
        (get_profile_by_username(&conn, username).await, signed)
    {
        let activity = activity.clone();

        if retriever::get_actor(&conn, profile.clone(), activity.actor.clone())
            .await
            .is_some()
        {
            if create_remote_activity(&conn, (activity.clone(), profile.id).into())
                .await
                .is_some()
            {
                match activity.kind {
                    ApActivityType::Delete => inbox::activity::delete(conn, activity).await,
                    ApActivityType::Create => {
                        inbox::activity::create(conn, faktory, events, activity, profile).await
                    }
                    ApActivityType::Follow => {
                        inbox::activity::follow(conn, faktory, events, activity, profile).await
                    }
                    ApActivityType::Undo => inbox::activity::undo(conn, events, activity).await,
                    ApActivityType::Accept => inbox::activity::accept(conn, events, activity).await,
                    ApActivityType::Invite => {
                        inbox::activity::invite(conn, faktory, activity, profile).await
                    }
                    ApActivityType::Join => {
                        inbox::activity::join(conn, faktory, activity, profile).await
                    }
                    ApActivityType::Announce => {
                        inbox::activity::announce(conn, faktory, events, activity).await
                    }
                    _ => {
                        debug!("unknown activity\n{activity:#?}");
                        Err(Status::NoContent)
                    }
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        //log::debug!("request was unsigned or malformed in some way");
        Err(Status::NoContent)
    }
}

#[launch]
fn rocket() -> _ {
    env_logger::init();

    rocket::build()
        .attach(FaktoryConnection::fairing())
        .attach(EventChannels::fairing())
        .attach(Db::fairing())
        .mount(
            "/",
            routes![
                person,
                profile,
                webfinger,
                outbox_post,
                inbox_post,
                inbox_get,
                get_followers,
                get_leaders,
                create_user,
                authenticate_user,
                update_identity_cache,
                update_olm_sessions,
                get_processing_queue,
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
                authorize_stream,
            ],
        )
}
