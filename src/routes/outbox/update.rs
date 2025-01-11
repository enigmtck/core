use crate::{
    db::Db,
    models::{
        activities::{create_activity, NewActivity, TryFromExtendedActivity},
        actors::{set_mls_credentials_by_username, update_mls_storage_by_username, Actor},
        mls_key_packages::create_mls_key_package,
    },
    routes::{ActivityJson, Outbox},
};
use jdt_activity_pub::{
    ActivityPub, ApActivity, ApInstrumentType, ApObject, ApUpdate, Collectible,
};
use jdt_maybe_reference::MaybeReference;
use rocket::http::Status;
use serde_json::Value;

impl Outbox for ApUpdate {
    async fn outbox(
        &self,
        conn: Db,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        if let MaybeReference::Actual(ApObject::Collection(collection)) = self.object.clone() {
            let items = collection.items().ok_or(Status::UnprocessableEntity)?;
            for item in items {
                if let ActivityPub::Object(ApObject::Instrument(instrument)) = item {
                    log::debug!("Updating Instrument: {instrument:#?}");
                    match instrument.kind {
                        ApInstrumentType::MlsCredentials => {
                            set_mls_credentials_by_username(
                                &conn,
                                profile
                                    .ek_username
                                    .clone()
                                    .ok_or(Status::InternalServerError)?,
                                instrument.content.ok_or_else(|| {
                                    log::debug!("MlsCredentials content must be Some");
                                    Status::UnprocessableEntity
                                })?,
                            )
                            .await
                            .map_err(|e| {
                                log::debug!("Failed to set Credentials: {e:#?}");
                                Status::InternalServerError
                            })?;
                        }
                        ApInstrumentType::MlsStorage => {
                            update_mls_storage_by_username(
                                &conn,
                                profile
                                    .ek_username
                                    .clone()
                                    .ok_or(Status::InternalServerError)?,
                                instrument.content.ok_or_else(|| {
                                    log::debug!("MlsStorage content must be Some");
                                    Status::UnprocessableEntity
                                })?,
                                instrument.hash.ok_or_else(|| {
                                    log::debug!("MlsStorage hash must be Some");
                                    Status::UnprocessableEntity
                                })?,
                                instrument.mutation_of,
                            )
                            .await
                            .map_err(|e| {
                                log::debug!("Failed to set Storage: {e:#?}");
                                Status::InternalServerError
                            })?;
                        }
                        ApInstrumentType::MlsKeyPackage => {
                            create_mls_key_package(
                                &conn,
                                (
                                    profile.id,
                                    instrument.content.ok_or_else(|| {
                                        log::debug!("MlsKeyPackage content must be Some");
                                        Status::UnprocessableEntity
                                    })?,
                                )
                                    .into(),
                            )
                            .await
                            .map_err(|e| {
                                log::debug!("Failed to create KeyPackage: {e:#?}");
                                Status::InternalServerError
                            })?;
                        }
                        _ => (),
                    }
                }
            }
        };

        let mut activity = NewActivity::try_from((self.clone().into(), None)).map_err(|e| {
            log::error!("Failed to build Activity: {e:#?}");
            Status::InternalServerError
        })?;

        activity.raw = Some(raw);

        Ok(ApActivity::try_from_extended_activity((
            create_activity(Some(&conn), activity.clone())
                .await
                .map_err(|e| {
                    log::error!("Failed to create Activity: {e:#?}");
                    Status::InternalServerError
                })?,
            None,
            None,
            None,
        ))
        .map_err(|_| Status::InternalServerError)?
        .into())
    }
}
