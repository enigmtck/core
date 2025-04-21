use crate::{
    db::Db,
    fairings::signatures::Signed,
    models::{
        activities::lookup_activity_id_by_as_id,
        actors::{
            get_actor_by_username, set_mls_credentials_by_username, update_mls_storage_by_username,
            Actor,
        },
        followers::{get_follower_count_by_actor_id, get_followers_by_actor_id},
        leaders::{get_leader_count_by_actor_id, get_leaders_by_actor_id},
        mls_group_conversations::create_mls_group_conversation,
        mls_key_packages::create_mls_key_package,
        unprocessable::create_unprocessable,
        vault::{create_vault_item, VaultItemParams},
        OffsetPaging,
    },
    LoadEphemeral,
};
use jdt_activity_pub::{
    ActivityPub, ApActor, ApCollection, ApInstrument, ApInstrumentType, ApObject, Collectible,
    FollowersPage, LeadersPage, MaybeReference,
};
use rocket::{get, http::Status, response::Redirect, serde::json::Json};
use serde_json::Value;

use super::{ActivityJson, LdJson};

async fn process_collection_items(
    conn: &Db,
    profile: &Actor,
    collection: ApCollection,
) -> Result<(), Status> {
    let items = collection.items().ok_or(Status::UnprocessableEntity)?;

    for item in items {
        if let ActivityPub::Object(ApObject::Instrument(instrument)) = item {
            log::debug!("Updating Instrument: {instrument:#?}");
            process_instrument(conn, profile, &instrument).await?;
        }
    }
    Ok(())
}

pub async fn process_instrument(
    conn: &Db,
    profile: &Actor,
    instrument: &ApInstrument,
) -> Result<(), Status> {
    let username = profile
        .ek_username
        .clone()
        .ok_or(Status::InternalServerError)?;

    match instrument.kind {
        ApInstrumentType::MlsGroupId => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsGroupId content must be Some");
                Status::UnprocessableEntity
            })?;

            let conversation = instrument.clone().conversation.ok_or_else(|| {
                log::error!("MlsGroupId conversation cannot be None");
                Status::UnprocessableEntity
            })?;

            create_mls_group_conversation(conn, (profile.id, content, conversation).into())
                .await
                .map_err(|e| {
                    log::error!("Failed to create or update GroupId: {e:#?}");
                    Status::InternalServerError
                })?;
        }
        ApInstrumentType::MlsCredentials => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsCredentials content must be Some");
                Status::UnprocessableEntity
            })?;
            set_mls_credentials_by_username(conn, username, content)
                .await
                .map_err(|e| {
                    log::debug!("Failed to set Credentials: {e:#?}");
                    Status::InternalServerError
                })?;
        }
        ApInstrumentType::MlsStorage => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsStorage content must be Some");
                Status::UnprocessableEntity
            })?;
            let hash = instrument.hash.clone().ok_or_else(|| {
                log::debug!("MlsStorage hash must be Some");
                Status::UnprocessableEntity
            })?;
            update_mls_storage_by_username(
                conn,
                username,
                content,
                hash,
                instrument.mutation_of.clone(),
            )
            .await
            .map_err(|e| {
                log::debug!("Failed to set Storage: {e:#?}");
                Status::InternalServerError
            })?;
        }
        ApInstrumentType::MlsKeyPackage => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsKeyPackage content must be Some");
                Status::UnprocessableEntity
            })?;
            create_mls_key_package(conn, (profile.id, content).into())
                .await
                .map_err(|e| {
                    log::debug!("Failed to create KeyPackage: {e:#?}");
                    Status::InternalServerError
                })?;
        }
        ApInstrumentType::VaultItem => {
            let activity_id = lookup_activity_id_by_as_id(
                conn,
                instrument.activity.clone().ok_or_else(|| {
                    log::error!("VaultItem Instrument must have an Activity");
                    Status::UnprocessableEntity
                })?,
            )
            .await
            .map_err(|e| {
                log::error!("Failed to lookup activity_id: {e:#?}");
                Status::InternalServerError
            })?;

            log::debug!("Activity ID: {activity_id}");

            let result = create_vault_item(
                conn,
                VaultItemParams {
                    instrument: instrument.clone(),
                    owner: profile.clone(),
                    activity_id,
                }
                .try_into()
                .map_err(|e| {
                    log::error!("Failed to build NewVaultItem: {e:#?}");
                    Status::UnprocessableEntity
                })?,
            )
            .await
            .map_err(|e| {
                log::error!("Failed to create VaultItem: {e:#?}");
                Status::InternalServerError
            });

            log::debug!("VaultItem insert result: {result:#?}");
        }
        _ => (),
    }
    Ok(())
}

#[post(
    "/user/<username>",
    format = "application/activity+json",
    data = "<raw>"
)]
pub async fn person_post(
    signed: Signed,
    conn: Db,
    username: String,
    raw: Json<Value>,
) -> Result<ActivityJson<ApObject>, Status> {
    let actor = signed.profile().ok_or(Status::Unauthorized)?;
    if username.as_str() != actor.ek_username.as_deref().ok_or(Status::Unauthorized)? {
        return Err(Status::Unauthorized);
    }

    log::debug!("POSTING TO ACTOR\n{raw:#?}");
    let raw = raw.into_inner();

    if let Ok(ApObject::Collection(collection)) = serde_json::from_value::<ApObject>(raw.clone()) {
        process_collection_items(&conn, &actor, collection).await?;

        Ok(ActivityJson(Json(ApObject::Actor(ApActor::from(actor)))))
    } else {
        create_unprocessable(&conn, raw.into()).await;
        Err(Status::UnprocessableEntity)
    }
}

#[get("/user/<username>", format = "text/html", rank = 1)]
pub async fn person_redirect(username: String) -> Redirect {
    log::debug!("REDIRECTING {username}");
    Redirect::to(format!("/@{username}"))
}

#[get("/user/<username>", format = "application/activity+json", rank = 2)]
pub async fn person_activity_json(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApActor>, Status> {
    match get_actor_by_username(&conn, username).await {
        Some(profile) => {
            let actor = if signed.local() {
                ApActor::from(profile)
                    .load_ephemeral(&conn, signed.profile())
                    .await
            } else {
                ApActor::from(profile)
            };

            Ok(ActivityJson(Json(actor)))
        }
        None => Err(Status::NotFound),
    }
}

#[get("/user/<username>", format = "application/ld+json", rank = 3)]
pub async fn person_ld_json(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<LdJson<ApActor>, Status> {
    match get_actor_by_username(&conn, username).await {
        Some(profile) => {
            let actor = if signed.local() {
                ApActor::from(profile)
                    .load_ephemeral(&conn, signed.profile())
                    .await
            } else {
                ApActor::from(profile)
            };

            Ok(LdJson(Json(actor)))
        }
        None => Err(Status::NotFound),
    }
}

#[get("/user/<username>/liked")]
pub async fn liked_get(conn: Db, username: String) -> Result<ActivityJson<ApCollection>, Status> {
    // I should make this real at some point.
    if let Some(_profile) = get_actor_by_username(&conn, username).await {
        Ok(ActivityJson(Json(ApCollection::default())))
    } else {
        Err(Status::NotFound)
    }
}

#[get("/user/<username>/followers?<page>")]
pub async fn get_followers(
    _signed: Signed,
    conn: Db,
    username: String,
    page: Option<u32>,
) -> Result<ActivityJson<ApCollection>, Status> {
    let profile = get_actor_by_username(&conn, username)
        .await
        .ok_or(Status::NotFound)?;

    let total_items = get_follower_count_by_actor_id(&conn, profile.id)
        .await
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE FOLLOWER COUNT: {e:#?}");
            Status::InternalServerError
        })?;

    let results = match page {
        Some(p) if p > 0 => {
            get_followers_by_actor_id(
                &conn,
                profile.id,
                Some(OffsetPaging {
                    page: p - 1,
                    limit: 20,
                }),
            )
            .await
        }
        _ => vec![],
    };

    let followers = results
        .iter()
        .map(|(follower, _)| {
            ActivityPub::try_from(MaybeReference::<ApActor>::Reference(follower.clone().actor))
                .unwrap()
        })
        .collect();

    let actors = Some(
        results
            .iter()
            .map(|(_, actor)| actor.clone())
            .collect::<Vec<_>>(),
    );

    Ok(ActivityJson(Json(
        ApCollection::try_from(FollowersPage {
            page,
            username: profile.ek_username.ok_or_else(|| {
                log::error!("Profile must have a Username");
                Status::InternalServerError
            })?,
            total_items,
            followers,
            actors,
        })
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADERS: {e:#?}");
            Status::InternalServerError
        })?,
    )))
}

#[get("/user/<username>/following?<page>")]
pub async fn get_leaders(
    _signed: Signed,
    conn: Db,
    username: String,
    page: Option<u32>, // page starts at 1; must be adjusted to 0 for query
) -> Result<ActivityJson<ApCollection>, Status> {
    let profile = get_actor_by_username(&conn, username)
        .await
        .ok_or(Status::NotFound)?;

    let total_items = get_leader_count_by_actor_id(&conn, profile.id)
        .await
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADER COUNT: {e:#?}");
            Status::InternalServerError
        })?;

    let results = match page {
        Some(p) if p > 0 => {
            get_leaders_by_actor_id(
                &conn,
                profile.id,
                Some(OffsetPaging {
                    page: p - 1,
                    limit: 20,
                }),
            )
            .await
        }
        _ => vec![],
    };

    let leaders = results
        .iter()
        .map(|(leader, _)| {
            ActivityPub::try_from(MaybeReference::<ApActor>::Reference(
                leader.clone().leader_ap_id,
            ))
            .unwrap()
        })
        .collect();

    let actors = Some(
        results
            .iter()
            .filter_map(|(_, actor)| actor.clone())
            .collect::<Vec<_>>(),
    );

    Ok(ActivityJson(Json(
        ApCollection::try_from(LeadersPage {
            page,
            username: profile.ek_username.ok_or_else(|| {
                log::error!("Profile must have a Username");
                Status::InternalServerError
            })?,
            total_items,
            leaders,
            actors,
        })
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADERS: {e:#?}");
            Status::InternalServerError
        })?,
    )))
}
