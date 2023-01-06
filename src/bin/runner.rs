#[macro_use]
extern crate log;

use enigmatick::{
    activity_pub::sender::send_follower_accept,
    helper::get_local_username_from_ap_id,
    models::{
        followers::Follower,
        profiles::Profile,
        remote_actors::RemoteActor,
        remote_encrypted_sessions::RemoteEncryptedSession,
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

fn provide_one_time_key(job: Job) -> io::Result<()> {
    // look up remote_encrypted_session with ap_id from job.args()

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();

        if let Some(session) = get_remote_encrypted_session_by_ap_id(ap_id) {
            if let Some(username) = get_local_username_from_ap_id(session.ap_to) {
                if let Some(profile) = get_profile_by_username(username) {
                    // send Join activity with Identity and OTK
                    if let Some(keystore) = profile.keystore {
                        let identity_key = keystore.olm_identity_public_key;
                        
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
