#[macro_use]
extern crate rocket;

use enigmatick::{
    activity_pub::{
        retriever::{self, get_remote_webfinger},
        ApActivity, ApActivityType, ApActor, ApBaseObjectSuper, ApCollection, ApObject,
        ApOrderedCollection, FollowersPage, LeadersPage,
    },
    admin,
    api::{instance::InstanceInformation, processing_queue},
    db::{
        create_remote_activity, get_followers_by_profile_id, get_leaders_by_profile_id,
        get_profile_by_username, Db,
    },
    fairings::{events::EventChannels, faktory::FaktoryConnection, signatures::Signed},
    inbox,
    models::{
        profiles::{
            update_olm_external_identity_keys_by_username, update_olm_sessions_by_username,
            KeyStore, Profile,
        },
        remote_actors::RemoteActor,
    },
    outbox,
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
    Request, Shutdown,
};

use serde::Deserialize;

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
        let rx = events.subscribe(username);
        let mut interval = time::interval(Duration::from_secs(1));

        let mut i = 1;
        let mut k = 0;

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
            debug!("profile\n{:#?}", profile);
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

#[post("/api/user/<username>/avatar", data = "<media>")]
pub async fn upload_avatar(
    signed: Signed,
    username: String,
    media: Data<'_>,
) -> Result<Status, Status> {
    if let Signed(true) = signed {
        let stream = media
            .open(2.mebibytes())
            .into_file("/srv/data/file.png")
            .await;

        Ok(Status::Accepted)
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

#[derive(Deserialize, Debug, Clone)]
pub struct ActorLookup {
    webfinger: String,
}

#[post("/api/user/<username>/remote", format = "json", data = "<actor>")]
pub async fn remote_actor_lookup(
    //signed: Signed,
    conn: Db,
    username: String,
    actor: Result<Json<ActorLookup>, Error<'_>>,
) -> Result<Json<RemoteActor>, Status> {
    debug!("raw\n{:#?}", actor);

    //if let Signed(true) = signed {
    if let Ok(actor) = actor {
        if let Some(profile) = get_profile_by_username(&conn, username).await {
            if let Some(webfinger) = get_remote_webfinger(actor.webfinger.clone()).await {
                let mut ap_id = Option::<String>::None;
                for link in webfinger.links {
                    if let (Some(kind), Some(href)) = (link.kind, link.href) {
                        if kind == "application/activity+json" {
                            ap_id = Option::from(href);
                        }
                    }
                }

                if let Some(ap_id) = ap_id {
                    // this should be converted to an ApActor
                    Ok(Json(
                        retriever::get_actor(&conn, profile, ap_id)
                            .await
                            .unwrap()
                            .into(),
                    ))
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
    debug!("raw\n{:#?}", user);

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
    debug!("raw\n{:#?}", keystore);

    if let Signed(true) = signed {
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
    if let Signed(true) = signed {
        if let Some(profile) = get_profile_by_username(&conn, username).await {
            let l = processing_queue::retrieve(&conn, profile).await;

            debug!("queue\n{:#?}", l);
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
    debug!("raw\n{:#?}", keystore);

    if let Signed(true) = signed {
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
pub struct AuthenticationData {
    pub username: String,
    pub password: String,
}

#[post("/api/user/authenticate", format = "json", data = "<user>")]
pub async fn authenticate_user(
    conn: Db,
    user: Result<Json<AuthenticationData>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    debug!("raw\n{:#?}", user);

    if let Ok(user) = user {
        if let Some(profile) =
            admin::authenticate(&conn, user.username.clone(), user.password.clone()).await
        {
            debug!("sending profile\n{:#?}", profile);
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/outbox", data = "<object>")]
pub async fn outbox_post(
    //signed: Signed,
    conn: Db,
    username: String,
    object: Result<Json<ApBaseObjectSuper>, Error<'_>>,
) -> Result<Status, Status> {
    debug!("raw\n{:#?}", object);

    //if let Signed(true) = signed {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => match object {
            Ok(object) => match object {
                Json(ApBaseObjectSuper::Activity(activity)) => {
                    if create_remote_activity(&conn, (activity.clone(), profile.id).into())
                        .await
                        .is_some()
                    {
                        match activity.kind {
                            ApActivityType::Undo => {
                                outbox::activity::undo(conn, activity, profile).await
                            }
                            ApActivityType::Follow => {
                                outbox::activity::follow(conn, activity, profile).await
                            }
                            _ => Err(Status::NoContent),
                        }
                    } else {
                        Err(Status::NoContent)
                    }
                }
                Json(ApBaseObjectSuper::Object(ApObject::Note(note))) => {
                    outbox::object::note(conn, note, profile).await
                }
                Json(ApBaseObjectSuper::Object(ApObject::Session(session))) => {
                    outbox::object::session(conn, session, profile).await
                }
                _ => Err(Status::NoContent),
            },
            Err(_) => Err(Status::NoContent),
        },
        None => Err(Status::NoContent),
    }
    // } else {
    //     Err(Status::NoContent)
    // }
}

#[get("/user/<username>/inbox")]
pub async fn inbox_get(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<Json<ApObject>, Status> {
    debug!("inbox get request received");

    if let (Some(profile), Signed(true)) = (get_profile_by_username(&conn, username).await, signed)
    {
        let inbox = inbox::retrieve::all(conn, profile).await;
        debug!("inbox\n{:#?}", inbox);
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
    activity: Result<Json<ApActivity>, Error<'_>>,
) -> Result<Status, Status> {
    debug!("inbox: {:#?}", activity);

    if let (Ok(activity), Some(profile), Signed(true)) = (
        activity,
        get_profile_by_username(&conn, username).await,
        signed,
    ) {
        let activity = activity.0.clone();

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
                        inbox::activity::follow(conn, faktory, activity, profile).await
                    }
                    ApActivityType::Undo => inbox::activity::undo(conn, activity).await,
                    ApActivityType::Accept => inbox::activity::accept(conn, activity).await,
                    ApActivityType::Invite => {
                        inbox::activity::invite(conn, faktory, activity, profile).await
                    }
                    ApActivityType::Join => {
                        inbox::activity::join(conn, faktory, activity, profile).await
                    }
                    _ => Err(Status::NoContent),
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        log::debug!("request was unsigned or malformed in some way");
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
                upload_avatar,
            ],
        )
}
