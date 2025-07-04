use crate::{
    db::Db,
    fairings::events::EventChannels,
    models::{
        actors::{get_actor_by_username, Actor},
        actors::{
            update_avatar_by_username, update_banner_by_username, update_summary_by_username,
        },
    },
    routes::ActivityJson,
    runner, LoadEphemeral,
};
use image::{imageops::FilterType, io::Reader, DynamicImage};
use jdt_activity_pub::{ApActor, ApImage};
use rocket::{
    data::{Data, ToByteUnit},
    http::Status,
    post,
    serde::json::Error,
    serde::json::Json,
};
use serde::Deserialize;
use serde_json::json;

use crate::fairings::signatures::Signed;

#[derive(Deserialize, Debug, Clone)]
pub struct SummaryUpdate {
    content: String,
    markdown: String,
}

#[post(
    "/api/user/<username>/update/summary",
    format = "json",
    data = "<summary>"
)]
pub async fn update_summary(
    signed: Signed,
    conn: Db,
    channels: EventChannels,
    username: String,
    summary: Result<Json<SummaryUpdate>, Error<'_>>,
) -> Result<Json<Actor>, Status> {
    log::debug!("UPDATING SUMMARY\n{summary:#?}");

    if !signed.local() {
        return Err(Status::NoContent);
    }

    let Json(summary) = summary.map_err(|e| {
        log::error!("FAILED TO DECODE Summary: {e:#?}");
        Status::InternalServerError
    })?;

    let profile = update_summary_by_username(&conn, username, summary.content, summary.markdown)
        .await
        .ok_or_else(|| {
            log::error!("FAILED TO UPDATE Summary");
            Status::NoContent
        })?;

    runner::run(
        runner::user::send_actor_update_task,
        conn,
        Some(channels),
        vec![profile.ek_uuid.clone().ok_or(Status::InternalServerError)?],
    )
    .await;

    Ok(Json(profile))
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

#[allow(unused_variables)]
#[post("/api/user/<username>/avatar?<extension>", data = "<media>")]
pub async fn upload_avatar(
    signed: Signed,
    conn: Db,
    username: String,
    extension: String,
    mut media: Data<'_>,
) -> Result<Status, Status> {
    if !signed.local() {
        return Err(Status::Forbidden);
    }
    let header = media.peek(512).await;
    let kind = infer::get(header).ok_or(Status::UnsupportedMediaType)?;
    let mime_type_str = kind.mime_type().to_string();
    let filename = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension());
    let path = format!("{}/avatars/{}", *crate::MEDIA_DIR, filename);
    let url = format!("https://{}/media/avatars/{}", *crate::SERVER_NAME, filename);
    let as_image: ApImage = url.clone().into();

    let file = media
        .open(20.mebibytes())
        .into_file(&path.clone())
        .await
        .map_err(|e| {
            log::error!("FAILED TO SAVE FILE: {e:#?}");
            Status::UnsupportedMediaType
        })?;

    if !file.is_complete() {
        return Err(Status::PayloadTooLarge);
    }

    if process_avatar(filename.clone(), mime_type_str).is_none() {
        return Err(Status::NoContent);
    }

    if let Some(actor) = update_avatar_by_username(&conn, username, filename, json!(as_image)).await
    {
        runner::run(
            runner::user::send_actor_update_task,
            conn,
            None,
            vec![actor.ek_uuid.clone().ok_or(Status::InternalServerError)?],
        )
        .await;
    } else {
        return Err(Status::InternalServerError);
    }

    Ok(Status::Accepted)
}

#[allow(unused_variables)]
#[post("/api/user/<username>/banner?<extension>", data = "<media>")]
pub async fn upload_banner(
    signed: Signed,
    conn: Db,
    username: String,
    extension: String,
    mut media: Data<'_>,
) -> Result<Status, Status> {
    if !signed.local() {
        return Err(Status::Forbidden);
    }

    let filename = uuid::Uuid::new_v4().to_string();
    let header = media.peek(512).await;
    let kind = infer::get(header).ok_or(Status::UnsupportedMediaType)?;
    let mime_type_str = kind.mime_type().to_string();
    let filename = format!("{}.{}", uuid::Uuid::new_v4(), kind.extension());
    let path = format!("{}/banners/{}", *crate::MEDIA_DIR, filename);
    let url = format!("https://{}/media/banners/{}", *crate::SERVER_NAME, filename);
    let as_image: ApImage = url.clone().into();

    let file = media
        .open(20.mebibytes())
        .into_file(&path.clone())
        .await
        .map_err(|e| {
            log::error!("FAILED TO SAVE FILE: {e:#?}");
            Status::UnsupportedMediaType
        })?;

    if !file.is_complete() {
        return Err(Status::PayloadTooLarge);
    }

    if process_banner(filename.clone(), mime_type_str).is_none() {
        return Err(Status::NoContent);
    }

    if let Some(actor) = update_banner_by_username(&conn, username, filename, json!(as_image)).await
    {
        runner::run(
            runner::user::send_actor_update_task,
            conn,
            None,
            vec![actor.ek_uuid.clone().ok_or(Status::InternalServerError)?],
        )
        .await;
    } else {
        return Err(Status::InternalServerError);
    }

    Ok(Status::Accepted)
}

#[get("/api/user/<username>", format = "application/activity+json", rank = 2)]
pub async fn user_activity_json(
    signed: Signed,
    conn: Db,
    username: String,
) -> Result<ActivityJson<ApActor>, Status> {
    if let Ok(profile) = get_actor_by_username(Some(&conn), username).await {
        Ok(ActivityJson(Json(
            ApActor::from(profile)
                .load_ephemeral(&conn, signed.profile())
                .await,
        )))
    } else {
        Err(Status::NotFound)
    }
}
