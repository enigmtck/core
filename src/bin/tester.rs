#[macro_use]
extern crate log;

use enigmatick::{
    activity_pub::{
        retriever,
        sender::{send_activity, send_follower_accept},
        ApActivity, ApActor, ApBasicContentType, ApInstrument, ApNote, ApObject, ApObjectType,
        ApSession, JoinData,
    },
    db::jsonb_set,
    helper::{get_local_username_from_ap_id, is_local, is_public},
    models::{
        encrypted_sessions::{EncryptedSession, NewEncryptedSession},
        followers::Follower,
        leaders::Leader,
        notes::Note,
        processing_queue::{NewProcessingItem, ProcessingItem},
        profiles::{KeyStore, Profile},
        remote_activities::{NewRemoteActivity, RemoteActivity},
        remote_actors::{NewRemoteActor, RemoteActor},
        remote_encrypted_sessions::RemoteEncryptedSession,
        remote_notes::RemoteNote,
        timeline::{
            NewTimelineItem, NewTimelineItemCc, NewTimelineItemTo, TimelineItem, TimelineItemCc,
            TimelineItemTo,
        },
    },
    signing::{Method, SignParams},
};

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use faktory::{ConsumerBuilder, Job};
use lazy_static::lazy_static;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    io,
};
use tokio::runtime::Runtime;

type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

lazy_static! {
    static ref POOL: Pool = {
        let database_url = &*enigmatick::DATABASE_URL;
        debug!("database: {}", database_url);
        let manager = ConnectionManager::<PgConnection>::new(database_url);
        Pool::new(manager).expect("failed to create db pool")
    };
}

pub fn get_leader_by_followers_endpoint(
    endpoint: String,
) -> Vec<(RemoteActor, Leader, Option<Profile>)> {
    use enigmatick::schema::leaders::dsl::{leader_ap_id, leaders, profile_id};
    use enigmatick::schema::profiles::dsl::{id as pid, profiles};
    use enigmatick::schema::remote_actors::dsl::{ap_id as ra_apid, followers, remote_actors};

    if let Ok(conn) = POOL.get() {
        match remote_actors
            .inner_join(leaders.on(leader_ap_id.eq(ra_apid)))
            .left_join(profiles.on(profile_id.eq(pid)))
            .filter(followers.eq(endpoint))
            .get_results::<(RemoteActor, Leader, Option<Profile>)>(&conn)
        {
            Ok(x) => x,
            Err(e) => {
                log::error!("{:#?}", e);
                vec![]
            }
        }
    } else {
        vec![]
    }
}

fn main() {
    env_logger::init();

    let res =
        get_leader_by_followers_endpoint("https://me.dm/users/anildash/followers".to_string());

    println!("{}", &format!("{res:#?}"));
}
