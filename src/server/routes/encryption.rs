use std::collections::HashMap;

use crate::{
    models::{
        activities::lookup_activity_id_by_as_id,
        actors::{get_actor_by_username, update_olm_account_by_username},
        mls_key_packages::{get_mkp_count_by_profile_id, get_next_mkp_by_actor_id},
        vault::{create_vault_item, VaultItemParams},
    },
    server::{extractors::AxumSigned, AppState},
};
use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use jdt_activity_pub::{ActivityPub, ApCollection, ApInstrument, ApObject, Collectible};
use serde::{Deserialize, Serialize};

use super::ActivityJson;

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OtkUpdateParams {
    pub keys: HashMap<String, String>,
    pub account: String,
    pub mutation_of: String,
    pub account_hash: String,
}

pub async fn update_instruments(
    signed: AxumSigned,
    State(state): State<AppState>,
    collection: Result<Json<ApCollection>, JsonRejection>,
) -> Result<StatusCode, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Json(collection) = collection.map_err(|e| {
        log::error!("Failed to decode Instruments Collection: {e:#?}");
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    let instruments = collection.items().ok_or_else(|| {
        log::error!("No items in Collection");
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    for instrument in instruments {
        match instrument {
            ActivityPub::Object(ApObject::Instrument(instrument)) if instrument.is_vault_item() => {
                let activity_id = lookup_activity_id_by_as_id(
                    &conn,
                    instrument.activity.clone().ok_or_else(|| {
                        log::error!("VaultItem Instrument must have an Activity");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to lookup activity_id: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

                create_vault_item(
                    &conn,
                    VaultItemParams {
                        instrument: instrument.clone(),
                        owner: profile.clone(),
                        activity_id,
                    }
                    .try_into()
                    .map_err(|e| {
                        log::error!("Failed to build NewVaultItem: {e:#?}");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to create VaultItem: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            }
            ActivityPub::Object(ApObject::Instrument(instrument))
                if instrument.is_olm_account() =>
            {
                update_olm_account_by_username(
                    &conn,
                    profile.ek_username.clone().ok_or_else(|| {
                        log::error!("Account Instrument must have a username");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                    instrument.content.ok_or_else(|| {
                        log::error!("Account Instrument must have content");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                    instrument.hash.ok_or_else(|| {
                        log::error!("Account Instrument must have a hash");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                    instrument.mutation_of.ok_or_else(|| {
                        log::error!("Account Instrument must have a mutation_of field");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to update Account: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            }
            _ => (),
        }
    }

    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize, Debug)]
pub struct KeysQuery {
    mkp: Option<bool>,
    count: Option<bool>,
}

pub async fn keys(
    Query(query): Query<KeysQuery>,
    signed: AxumSigned,
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let profile = get_actor_by_username(&conn, username.clone())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Logic from keys_mkp_get
    if signed.actor().is_some_and(|x| x.id.is_some()) && query.mkp.is_some_and(|x| x) {
        let mkp = get_next_mkp_by_actor_id(
            &conn,
            signed.actor().unwrap().id.unwrap().to_string(),
            profile.id,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to get MKP: {e:#?}");
            StatusCode::NOT_FOUND
        })?;

        Ok(ActivityJson(ApCollection::from(vec![mkp.into()]).into()))
    } else if signed.profile().is_some()
        && (query.count.is_some_and(|x| !x) || query.count.is_none())
    {
        let actor = signed.profile().unwrap();
        let instruments: Vec<ApInstrument> = actor.into();
        Ok(ActivityJson(ApCollection::from(instruments).into()))
    } else {
        let count = get_mkp_count_by_profile_id(&conn, profile.id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve MKP count: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let collection = ApCollection {
            total_items: Some(count as i64),
            id: Some(format!(
                "https://{}/user/{username}/keys",
                *crate::SERVER_NAME
            )),
            ..Default::default()
        };

        Ok(ActivityJson(collection.into()))
    }
}
