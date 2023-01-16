#[macro_use]
extern crate log;

use enigmatick::{
    activity_pub::{
        retriever,
        sender::{send_activity, send_follower_accept},
        ApActivity, ApBasicContentType, ApInstrument, ApObject, ApObjectType, ApSession, JoinData,
    },
    db::jsonb_set,
    helper::get_local_username_from_ap_id,
    models::{
        encrypted_sessions::{EncryptedSession, NewEncryptedSession},
        followers::Follower,
        processing_queue::{NewProcessingItem, ProcessingItem},
        profiles::{KeyStore, Profile},
        remote_activities::{NewRemoteActivity, RemoteActivity},
        remote_actors::RemoteActor,
        remote_encrypted_sessions::RemoteEncryptedSession,
        remote_notes::RemoteNote,
        timeline::{
            NewTimelineItem, NewTimelineItemCc, NewTimelineItemTo, TimelineItem, TimelineItemCc,
            TimelineItemTo,
        },
    },
};

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use faktory::{ConsumerBuilder, Job};
use lazy_static::lazy_static;
use std::{collections::HashMap, io};
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
            .get_result::<RemoteActivity>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn create_processing_item(processing_item: NewProcessingItem) -> Option<ProcessingItem> {
    use enigmatick::schema::processing_queue;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(processing_queue::table)
            .values(&processing_item)
            .get_result::<ProcessingItem>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn create_timeline_item_to(timeline_item_to: NewTimelineItemTo) -> Option<TimelineItemTo> {
    use enigmatick::schema::timeline_to;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(timeline_to::table)
            .values(&timeline_item_to)
            .get_result::<TimelineItemTo>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn create_timeline_item_cc(timeline_item_cc: NewTimelineItemCc) -> Option<TimelineItemCc> {
    use enigmatick::schema::timeline_cc;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(timeline_cc::table)
            .values(&timeline_item_cc)
            .get_result::<TimelineItemCc>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn create_timeline_item(timeline_item: NewTimelineItem) -> Option<TimelineItem> {
    use enigmatick::schema::timeline;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(timeline::table)
            .values(&timeline_item)
            .get_result::<TimelineItem>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn create_encrypted_session(
    encrypted_session: NewEncryptedSession,
) -> Option<EncryptedSession> {
    use enigmatick::schema::encrypted_sessions;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(encrypted_sessions::table)
            .values(&encrypted_session)
            .get_result::<EncryptedSession>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn get_remote_encrypted_session_by_ap_id(apid: String) -> Option<RemoteEncryptedSession> {
    use enigmatick::schema::remote_encrypted_sessions::dsl::{ap_id, remote_encrypted_sessions};

    log::debug!("looking for remote_encrypted_session_by_ap_id: {:#?}", apid);
    if let Ok(conn) = POOL.get() {
        match remote_encrypted_sessions
            .filter(ap_id.eq(apid))
            .first::<RemoteEncryptedSession>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
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
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn get_remote_actor_by_ap_id(apid: String) -> Option<RemoteActor> {
    use enigmatick::schema::remote_actors::dsl::{ap_id, remote_actors};

    if let Ok(conn) = POOL.get() {
        match remote_actors
            .filter(ap_id.eq(apid))
            .first::<RemoteActor>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn update_otk_by_username(username: String, keystore: KeyStore) -> Option<Profile> {
    use enigmatick::schema::profiles::dsl::{keystore as k, profiles, username as u};

    if let Ok(conn) = POOL.get() {
        match diesel::update(profiles.filter(u.eq(username)))
            .set(k.eq(jsonb_set(
                k,
                vec![String::from("olm_one_time_keys")],
                serde_json::to_value(keystore.olm_one_time_keys).unwrap(),
            )))
            .get_result::<Profile>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn update_external_one_time_keys_by_username(
    username: String,
    keystore: KeyStore,
) -> Option<Profile> {
    use enigmatick::schema::profiles::dsl::{keystore as k, profiles, username as u};

    if let Ok(conn) = POOL.get() {
        match diesel::update(profiles.filter(u.eq(username)))
            .set(k.eq(jsonb_set(
                k,
                vec![String::from("olm_external_one_time_keys")],
                serde_json::to_value(keystore.olm_external_one_time_keys).unwrap(),
            )))
            .get_result::<Profile>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub fn update_external_identity_keys_by_username(
    username: String,
    keystore: KeyStore,
) -> Option<Profile> {
    use enigmatick::schema::profiles::dsl::{keystore as k, profiles, username as u};

    if let Ok(conn) = POOL.get() {
        match diesel::update(profiles.filter(u.eq(username)))
            .set(k.eq(jsonb_set(
                k,
                vec![String::from("olm_external_identity_keys")],
                serde_json::to_value(keystore.olm_external_identity_keys).unwrap(),
            )))
            .get_result::<Profile>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
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
                                        reference: session.ap_id,
                                    });

                                    let activity = ApActivity::from(session.clone());
                                    let encrypted_session: NewEncryptedSession =
                                        (session.clone(), profile.id).into();

                                    // this activity should be saved so that the id makes sense
                                    // but it's not right now
                                    log::debug!("activity\n{:#?}", activity);

                                    if create_encrypted_session(encrypted_session).is_some() {
                                        handle.block_on(async {
                                            match send_activity(activity, profile, actor.inbox)
                                                .await
                                            {
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
                                            match send_follower_accept(
                                                ap_id,
                                                profile,
                                                actor.clone(),
                                            )
                                            .await
                                            {
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

fn process_join(job: Job) -> io::Result<()> {
    let ap_ids = job.args();

    debug!("running process_join job: {:#?}", ap_ids);

    for ap_id in ap_ids {
        if let Some(session) =
            get_remote_encrypted_session_by_ap_id(ap_id.as_str().unwrap().to_string())
        {
            // this is the username of the Enigmatick user who received the Invite
            if let Some(username) = get_local_username_from_ap_id(session.ap_to.clone()) {
                if let Some(profile) = get_profile_by_username(username.clone()) {
                    if let Some(actor) = get_remote_actor_by_ap_id(session.attributed_to.clone()) {
                        debug!("in actor: {:#?}", actor);
                        let session: ApSession = session.into();

                        debug!("session: {:#?}", session);

                        let mut keystore = profile.keystore.clone();
                        if let (Some(mut ext_otks), Some(mut ext_idks)) = (
                            keystore.olm_external_one_time_keys,
                            keystore.olm_external_identity_keys.clone(),
                        ) {
                            debug!("in ext_otks: {:#?}", (ext_otks.clone(), ext_idks.clone()));
                            fn add_key(
                                ap_id: String,
                                obj: ApObject,
                                mut ext_otks: HashMap<String, String>,
                                mut ext_idks: HashMap<String, String>,
                            ) -> (HashMap<String, String>, HashMap<String, String>)
                            {
                                debug!("in add_key: {:#?}", obj);
                                debug!("in add_key: {:#?}", (ext_otks.clone(), ext_idks.clone()));

                                if let ApObject::Basic(instrument) = obj {
                                    match instrument.kind {
                                        ApBasicContentType::SessionKey => {
                                            ext_otks.insert(ap_id, instrument.content);
                                        }
                                        ApBasicContentType::IdentityKey => {
                                            ext_idks.insert(ap_id, instrument.content);
                                        }
                                    }
                                }

                                (ext_otks, ext_idks)
                            }

                            match session.instrument {
                                ApInstrument::Multiple(x) => {
                                    debug!("x is: {:#?}", x);
                                    for obj in x {
                                        (ext_otks, ext_idks) = add_key(
                                            actor.ap_id.clone(),
                                            obj,
                                            ext_otks.clone(),
                                            ext_idks.clone(),
                                        );
                                    }
                                }
                                ApInstrument::Single(x) => {
                                    debug!("wtf is: {:#?}", x);
                                    (ext_otks, ext_idks) = add_key(
                                        actor.ap_id.clone(),
                                        *x,
                                        ext_otks.clone(),
                                        ext_idks.clone(),
                                    );
                                }
                                _ => {}
                            }

                            keystore.olm_external_one_time_keys = Option::from(ext_otks);
                            keystore.olm_external_identity_keys = Option::from(ext_idks);

                            update_external_one_time_keys_by_username(
                                username.clone(),
                                keystore.clone(),
                            );
                            update_external_identity_keys_by_username(username, keystore);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn process_remote_note(job: Job) -> io::Result<()> {
    use enigmatick::schema::remote_notes::dsl::{ap_id as rn_id, remote_notes};

    debug!("running process_remote_note job");

    let ap_ids = job.args();

    match POOL.get() {
        Ok(conn) => {
            for ap_id in ap_ids {
                let ap_id = ap_id.as_str().unwrap().to_string();
                debug!("looking for ap_id: {}", ap_id);

                match remote_notes
                    .filter(rn_id.eq(ap_id))
                    .first::<RemoteNote>(&conn)
                {
                    Ok(remote_note) => {
                        if remote_note.kind == "Note" {
                            if let Some(actor) =
                                get_remote_actor_by_ap_id(remote_note.clone().attributed_to)
                            {
                                if let Some(timeline_item) =
                                    create_timeline_item((remote_note.clone(), actor.id).into())
                                {
                                    if let Some(ap_to) = remote_note.clone().ap_to {
                                        let to_vec: Vec<String> =
                                            serde_json::from_value(ap_to).unwrap();

                                        for to in to_vec {
                                            create_timeline_item_to(
                                                (timeline_item.clone(), to).into(),
                                            );
                                        }
                                    }

                                    if let Some(cc) = remote_note.clone().cc {
                                        let cc_vec: Vec<String> =
                                            serde_json::from_value(cc).unwrap();

                                        for cc in cc_vec {
                                            create_timeline_item_cc(
                                                (timeline_item.clone(), cc).into(),
                                            );
                                        }
                                    };
                                }
                            }
                        } else if remote_note.kind == "EncryptedNote" {
                            debug!("adding to processing queue");
                            create_processing_item(remote_note.into());
                        }
                    }
                    Err(e) => error!("error: {:#?}", e),
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
    consumer.register("process_remote_note", process_remote_note);
    consumer.register("process_join", process_join);

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
