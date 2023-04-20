use crate::activity_pub::retriever::{self, get_note, get_remote_webfinger};
use crate::activity_pub::{ApActor, ApNote};
use crate::db::Db;
use crate::helper::{get_local_identifier, is_local, LocalIdentifierType};
use rocket::http::Status;
use rocket::{post, serde::json::Error, serde::json::Json};
use serde::Deserialize;

use crate::fairings::signatures::Signed;
use crate::models::profiles::get_profile_by_username;
use crate::signing::VerificationType;

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
                        if let Some(x) = get_local_identifier(ap_id.clone()) {
                            if x.kind == LocalIdentifierType::User {
                                Ok(Json(
                                    get_profile_by_username(&conn, x.identifier)
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
                    } else if let Some(actor) =
                        retriever::get_actor(&conn, ap_id, Some(profile), true).await
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
