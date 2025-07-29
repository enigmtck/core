use crate::{
    db::runner::DbRunner,
    models::{
        activities::lookup_activity_id_by_as_id,
        actors::{
            get_actor_by_username, set_mls_credentials_by_username, update_avatar_by_username,
            update_banner_by_username, update_mls_storage_by_username, update_summary_by_username,
            Actor,
        },
        follows::{
            get_follower_count_by_actor_id, get_followers_by_actor_id,
            get_leader_count_by_follower_actor_id, get_leaders_by_follower_actor_id,
        },
        mls_group_conversations::create_mls_group_conversation,
        mls_key_packages::create_mls_key_package,
        unprocessable::create_unprocessable,
        vault::{create_vault_item, VaultItemParams},
        OffsetPaging,
    },
    runner::{self, user::send_actor_update_task},
    server::{extractors::AxumSigned, AppState},
    LoadEphemeral,
};
use axum::{
    body::Bytes,
    extract::{rejection::JsonRejection, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Json, Redirect},
};
use image::{imageops::FilterType, io::Reader, DynamicImage};
use jdt_activity_pub::{
    ActivityPub, ApActor, ApCollection, ApImage, ApInstrument, ApInstrumentType, ApObject,
    Collectible, FollowersPage, LeadersPage, MaybeReference,
};
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use tokio::fs;

use super::{AbstractResponse, ActivityJson, LdJson};

#[derive(Deserialize, Debug, Clone)]
pub struct SummaryUpdate {
    pub content: String,
    pub markdown: String,
}

fn banner(mut image: DynamicImage) -> DynamicImage {
    let width = image.width();
    let height = image.height();

    match width != (height * 3) {
        true if width > (height * 3) => {
            let extra = width - (height * 3);
            let side = extra / 2;
            image.crop(side, 0, height * 3, height)
        }
        true if width < (height * 3) => {
            let extra = height - (width / 3);
            let top = extra / 2;
            image.crop(0, top, width, width / 3)
        }
        _ => image,
    }
}

fn process_banner(filename: String, media_type: String) -> Option<ApImage> {
    let path = &format!("{}/banners/{}", *crate::MEDIA_DIR, filename);

    let meta = rexiv2::Metadata::new_from_path(path).ok()?;
    meta.clear();
    meta.save_to_file(path).ok()?;
    let img = Reader::open(path).ok()?;
    let img = img.with_guessed_format().ok()?;
    let decode = img.decode().ok()?;
    let decode = banner(decode);
    let decode = decode.resize(1500, 500, FilterType::CatmullRom);

    if decode.save(path).is_ok() {
        let mut image = ApImage::from(format!(
            "https://{}/media/banners/{}",
            *crate::SERVER_NAME,
            filename
        ));
        image.media_type = Some(media_type);
        Some(image)
    } else {
        None
    }
}

fn square(mut image: DynamicImage) -> DynamicImage {
    let width = image.width();
    let height = image.height();

    match width != height {
        true if width > height => {
            let extra = width - height;
            let side = extra / 2;
            image.crop(side, 0, height, height)
        }
        true if width < height => {
            let extra = height - width;
            let top = extra / 2;
            image.crop(0, top, width, width)
        }
        _ => image,
    }
}

fn process_avatar(filename: String, media_type: String) -> Option<ApImage> {
    let path = &format!("{}/avatars/{}", *crate::MEDIA_DIR, filename);

    let meta = rexiv2::Metadata::new_from_path(path).ok()?;
    meta.clear();
    meta.save_to_file(path).ok()?;
    let img = Reader::open(path).ok()?;
    let img = img.with_guessed_format().ok()?;
    let decode = img.decode().ok()?;
    let decode = square(decode);
    let decode = decode.resize(400, 400, FilterType::CatmullRom);

    if decode.save(path).is_ok() {
        let mut image = ApImage::from(format!(
            "https://{}/media/avatars/{}",
            *crate::SERVER_NAME,
            filename
        ));
        image.media_type = Some(media_type);
        Some(image)
    } else {
        None
    }
}

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

            create_vault_item(
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
            })?;
        }
        _ => (),
    }
    Ok(())
}

pub async fn process_collection_items<C: DbRunner>(
    conn: &C,
    profile: &Actor,
    collection: ApCollection,
) -> Result<(), StatusCode> {
    let items = collection.items().ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;

    for item in items {
        if let ActivityPub::Object(ApObject::Instrument(instrument)) = item {
            log::debug!("Updating Instrument: {instrument:#?}");
            process_instrument(conn, profile, &instrument).await?;
        }
    }
    Ok(())
}

pub async fn person_post(
    State(state): State<AppState>,
    Path(username): Path<String>,
    signed: AxumSigned,
    raw: Result<Json<Value>, JsonRejection>,
) -> Result<ActivityJson<ApObject>, StatusCode> {
    let actor = signed.profile().ok_or(StatusCode::UNAUTHORIZED)?;
    if username.as_str()
        != actor
            .ek_username
            .as_deref()
            .ok_or(StatusCode::UNAUTHORIZED)?
    {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let raw = raw.map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?.0;
    log::debug!("POSTING TO ACTOR\n{raw:#?}");

    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Ok(ApObject::Collection(collection)) = serde_json::from_value::<ApObject>(raw.clone()) {
        process_collection_items(&conn, &actor, collection).await?;
        Ok(ActivityJson(ApObject::Actor(ApActor::from(actor))))
    } else {
        create_unprocessable(&conn, raw.into()).await;
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    }
}

pub async fn person_get(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
    headers: HeaderMap,
) -> Result<AbstractResponse<ApActor>, StatusCode> {
    let conn = match state.db_pool.get().await {
        Ok(c) => c,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let profile = match get_actor_by_username(&conn, username.clone()).await {
        Ok(p) => p,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    let actor = if signed.local() {
        ApActor::from(profile)
            .load_ephemeral(&conn, signed.profile())
            .await
    } else {
        log::debug!("Returning actor without ephemeral data");
        ApActor::from(profile)
    };

    if let Some(accept) = headers.get(header::ACCEPT) {
        if let Ok(accept_str) = accept.to_str() {
            if accept_str.contains("text/html") {
                log::debug!("Redirecting to presentation page");
                return Ok(AbstractResponse::Redirect(Redirect::to(&format!(
                    "/@{username}"
                ))));
            }
            if accept_str.contains("application/activity+json") {
                log::debug!("Returning application/activity+json");
                return Ok(AbstractResponse::ActivityJson(ActivityJson(actor)));
            }
            if accept_str.contains("application/ld+json") {
                log::debug!("Returning application/ld+json");
                return Ok(AbstractResponse::LdJson(LdJson(actor)));
            }
        }
    }

    // Default to activity+json
    Ok(AbstractResponse::ActivityJson(ActivityJson(actor)))
}

pub async fn liked_get(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<ActivityJson<ApCollection>, StatusCode> {
    // This is a stub
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if get_actor_by_username(&conn, username).await.is_ok() {
        Ok(ActivityJson(ApCollection::default()))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

#[derive(Deserialize)]
pub struct PagingQuery {
    page: Option<u32>,
}

pub async fn get_followers(
    State(state): State<AppState>,
    Path(username): Path<String>,
    Query(query): Query<PagingQuery>,
) -> Result<ActivityJson<ApCollection>, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let profile = get_actor_by_username(&conn, username)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let total_items = get_follower_count_by_actor_id(&conn, profile.id)
        .await
        .map_err(|e| {
            log::error!("Failed to retrieve Follower count: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let results = match query.page {
        Some(p) if p > 0 => get_followers_by_actor_id(
            &conn,
            profile.id,
            Some(OffsetPaging {
                page: p - 1,
                limit: 20,
            }),
        )
        .await
        .map_err(|e| {
            log::error!("Failed to get Followers: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
        _ => vec![],
    };

    let followers = results
        .iter()
        .map(|(follower, _)| {
            ActivityPub::try_from(MaybeReference::<ApActor>::Reference(
                follower.clone().follower_ap_id,
            ))
            .unwrap()
        })
        .collect();

    let actors = Some(
        results
            .iter()
            .map(|(_, actor)| actor.clone())
            .collect::<Vec<_>>(),
    );

    Ok(ActivityJson(
        ApCollection::try_from(FollowersPage {
            page: query.page,
            username: profile.ek_username.ok_or_else(|| {
                log::error!("Profile must have a Username");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
            total_items,
            followers,
            actors,
        })
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE FOLLOWERS: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
    ))
}

pub async fn get_leaders(
    State(state): State<AppState>,
    Path(username): Path<String>,
    Query(query): Query<PagingQuery>,
) -> Result<ActivityJson<ApCollection>, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let profile = get_actor_by_username(&conn, username)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let total_items = get_leader_count_by_follower_actor_id(&conn, profile.id)
        .await
        .map_err(|e| {
            log::error!("Failed to retrieve Leader count: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let results = match query.page {
        Some(p) if p > 0 => get_leaders_by_follower_actor_id(
            &conn,
            profile.id,
            Some(OffsetPaging {
                page: p - 1,
                limit: 20,
            }),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
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

    Ok(ActivityJson(
        ApCollection::try_from(LeadersPage {
            page: query.page,
            username: profile.ek_username.ok_or_else(|| {
                log::error!("Profile must have a Username");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
            total_items,
            leaders,
            actors,
        })
        .map_err(|e| {
            log::error!("FAILED TO RETRIEVE LEADERS: {e:#?}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?,
    ))
}

pub async fn update_summary(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
    summary: Result<Json<SummaryUpdate>, JsonRejection>,
) -> Result<Json<Actor>, StatusCode> {
    if !signed.local() {
        return Err(StatusCode::FORBIDDEN);
    }

    let Json(summary) = summary.map_err(|e| {
        log::error!("Failed to decode Summary: {e:#?}");
        StatusCode::BAD_REQUEST
    })?;

    let db_pool = state.db_pool.clone();

    let conn = db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let profile = update_summary_by_username(&conn, username, summary.content, summary.markdown)
        .await
        .map_err(|e| {
            log::error!("Failed to update Summary: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let uuid = profile
        .ek_uuid
        .clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    runner::run(send_actor_update_task, db_pool, None, vec![uuid]).await;

    Ok(Json(profile))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
    bytes: Bytes,
) -> Result<StatusCode, StatusCode> {
    if !signed.local() {
        return Err(StatusCode::FORBIDDEN);
    }

    if bytes.len() > 20 * 1024 * 1024 {
        // 20 MiB
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let kind = infer::get(&bytes).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let mime_type_str = kind.mime_type().to_string();
    let filename = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension());
    let path = format!("{}/avatars/{}", *crate::MEDIA_DIR, filename);
    let url = format!("https://{}/media/avatars/{}", *crate::SERVER_NAME, filename);
    let as_image: ApImage = url.clone().into();

    fs::write(&path, &bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if process_avatar(filename.clone(), mime_type_str).is_none() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let db_pool = state.db_pool.clone();

    let conn = db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let actor = update_avatar_by_username(&conn, username, filename, json!(as_image))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let uuid = actor
        .ek_uuid
        .clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    runner::run(send_actor_update_task, db_pool, None, vec![uuid]).await;

    Ok(StatusCode::ACCEPTED)
}

pub async fn upload_banner(
    State(state): State<AppState>,
    signed: AxumSigned,
    Path(username): Path<String>,
    bytes: Bytes,
) -> Result<StatusCode, StatusCode> {
    if !signed.local() {
        return Err(StatusCode::FORBIDDEN);
    }

    if bytes.len() > 20 * 1024 * 1024 {
        // 20 MiB
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let kind = infer::get(&bytes).ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
    let mime_type_str = kind.mime_type().to_string();
    let filename = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension());
    let path = format!("{}/banners/{}", *crate::MEDIA_DIR, filename);
    let url = format!("https://{}/media/banners/{}", *crate::SERVER_NAME, filename);
    let as_image: ApImage = url.clone().into();

    fs::write(&path, &bytes)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if process_banner(filename.clone(), mime_type_str).is_none() {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let db_pool = state.db_pool.clone();
    let conn = db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let actor = update_banner_by_username(&conn, username, filename, json!(as_image))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let uuid = actor
        .ek_uuid
        .clone()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    runner::run(send_actor_update_task, db_pool, None, vec![uuid]).await;

    Ok(StatusCode::ACCEPTED)
}

pub async fn user_get_api(
    State(state): State<AppState>,
    Path(username): Path<String>,
    signed: AxumSigned,
) -> Result<ActivityJson<ApActor>, StatusCode> {
    let conn = state
        .db_pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let profile = get_actor_by_username(&conn, username)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let actor = ApActor::from(profile)
        .load_ephemeral(&conn, signed.profile())
        .await;
    Ok(ActivityJson(actor))
}
