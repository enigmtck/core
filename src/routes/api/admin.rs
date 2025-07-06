use crate::{
    admin::{self, NewUser},
    db::Db,
    fairings::signatures::Signed,
    models::actors::{
        get_actor_by_as_id, get_muted_terms_by_username, guaranteed_actor,
        update_muted_terms_by_username, Actor,
    },
    retriever::get_actor,
};
use jdt_activity_pub::MaybeMultiple;
use jdt_activity_pub::{ApActor, ApContext, ApFollow};
use rocket::{
    get,
    http::Status,
    post,
    request::{FromRequest, Outcome, Request},
    serde::json::Error,
    serde::json::Json,
};
use serde::Deserialize;
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

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MutedTermsActionType {
    Add,
    Remove,
}

#[derive(Deserialize)]
pub struct MutedTermsAction {
    pub action: MutedTermsActionType,
    pub terms: Vec<String>,
}

#[post("/api/user/create", format = "json", data = "<user>")]
pub async fn create_user(
    conn: Db,
    user: Result<Json<NewUser>, Error<'_>>,
) -> Result<Json<Actor>, Status> {
    if !*crate::REGISTRATION_ENABLED {
        return Err(Status::ServiceUnavailable);
    }

    if let Ok(Json(user)) = user {
        log::debug!("CREATING USER\n{user:#?}");

        if let Ok(profile) = admin::create_user(&conn, user).await {
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

#[get("/api/user/<username>/muted-terms")]
pub async fn get_muted_terms(
    conn: Db,
    username: String,
    signed: Signed,
) -> Result<Json<Vec<String>>, Status> {
    // Check if request is locally signed
    if signed.profile().is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = signed.profile().unwrap();

    // Check if the requesting user is retrieving their own muted terms
    if profile.ek_username != Some(username.clone()) {
        return Err(Status::Forbidden);
    }

    get_muted_terms_by_username(&conn, username)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[post("/api/user/<username>/muted-terms", format = "json", data = "<action>")]
pub async fn manage_muted_terms(
    conn: Db,
    username: String,
    signed: Signed,
    action: Result<Json<MutedTermsAction>, Error<'_>>,
) -> Result<Status, Status> {
    // Check if request is locally signed
    if signed.profile().is_none() {
        return Err(Status::Unauthorized);
    }

    let profile = signed.profile().unwrap();

    // Check if the requesting user is updating their own muted terms
    if profile.ek_username != Some(username.clone()) {
        return Err(Status::Forbidden);
    }

    if let Ok(Json(action)) = action {
        // Get current muted terms
        let mut all_terms = get_muted_terms_by_username(&conn, username.clone())
            .await
            .unwrap_or_default();

        // Process based on action
        match action.action {
            MutedTermsActionType::Add => {
                // Add new terms, avoiding duplicates
                for term in action.terms {
                    if !all_terms.contains(&term) {
                        all_terms.push(term);
                    }
                }
            }
            MutedTermsActionType::Remove => {
                // Remove specified terms
                all_terms.retain(|term| !action.terms.contains(term));
            }
        }

        // Update the muted terms
        update_muted_terms_by_username(&conn, username, all_terms)
            .await
            .map(|_| Status::Ok)
            .map_err(|_| Status::InternalServerError)
    } else {
        Err(Status::BadRequest)
    }
}
