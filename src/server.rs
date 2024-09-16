use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::PathBuf;

use dotenvy::dotenv;
use rocket::fs::FileServer;
use rocket::http::{ContentType, RawStr};
use rocket::request::FromParam;
use rocket::response::content::RawHtml;
use rocket::{Build, Rocket};
use rust_embed::RustEmbed;

use crate::{
    db::Db,
    fairings::{access_control::BlockList, events::EventChannels},
    routes::{
        api::{
            admin::*, authentication::*, encryption::*, image::*, profile::*, remote::*, stream::*,
            vault::*,
        },
        inbox::*,
        instance::*,
        notes::*,
        outbox::*,
        user::*,
        webfinger::*,
    },
};

#[derive(RustEmbed)]
#[folder = "client/"]
pub struct Client;

#[allow(dead_code)]
pub struct Handle<'r> {
    name: &'r str,
}

impl<'r> FromParam<'r> for Handle<'r> {
    type Error = &'r RawStr;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        if param.starts_with('@') {
            Ok(Handle { name: param })
        } else {
            Err(param.into())
        }
    }
}

#[allow(unused_variables)]
#[get("/<handle>")]
async fn client_profile(handle: Handle<'_>) -> Option<RawHtml<Cow<'static, [u8]>>> {
    let asset = Client::get("200.html")?;
    Some(RawHtml(asset.data))
}

#[allow(unused_variables)]
#[get("/notes?<uuid>")]
async fn client_notes(uuid: String) -> Option<RawHtml<Cow<'static, [u8]>>> {
    let client = Client::get("200.html")?;
    Some(RawHtml(client.data))
}

#[get("/timeline")]
async fn client_timeline() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let client = Client::get("200.html")?;
    Some(RawHtml(client.data))
}

#[get("/signup")]
async fn client_signup() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let client = Client::get("200.html")?;
    Some(RawHtml(client.data))
}

#[get("/login")]
async fn client_login() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let client = Client::get("200.html")?;
    Some(RawHtml(client.data))
}

#[get("/")]
async fn client_index() -> Option<RawHtml<Cow<'static, [u8]>>> {
    let client = Client::get("200.html")?;
    Some(RawHtml(client.data))
}

#[get("/_app/<file..>")]
async fn client_app_file(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = format!("_app/{}", file.display());
    log::debug!("FILENAME: {filename}");

    let asset = Client::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[get("/assets/<file..>")]
async fn client_assets_file(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = format!("assets/{}", file.display());
    log::debug!("FILENAME: {filename}");

    let asset = Client::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[get("/fontawesome/<file..>")]
async fn client_fontawesome_file(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = format!("fontawesome/{}", file.display());
    log::debug!("FILENAME: {filename}");

    let asset = Client::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[get("/fonts/<file..>")]
async fn client_fonts_file(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = format!("fonts/{}", file.display());
    log::debug!("FILENAME: {filename}");

    let asset = Client::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[get("/highlight/<file..>")]
async fn client_highlight_file(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = format!("highlight/{}", file.display());
    log::debug!("FILENAME: {filename}");

    let asset = Client::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[get("/icons/<file..>")]
async fn client_icons_file(file: PathBuf) -> Option<(ContentType, Cow<'static, [u8]>)> {
    let filename = format!("icons/{}", file.display());
    log::debug!("FILENAME: {filename}");

    let asset = Client::get(&filename)?;
    let content_type = file
        .extension()
        .and_then(OsStr::to_str)
        .and_then(ContentType::from_extension)
        .unwrap_or(ContentType::Bytes);

    Some((content_type, asset.data))
}

#[launch]
fn rocket() -> Rocket<Build> {
    if let Ok(profile) = std::env::var("PROFILE") {
        match profile.as_str() {
            "debug" => log4rs::init_file("log4rs.yml", Default::default()).unwrap(),
            "release" => env_logger::init(),
            _ => (),
        }
    } else {
        env_logger::init();
    }

    dotenv().ok();

    rocket::build()
        .attach(EventChannels::fairing())
        .attach(Db::fairing())
        .attach(BlockList::fairing())
        .mount("/media/avatars", FileServer::from("media/avatars").rank(5))
        .mount("/media/banners", FileServer::from("media/banners").rank(6))
        .mount(
            "/",
            routes![
                client_index,
                client_notes,
                client_signup,
                client_login,
                client_timeline,
                client_app_file,
                client_assets_file,
                client_fontawesome_file,
                client_fonts_file,
                client_highlight_file,
                client_icons_file,
                client_profile,
                person_redirect,
                person_activity_json,
                person_ld_json,
                webfinger_jrd_json,
                webfinger_activity_json,
                webfinger_json,
                webfinger_xml,
                outbox_post,
                outbox_get,
                inbox_post,
                shared_inbox_post,
                shared_inbox_get,
                inbox_get,
                liked_get,
                get_followers,
                get_leaders,
                create_user,
                authenticate_user,
                add_one_time_keys,
                get_processing_queue,
                update_processing_queue_item,
                get_olm_session,
                get_sessions,
                store_vault_item,
                vault_get,
                stream,
                instance_information,
                remote_actor,
                remote_actor_authenticated,
                remote_followers,
                remote_followers_authenticated,
                remote_following,
                remote_following_authenticated,
                remote_outbox,
                remote_outbox_authenticated,
                remote_id,
                remote_id_authenticated,
                remote_note,
                update_summary,
                upload_avatar,
                upload_banner,
                upload_image,
                change_password,
                note_get,
                conversation_get,
                //conversation_get_local,
                authorize_stream,
                host_meta,
                cached_image,
                user_activity_json,
                announcers_get,
            ],
        )
}

pub fn start() {
    main()
}
