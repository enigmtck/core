use crate::{
    db::runner::DbRunner,
    models::{
        activities::lookup_activity_id_by_as_id,
        actors::{set_mls_credentials_by_username, update_mls_storage_by_username, Actor},
        mls_group_conversations::create_mls_group_conversation,
        mls_key_packages::create_mls_key_package,
        vault::{create_vault_item, VaultItemParams},
    },
};
use jdt_activity_pub::{ApInstrument, ApInstrumentType};
use reqwest::StatusCode;

// async fn process_collection_items<C: DbRunner>(
//     conn: &C,
//     profile: &Actor,
//     collection: ApCollection,
// ) -> Result<(), StatusCode> {
//     let items = collection.items().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;

//     for item in items {
//         if let ActivityPub::Object(ApObject::Instrument(instrument)) = item {
//             log::debug!("Updating Instrument: {instrument:#?}");
//             process_instrument(conn, profile, &instrument).await?;
//         }
//     }
//     Ok(())
// }

pub async fn process_instrument<C: DbRunner>(
    conn: &C,
    profile: &Actor,
    instrument: &ApInstrument,
) -> Result<(), StatusCode> {
    let username = profile
        .ek_username
        .clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    match instrument.kind {
        ApInstrumentType::MlsGroupId => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsGroupId content must be Some");
                StatusCode::UNPROCESSABLE_ENTITY
            })?;

            let conversation = instrument.clone().conversation.ok_or_else(|| {
                log::error!("MlsGroupId conversation cannot be None");
                StatusCode::UNPROCESSABLE_ENTITY
            })?;

            create_mls_group_conversation(conn, (profile.id, content, conversation).into())
                .await
                .map_err(|e| {
                    log::error!("Failed to create or update GroupId: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        ApInstrumentType::MlsCredentials => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsCredentials content must be Some");
                StatusCode::UNPROCESSABLE_ENTITY
            })?;
            set_mls_credentials_by_username(conn, username, content)
                .await
                .map_err(|e| {
                    log::debug!("Failed to set Credentials: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        ApInstrumentType::MlsStorage => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsStorage content must be Some");
                StatusCode::UNPROCESSABLE_ENTITY
            })?;
            let hash = instrument.hash.clone().ok_or_else(|| {
                log::debug!("MlsStorage hash must be Some");
                StatusCode::UNPROCESSABLE_ENTITY
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
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
        ApInstrumentType::MlsKeyPackage => {
            let content = instrument.content.clone().ok_or_else(|| {
                log::debug!("MlsKeyPackage content must be Some");
                StatusCode::UNPROCESSABLE_ENTITY
            })?;
            create_mls_key_package(conn, (profile.id, content).into())
                .await
                .map_err(|e| {
                    log::debug!("Failed to create KeyPackage: {e:#?}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        }
        ApInstrumentType::VaultItem => {
            let activity_id = lookup_activity_id_by_as_id(
                conn,
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
                    StatusCode::UNPROCESSABLE_ENTITY
                })?,
            )
            .await
            .map_err(|e| {
                log::error!("Failed to create VaultItem: {e:#?}");
                StatusCode::INTERNAL_SERVER_ERROR
            });

            log::debug!("VaultItem insert result: {result:#?}");
        }
        _ => (),
    }
    Ok(())
}
