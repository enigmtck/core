#[macro_use]
extern crate log;

use enigmatick::{
    activity_pub::{
        sender::{send_activity, send_follower_accept},
        ApActivity, ApActor, ApBasicContentType, ApInstrument, ApNote, ApObject, ApSession,
        JoinData,
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
        remote_announces::RemoteAnnounce,
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

pub fn get_encrypted_session_by_uuid(uuid: String) -> Option<EncryptedSession> {
    use enigmatick::schema::encrypted_sessions::dsl::{encrypted_sessions, uuid as u};

    log::debug!("looking for encrypted_session_by_uuid: {:#?}", uuid);
    if let Ok(conn) = POOL.get() {
        match encrypted_sessions
            .filter(u.eq(uuid))
            .first::<EncryptedSession>(&conn)
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

pub fn get_profile_by_ap_id(ap_id: String) -> Option<Profile> {
    let server_url = (*enigmatick::SERVER_URL).clone();

    let id_re = regex::Regex::new(&format!(r#"{server_url}/user/([a-zA-Z0-9_]+)"#)).unwrap();

    if let Some(captures) = id_re.captures(&ap_id) {
        log::debug!("captures\n{captures:#?}");

        if captures.len() == 2 {
            if let Some(username) = captures.get(1) {
                get_profile_by_username(username.as_str().to_string())
            } else {
                Option::None
            }
        } else {
            Option::None
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

pub fn get_leader_by_actor_ap_id_and_profile(ap_id: String, profile_id: i32) -> Option<Leader> {
    use enigmatick::schema::leaders::dsl::{leader_ap_id, leaders, profile_id as pid};

    if let Ok(conn) = POOL.get() {
        match leaders
            .filter(leader_ap_id.eq(ap_id))
            .filter(pid.eq(profile_id))
            .first::<Leader>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn create_remote_actor(actor: NewRemoteActor) -> Option<RemoteActor> {
    use enigmatick::schema::remote_actors;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(remote_actors::table)
            .values(&actor)
            .get_result::<RemoteActor>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::debug!("database failure: {:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}

pub async fn get_actor(profile: Profile, id: String) -> Option<(RemoteActor, Option<Leader>)> {
    match get_remote_actor_by_ap_id(id.clone()) {
        Some(remote_actor) => {
            log::debug!("actor retrieved from storage");

            Option::from((
                remote_actor,
                get_leader_by_actor_ap_id_and_profile(id, profile.id),
            ))
        }
        None => {
            log::debug!("performing remote lookup for actor");

            let url = id.clone();
            let body = Option::None;
            let method = Method::Get;

            let signature = enigmatick::signing::sign(SignParams {
                profile,
                url,
                body,
                method,
            });

            let client = Client::new();
            match client
                .get(&id)
                .header("Signature", &signature.signature)
                .header("Date", signature.date)
                .header(
                    "Accept",
                    "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
                )
                .send()
                .await
            {
                Ok(resp) => match resp.status() {
                    StatusCode::ACCEPTED | StatusCode::OK => {
                        let actor: ApActor = resp.json().await.unwrap();
                        create_remote_actor(NewRemoteActor::from(actor)).map(|a| (a, Option::None))
                    }
                    StatusCode::GONE => {
                        log::debug!("GONE: {:#?}", resp.status());
                        Option::None
                    }
                    _ => {
                        log::debug!("STATUS: {:#?}", resp.status());
                        Option::None
                    }
                },
                Err(e) => {
                    log::debug!("{:#?}", e);
                    Option::None
                }
            }
        }
    }
}

pub fn get_remote_announce_by_ap_id(ap_id: String) -> Option<RemoteAnnounce> {
    use enigmatick::schema::remote_announces::dsl::{ap_id as a, remote_announces};

    if let Ok(conn) = POOL.get() {
        match remote_announces
            .filter(a.eq(ap_id))
            .first::<RemoteAnnounce>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn get_profile(id: i32) -> Option<Profile> {
    use enigmatick::schema::profiles::dsl::profiles;

    if let Ok(conn) = POOL.get() {
        match profiles.find(id).first::<Profile>(&conn) {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
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

pub fn get_followers_by_profile_id(profile_id: i32) -> Vec<Follower> {
    use enigmatick::schema::followers::dsl::{created_at, followers, profile_id as pid};

    if let Ok(conn) = POOL.get() {
        match followers
            .filter(pid.eq(profile_id))
            .order_by(created_at.desc())
            .get_results::<Follower>(&conn)
        {
            Ok(x) => x,
            Err(_) => vec![],
        }
    } else {
        vec![]
    }
}

pub fn get_follower_profiles_by_endpoint(
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
                vec![]
            }
        }
    } else {
        vec![]
    }
}

pub fn get_leader_by_endpoint(endpoint: String) -> Option<(RemoteActor, Leader)> {
    use enigmatick::schema::leaders::dsl::{leader_ap_id, leaders};
    use enigmatick::schema::remote_actors::dsl::{ap_id as ra_apid, followers, remote_actors};

    if let Ok(conn) = POOL.get() {
        match remote_actors
            .inner_join(leaders.on(leader_ap_id.eq(ra_apid)))
            .filter(followers.eq(endpoint))
            .first::<(RemoteActor, Leader)>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(e) => Option::None,
        }
    } else {
        Option::None
    }
}

fn get_follower_inboxes(profile: Profile) -> HashSet<String> {
    let mut inboxes: HashSet<String> = HashSet::new();

    for follower in get_followers_by_profile_id(profile.id) {
        if let Some(actor) = get_remote_actor_by_ap_id(follower.actor) {
            inboxes.insert(actor.inbox);
        }
    }

    inboxes
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

pub fn update_note_cc(note: Note) -> Option<Note> {
    use enigmatick::schema::notes::dsl::{cc, notes};

    if let Ok(conn) = POOL.get() {
        match diesel::update(notes.find(note.id))
            .set(cc.eq(note.cc))
            .get_result::<Note>(&conn)
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

async fn lookup_remote_note(id: String) -> Option<ApNote> {
    log::debug!("performing remote lookup for note");

    let url = id.clone();
    let method = Method::Get;

    let client = Client::new();
    match client
        .get(&id)
        .header(
            "Accept",
            "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
        )
        .send()
        .await
    {
        Ok(resp) => match resp.status() {
            StatusCode::ACCEPTED | StatusCode::OK => match resp.json().await {
                Ok(ApObject::Note(note)) => Option::from(note),
                Err(e) => {
                    log::error!("remote note decode error: {e:#?}");
                    Option::None
                }
                _ => Option::None,
            },
            StatusCode::GONE => {
                log::debug!("GONE: {:#?}", resp.status());
                Option::None
            }
            _ => {
                log::debug!("STATUS: {:#?}", resp.status());
                Option::None
            }
        },
        Err(e) => {
            log::debug!("{:#?}", e);
            Option::None
        }
    }
}

fn send_kexinit(job: Job) -> io::Result<()> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();

        if let Some(encrypted_session) = get_encrypted_session_by_uuid(uuid) {
            if let Some(sender) = get_profile(encrypted_session.profile_id) {
                let mut session: ApSession = encrypted_session.clone().into();
                session.id = Option::from(format!(
                    "{}/encrypted-sessions/{}",
                    *enigmatick::SERVER_URL,
                    encrypted_session.uuid
                ));

                let mut inbox = Option::<String>::None;

                if is_local(session.to.clone()) {
                    if let Some(username) = get_local_username_from_ap_id(session.to.clone()) {
                        if let Some(profile) = get_profile_by_username(username) {
                            inbox = Option::from(ApActor::from(profile).inbox);
                        }
                    }
                } else if let Some(actor) = get_remote_actor_by_ap_id(session.to.clone()) {
                    inbox = Option::from(actor.inbox);
                }

                if let Some(inbox) = inbox {
                    let activity = ApActivity::from(session);
                    handle.block_on(async {
                        match send_activity(activity, sender, inbox.clone()).await {
                            Ok(_) => {
                                info!("join sent: {:#?}", inbox);
                            }
                            Err(e) => error!("error: {:#?}", e),
                        }
                    });
                }
            }
        }
    }

    Ok(())
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

fn add_to_timeline(ap_to: Option<Value>, cc: Option<Value>, timeline_item: TimelineItem) {
    if let Some(ap_to) = ap_to {
        let to_vec: Vec<String> = serde_json::from_value(ap_to).unwrap();

        for to in to_vec {
            create_timeline_item_to((timeline_item.clone(), to.clone()).into());

            if get_leader_by_endpoint(to.clone()).is_some() {
                for follower in get_follower_profiles_by_endpoint(to) {
                    if let Some(follower) = follower.2 {
                        log::debug!("adding to for {}", follower.username);

                        let follower =
                            format!("{}/user/{}", &*enigmatick::SERVER_URL, follower.username);
                        create_timeline_item_to((timeline_item.clone(), follower).into());
                    }
                }
            }
        }
    }

    if let Some(cc) = cc {
        if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc.clone()) {
            for cc in cc_vec {
                create_timeline_item_cc((timeline_item.clone(), cc.clone()).into());

                if get_leader_by_endpoint(cc.clone()).is_some() {
                    for follower in get_follower_profiles_by_endpoint(cc) {
                        if let Some(follower) = follower.2 {
                            log::debug!("adding cc for {}", follower.username);

                            let follower =
                                format!("{}/user/{}", &*enigmatick::SERVER_URL, follower.username);
                            create_timeline_item_cc((timeline_item.clone(), follower).into());
                        }
                    }
                }
            }
        }
    };
}

fn process_announce(job: Job) -> io::Result<()> {
    debug!("running process_announce job");

    let ap_ids = job.args();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in ap_ids {
        let ap_id = ap_id.as_str().unwrap().to_string();
        debug!("looking for ap_id: {}", ap_id);

        let announce = get_remote_announce_by_ap_id(ap_id);

        if let Some(announce) = announce {
            let activity: ApActivity = announce.clone().into();

            if activity.kind == "Announce".into() {
                if let ApObject::Plain(note_id) = activity.clone().object {
                    handle.block_on(async {
                        let note = lookup_remote_note(note_id).await;

                        if let Some(remote_note) = note {
                            if let Some(timeline_item) =
                                create_timeline_item((activity, remote_note.clone()).into())
                            {
                                add_to_timeline(
                                    Option::from(
                                        serde_json::to_value(remote_note.clone().to).unwrap(),
                                    ),
                                    Option::from(
                                        serde_json::to_value(remote_note.cc.unwrap()).unwrap(),
                                    ),
                                    timeline_item,
                                );
                            }
                        }
                    });
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
                            if let Some(timeline_item) =
                                create_timeline_item(remote_note.clone().into())
                            {
                                add_to_timeline(
                                    remote_note.clone().ap_to,
                                    remote_note.clone().cc,
                                    timeline_item,
                                );
                            }
                        } else if remote_note.kind == "EncryptedNote" {
                            // need to resolve ap_to to a profile_id for the command below
                            debug!("adding to processing queue");

                            if let Some(ap_to) = remote_note.clone().ap_to {
                                let to_vec: Vec<String> = {
                                    match serde_json::from_value(ap_to) {
                                        Ok(x) => x,
                                        Err(e) => vec![],
                                    }
                                };

                                for ap_id in to_vec {
                                    if let Some(profile) = get_profile_by_ap_id(ap_id) {
                                        create_processing_item(
                                            (remote_note.clone(), profile.id).into(),
                                        );
                                    }
                                }
                            }
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

fn process_outbound_note(job: Job) -> io::Result<()> {
    use enigmatick::schema::notes::dsl::{notes, uuid as u};

    debug!("running process_outbound_note job");

    let uuids = job.args();

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match POOL.get() {
        Ok(conn) => {
            for uuid in uuids {
                let uuid = uuid.as_str().unwrap().to_string();
                debug!("looking for uuid: {}", uuid);

                match notes.filter(u.eq(uuid)).first::<Note>(&conn) {
                    Ok(mut note) => {
                        let mut inboxes: HashSet<String> = HashSet::new();

                        // this is the profile where the note was posted to the outbox
                        if let Some(sender) = get_profile(note.profile_id) {
                            let sender = sender.clone();

                            if let Ok(recipients) =
                                serde_json::from_value::<Vec<String>>(note.clone().ap_to)
                            {
                                for recipient in recipients {
                                    // check if this is the special Public recipient
                                    if is_public(recipient.clone()) {
                                        // if it is, get all the inboxes for this sender's followers
                                        inboxes.extend(get_follower_inboxes(sender.clone()));

                                        // add the special followers address for the sending profile to the
                                        // note's cc field
                                        if let Some(cc) = note.clone().cc {
                                            let mut cc: Vec<String> =
                                                serde_json::from_value(cc).unwrap();
                                            cc.push(ApActor::from(sender.clone()).followers);
                                            note.cc =
                                                Option::from(serde_json::to_value(cc).unwrap());
                                        } else {
                                            note.cc = Option::from(
                                                serde_json::to_value(vec![
                                                    ApActor::from(sender.clone()).followers,
                                                ])
                                                .unwrap(),
                                            );
                                        }

                                        update_note_cc(note.clone());
                                    // } else if is_local(recipient.clone()) {
                                    //     if let Some(username) =
                                    //         get_local_username_from_ap_id(recipient.to_string())
                                    //     {
                                    //         if let Some(profile) = get_profile_by_username(username) {
                                    //             inboxes.insert(ApActor::from(profile).inbox);
                                    //         }
                                    //     }
                                    } else if let Some(receiver) = handle.block_on(async {
                                        get_actor(sender.clone(), recipient.clone()).await
                                    }) {
                                        inboxes.insert(receiver.0.inbox);
                                    }
                                }
                            }

                            if let Some(actor) =
                                get_remote_actor_by_ap_id(note.clone().attributed_to)
                            {
                                if let Some(timeline_item) =
                                    create_timeline_item(ApNote::from(note.clone()).into())
                                {
                                    add_to_timeline(
                                        Option::from(note.clone().ap_to),
                                        note.clone().cc,
                                        timeline_item,
                                    );
                                }
                            }

                            let create = ApActivity::from(ApNote::from(note.clone()));

                            for url in inboxes {
                                let body = Option::from(serde_json::to_string(&create).unwrap());
                                let method = Method::Post;

                                let signature = enigmatick::signing::sign(SignParams {
                                    profile: sender.clone(),
                                    url: url.clone(),
                                    body: body.clone(),
                                    method,
                                });

                                let client = Client::new()
                                    .post(&url)
                                    .header("Date", signature.date)
                                    .header("Digest", signature.digest.unwrap())
                                    .header("Signature", &signature.signature)
                                    .header(
                                        "Content-Type",
                                        "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"",
                                    )
                                    .body(body.unwrap());

                                handle.block_on(async {
                                    if let Ok(resp) = client.send().await {
                                        if let Ok(text) = resp.text().await {
                                            debug!("send successful to: {}\n{}", url, text);
                                        }
                                    }
                                })
                            }
                        }
                    }
                    Err(e) => error!("error: {e:#?}"),
                }
            }
        }
        Err(e) => error!("error: {e:#?}"),
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
    consumer.register("process_outbound_note", process_outbound_note);
    consumer.register("process_announce", process_announce);
    consumer.register("send_kexinit", send_kexinit);

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
