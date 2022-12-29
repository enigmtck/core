#[macro_use]
extern crate rocket;

use enigmatick::activity_pub::{ApActivityType, ApOrderedCollection, FollowersPage, ApBaseObjectSuper};
use enigmatick::models::notes::{NewNote, Note};
use enigmatick::models::remote_notes::NewRemoteNote;
use enigmatick::models::remote_activities::NewRemoteActivity;
use enigmatick::models::followers::NewFollower;
use enigmatick::models::leaders::NewLeader;
use enigmatick::activity_pub::{ApObject, ApActor, ApNote, ApActivity, retriever, sender};
use enigmatick::webfinger::WebFinger;
use enigmatick::db::{Db,
                     get_profile_by_username,
                     create_note,
                     create_remote_activity,
                     create_remote_note,
                     create_follower,
                     delete_follower_by_ap_id,
                     create_leader,
                     update_leader_by_uuid,
                     get_remote_activity_by_ap_id,
                     get_followers_by_profile_id,
                     get_remote_actor_by_ap_id};
use enigmatick::signing::{sign, verify, VerifyParams, SignParams, Method};

use faktory::{Producer, Job};

use rocket::http::RawStr;
use rocket::request::{FromParam, FromRequest, Request, Outcome};
use rocket::serde::json::Json;
use rocket::serde::json::Error;
use rocket::http::Status;
// use rocket::fairing::{Fairing, Info, Kind, self};
// use rocket::{Rocket, Build, State};
use rocket::State;

use reqwest::Client;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub struct FaktoryConnection {
    pub producer: Arc<Mutex<Producer<TcpStream>>>
}

// impl FaktoryConnection {
//     pub fn fairing() -> impl Fairing {
//         FaktoryConnectionFairing
//     }
// }
// struct FaktoryConnectionFairing;

// #[rocket::async_trait]
// impl Fairing for FaktoryConnectionFairing {
//     fn info(&self) -> Info {
//         Info {
//             name: "Faktory Connection",
//             kind: Kind::Ignite | Kind::Liftoff | Kind::Response
//         }
//     }

//     async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
//         log::info!("connecting to faktory");
        
//         Ok(rocket.manage(FaktoryConnection {
//             producer: Producer::connect(Some("tcp://:password@localhost:7419")).unwrap()
//         }))
//     }
// }

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
pub async fn test(conn: Db, faktory: &State<FaktoryConnection>, username: String)
                  -> Result<Json<ApActor>, Status> {    
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

// 2022-12-29T01:26:54Z DEBUG server] created_remote_activity
//     RemoteActivity {
//         id: 501,
//         created_at: 2022-12-29T01:26:54.146936Z,
//         updated_at: 2022-12-29T01:26:54.146936Z,
//         profile_id: 1,
//         context: Some(
//             String("https://www.w3.org/ns/activitystreams"),
//         ),
//         kind: "Follow",
//         ap_id: "https://ser.endipito.us/4bf6fc9f-c63a-49a4-85f1-48deedc17a62",
//         ap_to: Some(
//             Array [],
//         ),
//         cc: Some(
//             Array [],
//         ),
//         actor: "https://ser.endipito.us/users/lloyd",
//         published: None,
//         ap_object: Some(
//             String("https://enigmatick.jdt.dev/user/justin"),
//         ),
//     }
// [2022-12-29T01:26:54Z DEBUG server] created_follower
//     Follower {
//         id: 17,
//         created_at: 2022-12-29T01:26:54.155454Z,
//         updated_at: 2022-12-29T01:26:54.155454Z,
//         profile_id: 1,
//         ap_id: "https://ser.endipito.us/4bf6fc9f-c63a-49a4-85f1-48deedc17a62",
//         actor: "https://ser.endipito.us/users/lloyd",
//         followed_ap_id: "https://enigmatick.jdt.dev/user/justin",
//         uuid: "1e8e2500-c564-4064-a9c3-fda05058a430",
//     }

// ApActivity {
//     base: ApBaseObject {
//         context: Some(
//             Plain(
//                 "https://www.w3.org/ns/activitystreams",
//             ),
//         ),
//         to: None,
//         cc: None,
//         bcc: None,
//         tag: None,
//         attachment: None,
//         attributed_to: None,
//         audience: None,
//         content: None,
//         name: None,
//         end_time: None,
//         generator: None,
//         icon: None,
//         in_reply_to: None,
//         location: None,
//         preview: None,
//         published: None,
//         replies: None,
//         start_time: None,
//         summary: None,
//         updated: None,
//         url: None,
//         bto: None,
//         media_type: None,
//         duration: None,
//         kind: None,
//         id: Some(
//             "https://ser.endipito.us/users/justin#accepts/follows/846",
//         ),
//         uuid: None,
//     },
//     kind: Accept,
//     actor: "https://ser.endipito.us/users/justin",
//     object: Identifier(
//         ApIdentifier {
//             id: "https://enigmatick.jdt.dev/leader/f208be33-55c9-44f7-8533-d36da8a8d4cf",
//         },
//     ),
// },

#[post("/user/<username>/outbox", format="json", data="<object>")]
pub async fn
    outbox_post(conn: Db,
                username: String,
                object: Result<Json<ApBaseObjectSuper>, Error<'_>>)
                -> Result<Json<Note>, Status>
{
    debug!("raw\n{:#?}", object);

    match get_profile_by_username(&conn, username).await {
        Some(profile) => match object {
            Ok(object) => match object {
                Json(ApBaseObjectSuper::Activity(mut activity)) => {
                    debug!("this looks like an ApActivity\n{:#?}", activity);
                    match activity.kind {
                        ApActivityType::Follow => {
                            debug!("this looks like a Follow activity");
                            
                            activity.actor = format!("https://enigmatick.jdt.dev/user/{}", profile.username);
                            
                            let mut leader = NewLeader::from(activity.clone());
                            leader.profile_id = profile.id;

                            if let Some(leader) = create_leader(&conn, leader).await {
                                debug!("leader created: {}", leader.uuid);
                                activity.base.id = Option::from(format!("https://enigmatick.jdt.dev/leader/{}",
                                                                        leader.uuid));
                                
                                debug!("updated activity\n{:#?}", activity);

                                if let ApObject::Plain(object) = activity.clone().object {
                                    if let Some(actor) = get_remote_actor_by_ap_id(&conn, object).await {
                                        if sender::send_follow(activity, profile, actor).await.is_ok() {
                                            debug!("sent follow request successfully");
                                        }
                                    }
                                }
                            }
                        },
                        _ => debug!("looks like something else")
                    }

                    Err(Status::NoContent)
                },
                Json(ApBaseObjectSuper::Object(ApObject::Note(note))) => {
                    debug!("this looks like an ApNote");
                    
                    let create = ApActivity::from(note.clone());

                    let n = NewNote { uuid: create.clone().base.uuid.unwrap(),
                                      profile_id: profile.id,
                                      content: note.content,
                                      ap_to: note.to.clone().into(),
                                      ap_tag: Option::from(serde_json::to_value(&note.base.tag).unwrap()) };
                    
                    if let Some(created_note) = create_note(&conn, n).await {
                        for recipient in note.to {                            
                            let profile = profile.clone();
                            if let Some(receiver) = retriever::get_actor(&conn,
                                                                         profile.clone(),
                                                                         recipient.clone()).await {
                                let url = receiver.inbox;

                                let body = Option::from(serde_json::to_string(&create).unwrap());
                                let method = Method::Post;
                                
                                let signature = sign(
                                    SignParams { profile,
                                                 url: url.clone(),
                                                 body: body.clone(),
                                                 method }
                                );

                                let client = Client::new().post(&url)
                                    .header("Date", signature.date)
                                    .header("Digest", signature.digest.unwrap())
                                    .header("Signature", &signature.signature)
                                    .header("Content-Type",
                                            "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"")
                                    .body(body.unwrap());
                                
                                if let Ok(resp) = client.send().await {
                                    if let Ok(text) = resp.text().await {
                                        debug!("send successful to: {}\n{}", recipient, text);
                                    }
                                }
                            }
                        }
                        Ok(Json(created_note))
                    } else {
                        Err(Status::NoContent)
                    }
                },
                _ => Err(Status::NoContent)
            },
            Err(_) => Err(Status::NoContent)
        },
        None => Err(Status::NoContent)
    }
}

#[post("/user/<username>/inbox", format="application/activity+json", data="<activity>")]
pub async fn
    inbox_post(signed: Signed,
               conn: Db,
               faktory: &State<FaktoryConnection>,
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

                        match faktory.producer.try_lock() {
                            Ok(mut x) => {
                                if x.enqueue(Job::new("acknowledge_followers", vec![created_follower.uuid]))
                                    .is_err() {
                                    log::error!("failed to enqueue job");
                                }
                            },
                            Err(e) => log::debug!("failed to lock mutex: {}", e)
                        }
                        
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
                ApActivityType::Accept => {
                    if let ApObject::Identifier(x) = activity.object {
                        let ap_id_re = regex::Regex::new(r#"(\w+://)(.+?/)+(.+)"#).unwrap();
                        if let Some(ap_id_match) = ap_id_re.captures(&x.id) {
                            debug!("ap_id_match: {:#?}", ap_id_match);

                            let matches = ap_id_match.len();
                            let uuid = ap_id_match.get(matches-1).unwrap().as_str();

                            if let Some(id) = activity.base.id {
                                if update_leader_by_uuid(&conn, uuid.to_string(), id).await.is_some() {
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
        .manage(FaktoryConnection {
            producer: Arc::new(
                Mutex::new(
                    Producer::connect(
                        Some("tcp://:password@localhost:7419")).unwrap()
                )
            )
        })
        .attach(Db::fairing())
        .mount("/", routes![
            person,
            profile,
            webfinger,
            outbox_post,
            inbox_post,
            get_followers,
            test
        ])
}
