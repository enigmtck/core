use dotenvy::dotenv;
use rocket::{Build, Rocket};

use crate::{
    db::Db,
    fairings::{
        access_control::BlockList, events::EventChannels, faktory::FaktoryConnection,
        mq::MqConnection,
    },
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
        .attach(FaktoryConnection::fairing())
        .attach(EventChannels::fairing())
        .attach(Db::fairing())
        .attach(BlockList::fairing())
        .attach(MqConnection::fairing())
        .mount(
            "/",
            routes![
                person_redirect,
                person,
                webfinger_jrd_json,
                webfinger_activity_json,
                webfinger_json,
                webfinger_xml,
                outbox_post,
                outbox_get,
                inbox_post,
                shared_inbox_post,
                timeline,
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
                conversation_get_local,
                authorize_stream,
                host_meta,
                cached_image
            ],
        )
}

pub fn start() {
    main()
}
