use crate::{
    activity_pub::{ApImage, ApImageType},
    db::Db,
    fairings::faktory::{assign_to_faktory, FaktoryConnection},
    models::profiles::{update_avatar_by_username, update_banner_by_username},
};
use image::{imageops::FilterType, io::Reader, DynamicImage, ImageFormat};
use rocket::{
    data::{Data, ToByteUnit},
    http::Status,
    post,
    serde::json::Error,
    serde::json::Json,
};
use serde::Deserialize;

use crate::{
    fairings::signatures::Signed,
    models::profiles::{update_summary_by_username, Profile},
    signing::VerificationType,
};

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
    faktory: FaktoryConnection,
    username: String,
    summary: Result<Json<SummaryUpdate>, Error<'_>>,
) -> Result<Json<Profile>, Status> {
    log::debug!("UPDATING SUMMARY\n{summary:#?}");

    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(Json(summary)) = summary {
            if let Some(profile) =
                update_summary_by_username(&conn, username, summary.content, summary.markdown).await
            {
                if assign_to_faktory(
                    faktory,
                    String::from("send_profile_update"),
                    vec![profile.uuid.clone()],
                )
                .is_ok()
                {
                    Ok(Json(profile))
                } else {
                    Err(Status::NoContent)
                }
            } else {
                Err(Status::NoContent)
            }
        } else {
            Err(Status::NoContent)
        }
    } else {
        Err(Status::NoContent)
    }
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

fn process_banner(filename: String) -> Option<ApImage> {
    let path = &format!("{}/banners/{}", *crate::MEDIA_DIR, filename);

    if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
        meta.clear();
        if meta.save_to_file(path).is_ok() {
            if let Ok(img) = Reader::open(path) {
                if let Ok(img) = img.with_guessed_format() {
                    if let Ok(decode) = img.decode() {
                        let decode = banner(decode);
                        let decode = decode.resize(1500, 500, FilterType::CatmullRom);

                        if decode.save_with_format(path, ImageFormat::Png).is_ok() {
                            Some(ApImage {
                                kind: crate::activity_pub::ApImageType::Image,
                                media_type: Some("image/png".to_string()),
                                url: format!("{}/media/banners/{}", *crate::SERVER_URL, filename),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
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

fn process_avatar(filename: String) -> Option<ApImage> {
    let path = &format!("{}/avatars/{}", *crate::MEDIA_DIR, filename);

    if let Ok(meta) = rexiv2::Metadata::new_from_path(path) {
        meta.clear();
        if meta.save_to_file(path).is_ok() {
            if let Ok(img) = Reader::open(path) {
                if let Ok(img) = img.with_guessed_format() {
                    if let Ok(decode) = img.decode() {
                        let decode = square(decode);
                        let decode = decode.resize(400, 400, FilterType::CatmullRom);

                        if decode.save_with_format(path, ImageFormat::Png).is_ok() {
                            Some(ApImage {
                                kind: ApImageType::Image,
                                media_type: Some("image/png".to_string()),
                                url: format!("{}/media/avatars/{}", *crate::SERVER_URL, filename),
                            })
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

#[post("/api/user/<username>/avatar?<_extension>", data = "<media>")]
pub async fn upload_avatar(
    signed: Signed,
    conn: Db,
    username: String,
    _extension: String,
    media: Data<'_>,
) -> Result<Status, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        let filename = uuid::Uuid::new_v4().to_string();

        if let Ok(file) = media
            .open(4.mebibytes())
            .into_file(&format!("{}/avatars/{}", *crate::MEDIA_DIR, filename))
            .await
        {
            if file.is_complete() {
                if process_avatar(filename.clone()).is_some() {
                    if update_avatar_by_username(&conn, username, filename)
                        .await
                        .is_some()
                    {
                        Ok(Status::Accepted)
                    } else {
                        log::error!("FAILED TO UPDATE DATABASE");
                        Err(Status::NoContent)
                    }
                } else {
                    log::error!("FAILED TO PROCESS AVATAR");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FILE UPLOAD WAS TOO LARGE");
                Err(Status::PayloadTooLarge)
            }
        } else {
            log::error!("COULD NOT OPEN MEDIA FILE");
            Err(Status::UnsupportedMediaType)
        }
    } else {
        log::error!("UNAUTHORIZED");
        Err(Status::Forbidden)
    }
}

#[post("/api/user/<username>/banner?<_extension>", data = "<media>")]
pub async fn upload_banner(
    signed: Signed,
    conn: Db,
    username: String,
    _extension: String,
    media: Data<'_>,
) -> Result<Status, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        let filename = uuid::Uuid::new_v4().to_string();

        if let Ok(file) = media
            .open(4.mebibytes())
            .into_file(&format!("{}/banners/{}", *crate::MEDIA_DIR, filename))
            .await
        {
            if file.is_complete() {
                if process_banner(filename.clone()).is_some() {
                    if update_banner_by_username(&conn, username, filename)
                        .await
                        .is_some()
                    {
                        Ok(Status::Accepted)
                    } else {
                        log::error!("FAILED TO UPDATE DATABASE");
                        Err(Status::NoContent)
                    }
                } else {
                    log::error!("FAILED TO PROCESS BANNER");
                    Err(Status::NoContent)
                }
            } else {
                log::error!("FILE UPLOAD WAS TOO LARGE");
                Err(Status::PayloadTooLarge)
            }
        } else {
            log::error!("COULD NOT OPEN MEDIA FILE");
            Err(Status::UnsupportedMediaType)
        }
    } else {
        log::error!("UNAUTHORIZED");
        Err(Status::Forbidden)
    }
}
