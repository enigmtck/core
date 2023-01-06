#[macro_use]
extern crate rocket;

use enigmatick::{
    admin,
    outbox,
    inbox,
    FaktoryConnection,
    models::{
        remote_activities::NewRemoteActivity,
        profiles::Profile,
    },
    activity_pub::{
        ApObject,
        ApActor,
        ApActivity,
        retriever,
        ApActivityType,
        ApOrderedCollection,
        FollowersPage,
        LeadersPage,
        ApBaseObjectSuper
    },
    webfinger::WebFinger,
    db::{
        Db,
        get_profile_by_username,
        create_remote_activity,
        get_followers_by_profile_id,
        get_leaders_by_profile_id
    },
    signing::{verify, VerifyParams}
};

use faktory::Job;

use rocket::{
    Response,
    http::RawStr,
    request::{FromParam, FromRequest, Request, Outcome},
    serde::json::{Json, Error},
    http::{Status, Header},
    fairing::{Fairing, Info, Kind},
};

use serde::Deserialize;

pub struct Signed(bool);

#[derive(Debug)]
pub enum SignatureError {
    NonExistent,
    MultipleSignatures,
    InvalidRequestPath,
    InvalidRequestUsername,
    LocalUserNotFound,
    SignatureInvalid
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Signed {
    type Error = SignatureError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let conn = request.guard::<Db>().await.unwrap();
        let method = request.method().to_string();
        let host = request.host().unwrap().to_string();
        let path = request.uri().path().to_string();

        log::debug!("request: {:#?}", request);

        let username_re = regex::Regex::new(r"(/user/)([a-zA-Z0-9_]+)(/.*)").unwrap();
        if let Some(username_match) = username_re.captures(&path) {
            if let Some(username) = username_match.get(2) {
                match get_profile_by_username(&conn, username.as_str().to_string()).await {
                    Some(profile) => {
                        
                        let request_target = format!("{} {}", method.to_lowercase(), path);

                        let mut date = String::new();
                        let date_vec: Vec<_> = request.headers().get("date").collect();
                        if date_vec.len() == 1 {
                            date = date_vec[0].to_string();
                        } else {
                            // browser fetch is a jerk and forbids the "date" header; browsers
                            // aggressively strip it, so I use Enigmatick-Date as a backup
                            let enigmatick_date_vec: Vec<_> =
                                request.headers().get("enigmatick-date").collect();

                            if enigmatick_date_vec.len() == 1 {
                                date = enigmatick_date_vec[0].to_string();
                            }
                        }

                        let mut digest = Option::<String>::None;
                        let digest_vec: Vec<_> = request.headers().get("digest").collect();
                        if digest_vec.len() == 1 {
                            digest = Option::from(digest_vec[0].to_string());
                        }

                        let content_type = request.content_type().unwrap().to_string();

                        let signature_vec: Vec<_> = request.headers().get("signature").collect();
                        let signature = signature_vec[0].to_string();
                        
                        match signature_vec.len() {
                            0 => Outcome::Failure((Status::BadRequest, SignatureError::NonExistent)),
                            1 => {
                                if verify(conn,
                                          VerifyParams { profile,
                                                         signature,
                                                         request_target,
                                                         host,
                                                         date,
                                                         digest,
                                                         content_type }).await {
                                    Outcome::Success(Signed(true))
                                } else {
                                    Outcome::Success(Signed(false))
                                }
                            },
                            _ => Outcome::Failure((Status::BadRequest,
                                                   SignatureError::MultipleSignatures)),
                        }
                    },
                    None => Outcome::Failure((Status::BadRequest, SignatureError::LocalUserNotFound)),
                }
            } else {
                Outcome::Failure((Status::BadRequest, SignatureError::InvalidRequestUsername))
            }
        } else {
            Outcome::Failure((Status::BadRequest, SignatureError::InvalidRequestPath))
        }
    }
}

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

#[get("/<handle>")]
pub async fn profile(conn: Db, handle: Handle<'_>) -> Result<Json<ApActor>, Status> {
    let mut username = handle.name.to_string();
    username.remove(0);
    match get_profile_by_username(&conn, username).await {
        Some(profile) => Ok(Json(ApActor::from(profile))),
        None => Err(Status::NoContent)
    }
}

#[get("/user/<username>/test")]
pub async fn test(conn: Db, faktory: FaktoryConnection, username: String)
                  -> Result<Json<ApActor>, Status> {
    match faktory.producer.try_lock() {
        Ok(mut x) => {
            if x.enqueue(Job::new("test_job", vec!["arg"]))
                .is_err() {
                    log::error!("failed to enqueue job");
                }
        },
        Err(e) => log::debug!("failed to lock mutex: {}", e)
    }
    
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            retriever::get_followers(&conn,
                                     profile.clone(),
                                     "https://ser.endipito.us/users/justin".to_string(),
                                     Option::from(1)).await;
            
            Ok(Json(ApActor::from(profile)))
        },
        None => Err(Status::NoContent)
    }
}

#[get("/user/<username>")]
pub async fn person(conn: Db, username: String) -> Result<Json<ApActor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            debug!("profile\n{:#?}", profile);
            Ok(Json(ApActor::from(profile)))
        },
        None => Err(Status::NoContent)
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
            None => Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/followers")]
pub async fn get_followers(conn: Db, username: String) -> Result<Json<ApOrderedCollection>, Status> {

    if let Some(profile) = get_profile_by_username(&conn, username).await {
        let followers = get_followers_by_profile_id(&conn, profile.id).await;

        Ok(Json(ApOrderedCollection::from(FollowersPage {
            page: 0,
            profile,
            followers
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
            leaders
        })))

        //log::debug!("followers: {:#?}", followers);
    } else {
        Err(Status::NoContent)
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

#[post("/api/user/create", format="json", data="<user>")]
pub async fn create_user(conn: Db, user: Result<Json<NewUser>, Error<'_>>)
                         -> Result<Json<Profile>, Status>
{
    debug!("raw\n{:#?}", user);

    if let Ok(user) = user {
        let keystore_value: serde_json::Value = serde_json::from_str(&user.keystore.clone()).unwrap();
        
        if let Some(profile) =    
            admin::create_user(&conn,
                               user.username.clone(),
                               user.display_name.clone(),
                               user.password.clone(), 
                               Some(user.client_public_key.clone()),
                               Some(keystore_value)).await {
                Ok(Json(profile))
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

#[post("/api/user/authenticate", format="json", data="<user>")]
pub async fn authenticate_user(conn: Db, user: Result<Json<AuthenticationData>, Error<'_>>)
                               -> Result<Json<Profile>, Status>
{
    debug!("raw\n{:#?}", user);

    if let Ok(user) = user {
        if let Some(profile) =
            admin::authenticate(&conn,
                               user.username.clone(),
                               user.password.clone()).await {
                Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/outbox", format="application/activity+json", data="<object>")]
pub async fn
    outbox_post(
        signed: Signed,
        conn: Db,
        username: String,
        object: Result<Json<ApBaseObjectSuper>, Error<'_>>)
    -> Result<Status, Status>
{
    debug!("raw\n{:#?}", object);

    if let Signed(true) = signed {
        match get_profile_by_username(&conn, username).await {
            Some(profile) => match object {
                Ok(object) => match object {
                    Json(ApBaseObjectSuper::Activity(activity)) => {
                        match activity.kind {                        
                            ApActivityType::Undo =>
                                outbox::activity::undo(conn, activity, profile).await,
                            ApActivityType::Follow =>
                                outbox::activity::follow(conn, activity, profile).await,
                            _ => Err(Status::NoContent)
                        }
                    },
                    Json(ApBaseObjectSuper::Object(ApObject::Note(note))) => 
                        outbox::object::note(conn, note, profile).await,
                    Json(ApBaseObjectSuper::Object(ApObject::Session(session))) => {
                        outbox::object::session(conn, session, profile).await
                    },
                    _ => Err(Status::NoContent)
                },
                Err(_) => Err(Status::NoContent)
            },
            None => Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/user/<username>/inbox", format="application/activity+json", data="<activity>")]
pub async fn
    inbox_post(signed: Signed,
               conn: Db,
               faktory: FaktoryConnection,
               username: String,
               activity: Result<Json<ApActivity>, Error<'_>>)
               -> Result<Status, Status> {
    
    debug!("inbox: {:#?}", activity);

    // if let (Ok(activity), Some(profile)) =
    //     (activity, get_profile_by_username(&conn, username).await)
    if let (Ok(activity), Some(profile), Signed(true)) =
        (activity, get_profile_by_username(&conn, username).await, signed)
    {                          
        let activity = activity.0.clone();
        
        if retriever::get_actor(&conn,
                                profile.clone(),
                                activity.actor.clone()).await.is_some()
        {
            let mut n = NewRemoteActivity::from(activity.clone());
            n.profile_id = profile.id;
            
            if let Some(created_activity) = create_remote_activity(&conn, n).await {
                log::debug!("created_remote_activity\n{:#?}", created_activity);
            } else {
                log::debug!("create_remote_activity failed");
            }

            match activity.kind {
                ApActivityType::Delete => inbox::activity::delete(conn, activity).await,
                ApActivityType::Create => inbox::activity::create(conn, activity, profile).await,
                ApActivityType::Follow =>
                    inbox::activity::follow(conn, faktory, activity, profile).await,
                ApActivityType::Undo => inbox::activity::undo(conn, activity).await,
                ApActivityType::Accept => inbox::activity::accept(conn, activity).await,
                ApActivityType::Invite =>
                    inbox::activity::invite(conn, faktory, activity, profile).await,
                ApActivityType::Join => inbox::activity::join(conn, activity, profile).await,
                _ => Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        log::debug!("request was unsigned or malformed in some way");
        Err(Status::NoContent)
    }
}

// TODO: Remove all of this CORS stuff for production; this is just to allow for testing on a single machine
// using different ports for services (i.e., a production server would expose the HTTP endpoints through a
// a common server name:port
/// Catches all OPTION requests in order to get the CORS related Fairing triggered.
#[options("/<_..>")]
fn all_options() {
    /* Intentionally left empty */
}

pub struct Cors;

#[rocket::async_trait]
impl Fairing for Cors {
    fn info(&self) -> Info {
        Info {
            name: "Cross-Origin-Resource-Sharing Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, PATCH, PUT, DELETE, HEAD, OPTIONS, GET",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[launch]
fn rocket() -> _ {
    env_logger::init();
    
    rocket::build()
        .attach(FaktoryConnection::fairing())
        .attach(Db::fairing())
        .attach(Cors)
        .mount("/", routes![
            person,
            profile,
            webfinger,
            outbox_post,
            inbox_post,
            get_followers,
            get_leaders,
            create_user,
            authenticate_user,
            test,
            all_options
        ])
}
