#[macro_use]
extern crate log;

use enigmatick::{
    activity_pub::{
        sender::{
            send_follower_accept,
            send_activity
        },
        JoinData, ApSession, ApActivity, retriever
    },
    helper::get_local_username_from_ap_id,
    db::jsonb_set,
    models::{
        followers::Follower,
        profiles::{Profile, KeyStore},
        remote_actors::RemoteActor,
        remote_encrypted_sessions::RemoteEncryptedSession,
        encrypted_sessions::{EncryptedSession, NewEncryptedSession}, remote_activities::{NewRemoteActivity, RemoteActivity}
    }
};

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use faktory::{ConsumerBuilder, Job};
use lazy_static::lazy_static;
use std::io;
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

pub fn create_remote_activity(remote_activity: NewRemoteActivity) -> Option<RemoteActivity> {
    use enigmatick::schema::remote_activities;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(remote_activities::table)
            .values(&remote_activity)
            .get_result::<RemoteActivity>(&conn) {
                Ok(x) => Some(x),
                Err(e) => { log::debug!("{:#?}",e); Option::None }
            }
    } else {
        Option::None
    }
}

pub fn create_encrypted_session(encrypted_session: NewEncryptedSession) -> Option<EncryptedSession> {
    use enigmatick::schema::encrypted_sessions;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(encrypted_sessions::table)
            .values(&encrypted_session)
            .get_result::<EncryptedSession>(&conn) {
                Ok(x) => Some(x),
                Err(e) => { log::debug!("{:#?}",e); Option::None }
            }
    } else {
        Option::None
    }
}

pub fn get_remote_encrypted_session_by_ap_id(apid: String) -> Option<RemoteEncryptedSession> {
    use enigmatick::schema::remote_encrypted_sessions::dsl::{remote_encrypted_sessions, ap_id};

    if let Ok(conn) = POOL.get() {
        match remote_encrypted_sessions.filter(ap_id.eq(apid)).first::<RemoteEncryptedSession>(&conn) {
            Ok(x) => Option::from(x),
            Err(_) => Option::None
        }
    } else {
        Option::None
    }
}

pub fn get_profile_by_username(username: String) -> Option<Profile> {
    use enigmatick::schema::profiles::dsl::{profiles, username as u};

    if let Ok(conn) = POOL.get() {
        match profiles.filter(u.eq(username)).first::<Profile>(&conn) {
            Ok(x) => Option::from(x),
            Err(_) => Option::None
        }
    } else {
        Option::None
    }
}

pub fn get_remote_actor_by_ap_id(apid: String) -> Option<RemoteActor> {
    use enigmatick::schema::remote_actors::dsl::{remote_actors, ap_id};

    if let Ok(conn) = POOL.get() {
        match remote_actors.filter(ap_id.eq(apid)).first::<RemoteActor>(&conn) {
            Ok(x) => Option::from(x),
            Err(_) => Option::None
        }
    } else {
        Option::None
    }
}

pub fn update_otk_by_username(username: String, keystore: KeyStore) -> Option<Profile> {
    use enigmatick::schema::profiles::dsl::{profiles, username as u, keystore as k};

    if let Ok(conn) = POOL.get() {
        match diesel::update(profiles.filter(u.eq(username)))
            .set(k.eq(jsonb_set(k,
                                vec![String::from("olm_one_time_keys")],
                                serde_json::to_value(keystore.olm_one_time_keys).unwrap())))
            .get_result::<Profile>(&conn) {
                Ok(x) => Some(x),
                Err(_) => Option::None
            }
    } else {
        Option::None
    }
}

fn provide_one_time_key(job: Job) -> io::Result<()> {
    // look up remote_encrypted_session with ap_id from job.args()

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    
    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();

        if let Some(session) = get_remote_encrypted_session_by_ap_id(ap_id) {
            // this is the username of the Enigmatick user who received the Invite
            if let Some(username) = get_local_username_from_ap_id(session.ap_to.clone()) {
                if let Some(profile) = get_profile_by_username(username.clone()) {
                    if let Some(actor) = get_remote_actor_by_ap_id(session.attributed_to.clone()) {
                        // send Join activity with Identity and OTK to attributed_to
                        let mut keystore = profile.keystore.clone();
                        
                        if let Some(identity_key) = keystore.olm_identity_public_key.clone() {
                            if let Some(mut otk) = keystore.olm_one_time_keys.clone() {
                                let mut keys = otk.keys().collect::<Vec<&String>>();

                                // it feels nice to address these in order, but I'm not sure
                                // that it actually matters; I may remove these
                                keys.sort();
                                keys.reverse();
                                
                                let key = keys.first().unwrap().to_string();
                                let value = otk.remove(&key).unwrap();
                                log::debug!("identity_key\n{:#?}", identity_key);
                                log::debug!("value\n{:#?}", value);
                                keystore.olm_one_time_keys = Some(otk);
                                if update_otk_by_username(username, keystore).is_some() {
                                    let session = ApSession::from(JoinData {
                                        one_time_key: base64::encode(value),
                                        identity_key,
                                        to: session.attributed_to,
                                        attributed_to: session.ap_to,
                                        reference: session.ap_id
                                    });
                                    
                                    let activity = ApActivity::from(session.clone());
                                    let mut encrypted_session =
                                        NewEncryptedSession::from(session.clone());
                                    encrypted_session.profile_id = profile.id;

                                    // this activity should be saved so that the id makes sense
                                    // but it's not right now
                                    log::debug!("activity\n{:#?}", activity);

                                    if create_encrypted_session(encrypted_session).is_some() {
                                        handle.block_on(async {
                                            match send_activity(activity, profile, actor.inbox).await {
                                                Ok(_) => {
                                                    info!("join sent: {:#?}", actor.ap_id);
                                                }
                                                Err(e) => error!("error: {:#?}", e),
                                            }
                                        });
                                    } else {
                                        log::error!("failed to save encrypted_session");
                                    }
                                } else {
                                    log::error!("failed to update keystore");
                                }
                            }
                        }   
                    }
                }
            }
        }
    }
    Ok(())
}

fn acknowledge_followers(job: Job) -> io::Result<()> {
    use enigmatick::schema::followers::dsl::{followers, uuid as uid};
    use enigmatick::schema::profiles::dsl::profiles;
    use enigmatick::schema::remote_actors::dsl::{ap_id as aid, remote_actors};

    debug!("running acknowledge job");

    let uuids = job.args();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match POOL.get() {
        Ok(conn) => {
            for uuid in uuids {
                let uuid = uuid.as_str().unwrap().to_string();
                debug!("looking for uuid: {}", uuid);

                match followers.filter(uid.eq(uuid)).first::<Follower>(&conn) {
                    Ok(follower) => {
                        debug!("follower\n{:#?}\n", follower);

                        let ap_id = follower.ap_id;

                        match profiles.find(follower.profile_id).first::<Profile>(&conn) {
                            Ok(profile) => {
                                debug!("profile\n{:#?}\n", profile);

                                match remote_actors
                                    .filter(aid.eq(follower.actor))
                                    .first::<RemoteActor>(&conn)
                                {
                                    Ok(actor) => {
                                        debug!("actor\n{:#?}\n", actor);

                                        handle.block_on(async {
                                            match send_follower_accept(ap_id, profile, actor.clone()).await {
                                                Ok(_) => {
                                                    info!("accept sent: {:#?}", actor.ap_id);
                                                }
                                                Err(e) => error!("error: {:#?}", e),
                                            }
                                        });
                                    }
                                    Err(e) => error!("failed to find RemoteActor: {:#?}", e),
                                }
                            }
                            Err(e) => error!("failed to find Profile: {:#?}", e),
                        }
                    }
                    Err(e) => {
                        error!("failed to find Follower: {:#?}", e)
                    }
                }
            }
        }
        Err(e) => error!("error: {:#?}", e),
    }

    Ok(())
}

fn main() {
    env_logger::init();

    let faktory_url = &*enigmatick::FAKTORY_URL;

    info!("starting faktory consumer: {}", faktory_url);

    let mut consumer = ConsumerBuilder::default();
    consumer.register("acknowledge_followers", acknowledge_followers);
    consumer.register("provide_one_time_key", provide_one_time_key);

    consumer.register("test_job", |job| {
        debug!("{:#?}", job);
        Ok(())
    });
                      
    let mut consumer = consumer
        .connect(Some("tcp://:password@localhost:7419"))
        .unwrap();

    if let Err(e) = consumer.run(&["default"]) {
        error!("worker failed: {}", e);
    }
}
