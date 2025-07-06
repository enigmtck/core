use std::collections::HashMap;

use crate::{
    db::Db,
    fairings::signatures::Signed,
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
    routes::ActivityJson,
};
use jdt_activity_pub::{
    ActivityPub, ApActivity, ApCollection, ApInstrument, ApInstrumentType, ApObject, Collectible,
};
use rocket::{get, http::Status, post, serde::json::Error, serde::json::Json};
use serde::{Deserialize, Serialize};
use urlencoding::decode;

#[get("/api/encrypted", format = "application/activity+json")]
pub async fn encrypted_activities_get(
    signed: Signed,
    conn: Db,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = signed.profile().ok_or(Status::Unauthorized)?;

    Ok(ActivityJson(Json(
        ApCollection::from((
            get_encrypted_activities(
                &conn,
                profile.ek_last_decrypted_activity,
                50,
                profile.as_id.into(),
            )
            .await
            .map_err(|e| {
                log::error!("Failed to get Encrypted Activities: {e:#?}");
                Status::InternalServerError
            })?
            .into_iter()
            .filter_map(|x| ApActivity::try_from_encrypted_activity(x).ok())
            .collect::<Vec<ActivityPub>>(),
            None,
        ))
        .into(),
    )))
}

#[get(
    "/api/instruments/olm-session?<conversation>",
    format = "application/activity+json"
)]
pub async fn olm_session_get(
    signed: Signed,
    conn: Db,
    conversation: String,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = signed.profile().ok_or(Status::Unauthorized)?;
    let conversation = decode(&conversation).map_err(|_| Status::UnprocessableEntity)?;

    let session =
        get_olm_session_by_conversation_and_actor(&conn, conversation.to_string(), profile.id)
            .await
            .map_err(|_| Status::NotFound)?;

    if session.owner_id != profile.id {
        return Err(Status::Unauthorized);
    }

    Ok(ActivityJson(Json(ApObject::Instrument(session.into()))))
}

#[get("/api/instruments/olm-account", format = "application/activity+json")]
pub async fn olm_account_get(signed: Signed) -> Result<ActivityJson<ApObject>, Status> {
    let profile = signed.profile().ok_or(Status::Unauthorized)?;

    let instruments: Vec<ApInstrument> = profile.into();
    let olm_account: ApInstrument = instruments
        .into_iter()
        .find(|instrument| instrument.kind == ApInstrumentType::OlmAccount)
        .ok_or_else(|| {
            log::error!("Failed to locate OlmAccount");
            Status::NotFound
        })?;

    Ok(ActivityJson(Json(olm_account.into())))
}

#[post(
    "/api/instruments",
    format = "application/activity+json",
    data = "<collection>"
)]
pub async fn update_instruments(
    signed: Signed,
    conn: Db,
    collection: Result<Json<ApCollection>, Error<'_>>,
) -> Result<Status, Status> {
    let profile = signed.profile().ok_or(Status::Unauthorized)?;

    let Json(collection) = collection.map_err(|e| {
        log::error!("Failed to decode Instruments Collection: {e:#?}");
        Status::UnprocessableEntity
    })?;

    let instruments = collection.items().ok_or_else(|| {
        log::error!("No items in Collection");
        Status::UnprocessableEntity
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
                        Status::UnprocessableEntity
                    })?,
                    instrument.clone().mutation_of,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to create or update OlmSession: {e:#?}");
                    Status::InternalServerError
                })?;
            }
            ActivityPub::Object(ApObject::Instrument(instrument)) if instrument.is_vault_item() => {
                let activity_id = lookup_activity_id_by_as_id(
                    &conn,
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
                        Status::UnprocessableEntity
                    })?,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to create VaultItem: {e:#?}");
                    Status::InternalServerError
                })?;
            }
            ActivityPub::Object(ApObject::Instrument(instrument))
                if instrument.is_olm_account() =>
            {
                update_olm_account_by_username(
                    &conn,
                    profile.ek_username.clone().ok_or_else(|| {
                        log::error!("Account Instrument must have a username");
                        Status::UnprocessableEntity
                    })?,
                    instrument.content.ok_or_else(|| {
                        log::error!("Account Instrument must have content");
                        Status::UnprocessableEntity
                    })?,
                    instrument.hash.ok_or_else(|| {
                        log::error!("Account Instrument must have a hash");
                        Status::UnprocessableEntity
                    })?,
                    instrument.mutation_of.ok_or_else(|| {
                        log::error!("Account Instrument must have a mutation_of field");
                        Status::UnprocessableEntity
                    })?,
                )
                .await
                .map_err(|e| {
                    log::error!("Failed to update Account: {e:#?}");
                    Status::InternalServerError
                })?;
            }
            _ => (),
        }
    }

    Ok(Status::Accepted)
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct OtkUpdateParams {
    pub keys: HashMap<String, String>,
    pub account: String,
    pub mutation_of: String,
    pub account_hash: String,
}

#[post("/api/user/<username>/otk", format = "json", data = "<params>")]
pub async fn add_one_time_keys(
    signed: Signed,
    conn: Db,
    username: String,
    params: Result<Json<OtkUpdateParams>, Error<'_>>,
) -> Result<Status, Status> {
    log::debug!("Adding One-Time Keys\n{params:#?}");

    let profile = signed.profile().ok_or(Status::Unauthorized)?;

    let Json(params) = params.map_err(|e| {
        log::error!("Failed to decode OtkUpdateParams: {e:#?}");
        Status::UnprocessableEntity
    })?;

    if profile
        .ek_olm_pickled_account_hash
        .ok_or(Status::InternalServerError)?
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
            Status::InternalServerError
        })?;

        for (key, otk) in params.keys {
            create_olm_one_time_key(&conn, (profile.id, key, otk).into()).await;
        }

        Ok(Status::Accepted)
    } else {
        log::error!("UNEXPECTED MUTATION");
        Err(Status::NoContent)
    }
}

#[get("/user/<username>/keys?<otk>", format = "application/activity+json")]
pub async fn keys_get(
    signed: Signed,
    conn: Db,
    username: String,
    otk: Option<bool>,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = get_actor_by_username(&conn, username.clone())
        .await
        .map_err(|_| Status::NotFound)?;

    // Requests for OTKs should come via the external path (i.e., signed.actor() not
    // signed.profile()). The WASM login process does call this endpoint to check for the
    // key population, so the 'else' path allows for unsigned access to the collection
    // description.
    if signed.actor().is_some_and(|x| x.id.is_some()) && otk.is_some_and(|x| x) {
        let otk = get_next_otk_by_profile_id(
            &conn,
            signed.actor().unwrap().id.unwrap().to_string(),
            profile.id,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to get OTK: {e:#?}");
            Status::NotFound
        })?;

        let instruments: Vec<ApInstrument> = profile.clone().into();

        let idk: ApInstrument = instruments
            .into_iter()
            .find(|instrument| instrument.kind == ApInstrumentType::OlmIdentityKey)
            .ok_or(Status::InternalServerError)?;

        // let idk = ApInstrument::try_from(profile.clone()).map_err(|e| {
        //     log::error!("Failed to get IDK: {e:#?}");
        //     Status::InternalServerError
        // })?;

        Ok(ActivityJson(Json(
            ApCollection::from(vec![otk.into(), idk]).into(),
        )))
    } else {
        let count = get_otk_count_by_profile_id(&conn, profile.id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve OTK count: {e:#?}");
                Status::InternalServerError
            })?;

        let mut collection = ApCollection::default();
        collection.total_items = Some(count as i64);
        collection.id = Some(format!(
            "https://{}/user/{username}/keys",
            *crate::SERVER_NAME
        ));

        Ok(ActivityJson(Json(collection.into())))
    }
}

#[get(
    "/user/<username>/keys?<mkp>&<count>",
    format = "application/activity+json"
)]
pub async fn keys_mkp_get(
    signed: Signed,
    conn: Db,
    username: String,
    mkp: Option<bool>,
    count: Option<bool>,
) -> Result<ActivityJson<ApObject>, Status> {
    let profile = get_actor_by_username(&conn, username.clone())
        .await
        .map_err(|_| Status::NotFound)?;

    // Requests for KeyPackages should come via the external path (i.e., signed.actor() not
    // signed.profile()). The WASM login process does call this endpoint to check for the
    // key population, so the 'else' path allows for unsigned access to the collection
    // description.
    if signed.actor().is_some_and(|x| x.id.is_some()) && mkp.is_some_and(|x| x) {
        let mkp = get_next_mkp_by_actor_id(
            &conn,
            signed.actor().unwrap().id.unwrap().to_string(),
            profile.id,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to get MKP: {e:#?}");
            Status::NotFound
        })?;

        Ok(ActivityJson(Json(
            ApCollection::from(vec![mkp.into()]).into(),
        )))
    } else if signed.profile().is_some() && (count.is_some_and(|x| !x) || count.is_none()) {
        let actor = signed.profile().unwrap();
        let instruments: Vec<ApInstrument> = actor.into();
        Ok(ActivityJson(Json(ApCollection::from(instruments).into())))
    } else {
        let count = get_mkp_count_by_profile_id(&conn, profile.id)
            .await
            .map_err(|e| {
                log::error!("Failed to retrieve MKP count: {e:#?}");
                Status::InternalServerError
            })?;

        let mut collection = ApCollection::default();
        collection.total_items = Some(count as i64);
        collection.id = Some(format!(
            "https://{}/user/{username}/keys",
            *crate::SERVER_NAME
        ));

        Ok(ActivityJson(Json(collection.into())))
    }
}
