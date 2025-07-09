use crate::{
    axum_server::{extractors::AxumSigned, AppState},
    helper::get_uuid,
    models::{
        activities::{
            get_encrypted_activities, lookup_activity_id_by_as_id, TryFromEncryptedActivity,
        },
        actors::{get_actor_by_username, update_olm_account_by_username},
        mls_key_packages::{get_mkp_count_by_profile_id, get_next_mkp_by_actor_id},
        olm_one_time_keys::{
            create_olm_one_time_key, get_next_otk_by_profile_id, get_otk_count_by_profile_id,
        },
        olm_sessions::{
            create_or_update_olm_session, get_olm_session_by_conversation_and_actor,
            OlmSessionParams,
        },
        vault::{create_vault_item, VaultItemParams},
    },
    routes::{api::encryption::OtkUpdateParams, ActivityJson},
};
use axum::{
    extract::{rejection::JsonRejection, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use jdt_activity_pub::{
    ActivityPub, ApActivity, ApCollection, ApInstrument, ApInstrumentType, ApObject, Collectible,
};
use serde::Deserialize;
use urlencoding::decode;

pub async fn encrypted_activities_get(
    State(state): State<AppState>,
    signed: AxumSigned,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let activities = get_encrypted_activities(
        &conn,
        profile.ek_last_decrypted_activity,
        50,
        profile.as_id.into(),
    )
    .await
    .map_err(|e| {
        log::error!("Failed to get Encrypted Activities: {e:#?}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .into_iter()
    .filter_map(|x| ApActivity::try_from_encrypted_activity(x).ok())
    .collect::<Vec<ActivityPub>>();

    let collection = ApCollection::from((activities, None));

    Ok(ActivityJson(collection.into()))
}

#[derive(Deserialize)]
pub struct OlmSessionQuery {
    conversation: String,
}

pub async fn olm_session_get(
    State(state): State<AppState>,
    Query(query): Query<OlmSessionQuery>,
    signed: AxumSigned,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let conversation = decode(&query.conversation).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;

    let session =
        get_olm_session_by_conversation_and_actor(&conn, conversation.to_string(), profile.id)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

    if session.owner_id != profile.id {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(ActivityJson(ApObject::Instrument(session.into())))
}

pub async fn olm_account_get(signed: AxumSigned) -> Result<ActivityJson<ApObject>, StatusCode> {
    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;

    let instruments: Vec<ApInstrument> = profile.into();
    let olm_account: ApInstrument = instruments
        .into_iter()
        .find(|instrument| instrument.kind == ApInstrumentType::OlmAccount)
        .ok_or_else(|| {
            log::error!("Failed to locate OlmAccount");
            StatusCode::NOT_FOUND
        })?;

    Ok(ActivityJson(olm_account.into()))
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
            ActivityPub::Object(ApObject::Instrument(instrument))
                if instrument.is_olm_session() =>
            {
                create_or_update_olm_session(
                    &conn,
                    OlmSessionParams {
                        uuid: instrument.clone().id.and_then(get_uuid),
                        instrument: instrument.clone(),
                        owner: profile.clone(),
                    }
                    .try_into()
                    .map_err(|e| {
                        log::error!("Failed to build NewOlmSession: {e:#?}");
                        StatusCode::UNPROCESSABLE_ENTITY
                    })?,
                    instrument.clone().mutation_of,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to create or update OlmSession: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            }
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

pub async fn add_one_time_keys(
    signed: AxumSigned,
    State(state): State<AppState>,
    Path(username): Path<String>,
    params: Result<Json<OtkUpdateParams>, JsonRejection>,
) -> Result<StatusCode, StatusCode> {
    log::debug!("Adding One-Time Keys\n{params:#?}");

    let profile = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Json(params) = params.map_err(|e| {
        log::error!("Failed to decode OtkUpdateParams: {e:#?}");
        StatusCode::UNPROCESSABLE_ENTITY
    })?;

    if profile
        .ek_olm_pickled_account_hash
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        == params.mutation_of
    {
        update_olm_account_by_username(
            &conn,
            username,
            params.account,
            params.account_hash,
            params.mutation_of,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to update Olm Account: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        for (key, otk) in params.keys {
            create_olm_one_time_key(&conn, (profile.id, key, otk).into()).await;
        }

        Ok(StatusCode::ACCEPTED)
    } else {
        log::error!("UNEXPECTED MUTATION");
        Err(StatusCode::NO_CONTENT)
    }
}

#[derive(Deserialize, Debug)]
pub struct KeysQuery {
    otk: Option<bool>,
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

    if query.otk.is_some() {
        // Logic from keys_get
        if signed.actor().is_some_and(|x| x.id.is_some()) && query.otk.is_some_and(|x| x) {
            let otk = get_next_otk_by_profile_id(
                &conn,
                signed.actor().unwrap().id.unwrap().to_string(),
                profile.id,
            )
            .await
            .map_err(|e| {
                log::error!("Failed to get OTK: {e:#?}");
                StatusCode::NOT_FOUND
            })?;

            let instruments: Vec<ApInstrument> = profile.clone().into();

            let idk: ApInstrument = instruments
                .into_iter()
                .find(|instrument| instrument.kind == ApInstrumentType::OlmIdentityKey)
                .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

            Ok(ActivityJson(
                ApCollection::from(vec![otk.into(), idk]).into(),
            ))
        } else {
            let count = get_otk_count_by_profile_id(&conn, profile.id)
                .await
                .map_err(|e| {
                    log::error!("Failed to retrieve OTK count: {e:#?}");
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
    } else {
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
}
