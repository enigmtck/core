use crate::{
    admin::{self, NewUser},
    db::Db,
    models::actors::{get_actor_by_as_id, guaranteed_actor, Actor},
    retriever::get_actor,
};
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::{ApActor, ApContext, ApFollow};
use rocket::{
    http::Status,
    post,
    request::{FromRequest, Outcome, Request},
    serde::json::Error,
    serde::json::Json,
};
use std::net::IpAddr;

pub struct IpRestriction;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IpRestriction {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let ip = request.remote().map(|addr| addr.ip());

        match ip {
            Some(ip) => {
                // Define your allowed IP ranges here
                if is_allowed_ip(ip) {
                    Outcome::Success(IpRestriction)
                } else {
                    Outcome::Error((Status::Forbidden, ()))
                }
            }
            None => Outcome::Error((Status::Forbidden, ())),
        }
    }
}

fn is_allowed_ip(ip: IpAddr) -> bool {
    ip.is_loopback()
}

#[post("/api/user/create", format = "json", data = "<user>")]
pub async fn create_user(
    conn: Db,
    user: Result<Json<NewUser>, Error<'_>>,
) -> Result<Json<Actor>, Status> {
    if let Ok(Json(user)) = user {
        log::debug!("CREATING USER\n{user:#?}");

        if let Ok(profile) = admin::create_user(Some(&conn), user).await {
            Ok(Json(profile))
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
}

#[post("/api/system/relay", data = "<actor>")]
pub async fn relay_post(_ip: IpRestriction, conn: Db, actor: String) -> Result<Status, Status> {
    let profile = guaranteed_actor(&conn, None).await;

    let actor = if let Ok(actor) = get_actor_by_as_id(&conn, actor.clone()).await {
        Some(ApActor::from(actor))
    } else {
        (get_actor(&conn, actor, None, true).await).ok()
    };

    let inbox = if let Some(actor) = actor.clone() {
        if let Some(endpoints) = actor.endpoints {
            Some(endpoints.shared_inbox)
        } else {
            Some(actor.inbox)
        }
    } else {
        None
    };

    if let (Some(_inbox), Some(actor)) = (inbox, actor) {
        let _follow = ApFollow {
            context: Some(ApContext::activity_streams()),
            actor: profile.as_id.into(),
            to: MaybeMultiple::Single(actor.id.unwrap_or_default()),
            ..Default::default()
        };
    }

    Ok(Status::Accepted)
}
