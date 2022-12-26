#[macro_use]
extern crate rocket;

use enigmatick::activity_pub::ApActivityType;
use enigmatick::models::notes::{NewNote, Note};
use enigmatick::models::remote_notes::NewRemoteNote;
use enigmatick::models::remote_activities::NewRemoteActivity;
use enigmatick::models::followers::NewFollower;
use rocket::http::RawStr;
use rocket::request::{FromParam, FromRequest, Request, Outcome};
use rocket::serde::json::Json;
use rocket::serde::json::Error;
use rocket::http::Status;

use reqwest::Client;
use url::Url;
use sha2::{Sha256, Digest};
use std::time::SystemTime;

use enigmatick::activity_pub::{ApObject, ApActor, ApNote, ApActivity, retriever};
use enigmatick::webfinger::WebFinger;
use enigmatick::db::{Db,
                     get_profile_by_username,
                     create_note,
                     create_remote_activity,
                     create_remote_note,
                     create_follower,
                     delete_follower_by_ap_id,
                     get_remote_activity_by_ap_id};
use enigmatick::signing::{sign, verify, VerifyParams, SignParams};

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

        //log::debug!("request: {:#?}", request);

        let username_re = regex::Regex::new(r"(/user/)([a-zA-Z0-9]+)(/.*)").unwrap();
        if let Some(username_match) = username_re.captures(&path) {
            if let Some(username) = username_match.get(2) {
                match get_profile_by_username(&conn, username.as_str().to_string()).await {
                    Some(profile) => {
                        
                        let request_target = format!("{} {}", method.to_lowercase(), path);

                        let mut date = String::new();
                        let date_vec: Vec<_> = request.headers().get("date").collect();
                        if date_vec.len() == 1 {
                            date = date_vec[0].to_string();
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
                            _ => Outcome::Failure((Status::BadRequest, SignatureError::MultipleSignatures)),
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
pub async fn test(conn: Db, username: String) -> Result<Json<ApActor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            retriever::get_followers(&conn,
                                     profile.clone(),
                                     "https://ser.endipito.us/users/justin".to_string()).await;
            
            Ok(Json(ApActor::from(profile)))
        },
        None => Err(Status::NoContent)
    }
}

#[get("/user/<username>")]
pub async fn person(conn: Db, username: String) -> Result<Json<ApActor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
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

#[post("/user/<username>/outbox", format="json", data="<note>")]
pub async fn
    outbox_post(conn: Db,
                username: String,
                note: Result<Json<ApNote>, Error<'_>>)
                -> Result<Json<Note>, Status>
{
    if let Ok(note) = note {
        match get_profile_by_username(&conn, username).await {
            Some(profile) => {
                
                let create = ApActivity::from(note.clone().0);

                let n = NewNote { uuid: create.clone().base.uuid.unwrap(),
                                  profile_id: profile.id,
                                  content: note.0.content,
                                  ap_to: note.0.to.clone().into(),
                                  ap_tag: Option::from(serde_json::to_value(&note.0.base.tag).unwrap()) };
                
                if let Some(created_note) = create_note(&conn, n).await {
                    for recipient in note.0.to {                            
                        let profile = profile.clone();
                        if let Some(receiver) = retriever::get_actor(&conn,
                                                                     profile.clone(),
                                                                     recipient).await {
                            let inbox = receiver.inbox;
                            
                            let u = Url::parse(&inbox).unwrap();
                            let host = u.host().unwrap().to_string();
                            let path = u.path().to_string();

                            let create_json = serde_json::to_string(&create).unwrap();
                            debug!("json: {}", create_json);
                            let mut hasher = Sha256::new();
                            hasher.update(create_json.as_bytes());
                            let hashed = base64::encode(hasher.finalize());
                            let digest = format!("sha-256={}", hashed);

                            let now = SystemTime::now();
                            let date = httpdate::fmt_http_date(now);
                            debug!("date: {}", date);

                            let request_target = format!("post {}", path);
                            let signature = sign(
                                SignParams { profile,
                                             request_target,
                                             host,
                                             date: date.clone(),
                                             digest: Option::from(digest.clone()) }
                            ).await;
                            
                            debug!("signature: {}", signature);

                            let client = Client::new();
                            let c = client.post(&inbox)
                                .header("Date", date)
                                .header("Digest", digest)
                                .header("Signature", &signature)
                                .header("Content-Type", "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"")
                                .body(create_json);

                            debug!("wtf: {:#?}", c);
                            
                            let res = c.send()
                                .await
                                .unwrap()
                                .text()
                                .await
                                .unwrap();
                        }
                    }
                    Ok(Json(created_note))
                } else {
                    Err(Status::NoContent)
                }
            }
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
               username: String,
               activity: Result<Json<ApActivity>, Error<'_>>)
               -> Result<Status, Status> {
    
    debug!("inbox: {:#?}", activity);
    
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
                ApActivityType::Create => {
                    match activity.object {
                        ApObject::Note(x) => {
                            let mut n = NewRemoteNote::from(x);
                            n.profile_id = profile.id;

                            if let Some(created_note) = create_remote_note(&conn, n). await {
                                log::debug!("created_remote_note\n{:#?}", created_note);
                                Ok(Status::Accepted)
                            } else {
                                log::debug!("create_remote_note failed");
                                Err(Status::NoContent)
                            }
                        },
                        _ => Err(Status::NoContent)
                    }
                },
                ApActivityType::Follow => {
                    let mut f = NewFollower::from(activity);
                    f.profile_id = profile.id;

                    if let Some(created_follower) = create_follower(&conn, f). await {
                        log::debug!("created_follower\n{:#?}", created_follower);
                        Ok(Status::Accepted)
                    } else {
                        log::debug!("create_follower failed");
                        Err(Status::NoContent)
                    }
                },
                ApActivityType::Undo => {
                    if let ApObject::Identifier(x) = activity.object {
                        if let Some(x) = get_remote_activity_by_ap_id(&conn, x.id).await {
                            if x.kind == ApActivityType::Follow.to_string() &&
                                delete_follower_by_ap_id(&conn, x.ap_id).await {
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
                },
                _ => Ok(Status::Accepted)
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
        .attach(Db::fairing())
        .mount("/", routes![
            person,
            profile,
            webfinger,
            outbox_post,
            inbox_post,
            test
        ])
}
