#[macro_use]
extern crate rocket;

use rocket::http::RawStr;
use rocket::request::FromParam;
use rocket::serde::json::Json;
use rocket::serde::json::Error;
use rocket::http::Status;

use reqwest::Client;
use serde::Serialize;

use enigmatick::activity_pub::{Actor, Note, Create, retriever};
use enigmatick::webfinger::WebFinger;
use enigmatick::db::{Db, get_profile_by_username};
use enigmatick::signing::sign;
// #[derive(Default)]
// pub struct Username<'r> {
//     id: &'r str,
//     key: bool,
// }

// impl <'r> FromParam<'r> for Username<'r> {
//     type Error = &'r RawStr;

//     fn from_param(param: &'r str) -> Result<Self, Self::Error> {
//         debug!("param: {}", param);
//         if param.ends_with("#main-key") {
//             let parts: Vec<&str> = param.split('#').collect::<Vec<&str>>();
//             Ok(Username { id: parts[0], key: true })
//         } else {
//             Ok(Username { id: param, key: false})
//         }
//     }
// }
    
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
pub async fn profile(conn: Db, handle: Handle<'_>) -> Result<Json<Actor>, Status> {
    let mut username = handle.name.to_string();
    username.remove(0);
    match get_profile_by_username(&conn, username).await {
        Some(profile) => Ok(Json(Actor::from(profile))),
        None => Err(Status::NoContent)
    }
}

#[get("/user/<username>")]
pub async fn person(conn: Db, username: String) -> Result<Json<Actor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            let a = Actor::from(profile.clone());
            // let n = Note::from(a.clone()).to("test".to_string()).content("testing".to_string());
            // let c = Create::from(n);
            // log::debug!("{}", serde_json::to_string(&c).unwrap());

            let signature = sign(profile, "target".to_string(), "host".to_string(), "date".to_string());
            debug!("Signature: {}", signature);

            Ok(Json(a))
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

#[get("/remote/<username>")]
pub async fn remote(conn: Db, username: String) -> Result<Json<Actor>, Status> {
    match get_profile_by_username(&conn, username).await {
        Some(profile) => {
            let p = retriever::get_actor(profile,
                                         "https://mastodon.social/users/Gargron".to_string()).await;

            Ok(Json(p))
        },
        None => Err(Status::NoContent)
    }
}

// #[get("/user/<key_id>", rank=1)]
// pub async fn public_key(conn: Db, key_id: KeyId<'_>) -> Result<String, Status> {
//     match get_profile_by_username(&conn, key_id.id.to_string()).await {
//         Some(profile) => {
//             Ok(profile.public_key)
//         },
//         None => Err(Status::NoContent)
//     }
// }

#[post("/user/<username>/outbox", format="json", data="<note>")]
pub async fn outbox_post(conn: Db, username: String, note: Result<Json<Note>, Error<'_>>) -> Result<Json<Create>, Status> {
    match note {
        Ok(note) => {
            match get_profile_by_username(&conn, username).await {
                Some(profile) => {
                    let actor = Actor::from(profile);
                    let create = Create::from(note.0);

                    Ok(Json(create))
                },
                None => Err(Status::NoContent)
            }
        },
        Err(e) => {
            log::debug!("error: {:#?}", e);
            Err(Status::NoContent)
        }
    }
}

#[post("/user/<username>/inbox", format="json", data="<create>")]
pub async fn inbox_post(conn: Db, username: String, create: Result<Json<Create>, Error<'_>>) -> Result<Status, Status> {
    match create {
        Ok(create) => {
            match get_profile_by_username(&conn, username).await {
                Some(profile) => {
                    let actor = Actor::from(profile);

                    Ok(Status::Accepted)
                },
                None => Err(Status::NoContent)
            }
        },
        Err(e) => {
            log::debug!("error: {:#?}", e);
            Err(Status::NoContent)
        }
    }
}

#[launch]
fn rocket() -> _ {
    env_logger::init();
    
    rocket::build()
        .attach(Db::fairing())
        .mount("/", routes![person, profile, remote, webfinger, outbox_post, inbox_post])
}
