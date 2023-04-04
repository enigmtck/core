#[macro_use]
extern crate log;

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use enigmatick::{
    activity_pub::{
        sender::{send_activity, send_follower_accept},
        ApActivity, ApActor, ApInstrument, ApInstrumentType, ApInstruments, ApLike, ApNote,
        ApObject, ApSession, JoinData, Metadata,
    },
    helper::{get_local_username_from_ap_id, is_local, is_public},
    models::{
        encrypted_sessions::{EncryptedSession, NewEncryptedSession},
        followers::Follower,
        leaders::Leader,
        likes::Like,
        notes::Note,
        olm_one_time_keys::OlmOneTimeKey,
        olm_sessions::{NewOlmSession, OlmSession},
        processing_queue::{NewProcessingItem, ProcessingItem},
        profiles::Profile,
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
    schema::likes,
    signing::{Method, SignParams},
    MaybeReference,
};
use faktory::{ConsumerBuilder, Job};
use lapin::{
    options::{BasicPublishOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, ConnectionProperties,
};
use lazy_static::lazy_static;
use reqwest::{Client, StatusCode};
use serde_json::Value;
use std::{collections::HashSet, io};
use tokio::runtime::{Handle, Runtime};
use webpage::{Webpage, WebpageOptions};

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

pub fn update_timeline_items(timeline_item: NewTimelineItem) -> Vec<TimelineItem> {
    use enigmatick::schema::timeline::dsl::{ap_id, content, timeline};

    if let Ok(conn) = POOL.get() {
        match diesel::update(timeline.filter(ap_id.eq(timeline_item.ap_id)))
            .set(content.eq(timeline_item.content))
            .get_results::<TimelineItem>(&conn)
        {
            Ok(x) => x,
            Err(_) => {
                vec![]
            }
        }
    } else {
        vec![]
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

pub fn get_encrypted_session_by_profile_id_and_ap_to(
    profile_id: i32,
    ap_to: String,
) -> Option<EncryptedSession> {
    use enigmatick::schema::encrypted_sessions::dsl::{
        ap_to as a, encrypted_sessions, profile_id as p, updated_at,
    };

    if let Ok(conn) = POOL.get() {
        match encrypted_sessions
            .filter(p.eq(profile_id))
            .filter(a.eq(ap_to))
            .order(updated_at.desc())
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

pub fn get_like_by_uuid(uuid: String) -> Option<Like> {
    if let Ok(conn) = POOL.get() {
        match likes::table
            .filter(likes::uuid.eq(uuid))
            .first::<Like>(&conn)
        {
            Ok(x) => Option::from(x),
            Err(_) => Option::None,
        }
    } else {
        None
    }
}

pub fn get_one_time_key(profile_id: i32) -> Option<OlmOneTimeKey> {
    log::debug!("IN get_one_time_key");
    use enigmatick::schema::olm_one_time_keys::dsl::{
        distributed, olm_one_time_keys, profile_id as p,
    };

    if let Ok(conn) = POOL.get() {
        if let Ok(Some(otk)) = olm_one_time_keys
            .filter(p.eq(profile_id))
            .filter(distributed.eq(false))
            .first::<OlmOneTimeKey>(&conn)
            .optional()
        {
            log::debug!("OTK\n{otk:#?}");
            match diesel::update(olm_one_time_keys.find(otk.id))
                .set(distributed.eq(true))
                .get_results::<OlmOneTimeKey>(&conn)
            {
                Ok(mut x) => x.pop(),
                Err(e) => {
                    log::error!("FAILED TO RETRIEVE OTK: {e:#?}");
                    Option::None
                }
            }
        } else {
            Option::None
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
            Err(_) => Option::None,
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
            Err(_) => {
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
            Err(_) => Option::None,
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

async fn fetch_remote_note(id: String) -> Option<ApNote> {
    log::debug!("PERFORMING REMOTE LOOKUP FOR NOTE: {id}");

    let _url = id.clone();
    let _method = Method::Get;

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
    log::debug!("RUNNING send_kexinit JOB");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();

        if let Some(encrypted_session) = get_encrypted_session_by_uuid(uuid) {
            if let Some(sender) = get_profile(encrypted_session.profile_id) {
                let mut session: ApSession = encrypted_session.clone().into();
                session.id = Option::from(format!(
                    "{}/session/{}",
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
                                info!("INVITE SENT: {inbox:#?}");
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
    log::debug!("RUNNING provide_one_time_key JOB");

    // look up remote_encrypted_session with ap_id from job.args()

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();

        if let Some(session) = get_remote_encrypted_session_by_ap_id(ap_id) {
            // this is the username of the Enigmatick user who received the Invite
            log::debug!("SESSION\n{session:#?}");
            if let Some(username) = get_local_username_from_ap_id(session.ap_to.clone()) {
                log::debug!("USERNAME: {username}");
                if let Some(profile) = get_profile_by_username(username.clone()) {
                    log::debug!("PROFILE\n{profile:#?}");
                    if let Some(actor) = get_remote_actor_by_ap_id(session.attributed_to.clone()) {
                        log::debug!("ACTOR\n{actor:#?}");
                        // send Join activity with Identity and OTK to attributed_to

                        if let Some(identity_key) = profile.olm_identity_key.clone() {
                            log::debug!("IDENTITY KEY: {identity_key}");
                            if let Some(otk) = get_one_time_key(profile.id) {
                                log::debug!("IDK\n{identity_key:#?}");
                                log::debug!("OTK\n{otk:#?}");

                                let session = ApSession::from(JoinData {
                                    one_time_key: otk.key_data,
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
                                log::debug!("JOIN ACTIVITY\n{activity:#?}");

                                if create_encrypted_session(encrypted_session).is_some() {
                                    handle.block_on(async {
                                        match send_activity(activity, profile, actor.inbox).await {
                                            Ok(_) => {
                                                info!("JOIN SENT");
                                            }
                                            Err(e) => error!("ERROR SENDING JOIN: {e:#?}"),
                                        }
                                    });
                                } else {
                                    log::error!("FAILED TO SAVE ENCRYPTED SESSION");
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

fn process_join(job: Job) -> io::Result<()> {
    let ap_ids = job.args();

    debug!("RUNNING process_join JOB");

    for ap_id in ap_ids {
        if let Some(session) =
            get_remote_encrypted_session_by_ap_id(ap_id.as_str().unwrap().to_string())
        {
            // this is the username of the Enigmatick user who received the Invite
            if let Some(username) = get_local_username_from_ap_id(session.ap_to.clone()) {
                if let Some(_profile) = get_profile_by_username(username.clone()) {
                    if let Some(actor) = get_remote_actor_by_ap_id(session.attributed_to.clone()) {
                        debug!("ACTOR\n{actor:#?}");
                        //let session: ApSession = session.clone().into();

                        if let Some(item) = create_processing_item(session.clone().into()) {
                            debug!("PROCESSING ITEM\n{item:#?}");
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

fn add_to_timeline(ap_to: Option<Value>, cc: Option<Value>, timeline_item: TimelineItem) {
    if let Some(ap_to) = ap_to {
        if let Ok(to_vec) = serde_json::from_value::<Vec<String>>(ap_to.clone()) {
            //let to_vec: Vec<String> = serde_json::from_value(ap_to).unwrap();

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
        } else {
            log::error!("TO VALUE NOT A VEC: {ap_to:#?}");
        }
    }

    if let Some(cc) = cc {
        if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc.clone()) {
            //if let Ok(cc_vec) = serde_json::from_value::<Vec<String>>(cc) {
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
        } else {
            log::error!("CC VALUE NOT A VEC: {cc:#?}");
        }
    };
}

fn get_links(text: String) -> Vec<String> {
    let re = regex::Regex::new(r#"<a href="(.+?)".*?>"#).unwrap();

    re.captures_iter(&text)
        .filter(|cap| {
            !cap[0].to_string().contains("mention")
                && !cap[0].to_string().contains("u-url")
                && !cap[0].contains("hashtag")
        })
        .map(|cap| cap[1].to_string())
        .collect()
}

async fn send_to_mq(note: ApNote) {
    let mq = lapin::Connection::connect(&enigmatick::AMQP_URL, ConnectionProperties::default())
        .await
        .unwrap();
    log::debug!("SENDING TO MQ");

    let channel = mq.create_channel().await.unwrap();
    // let _queue = channel
    //     .queue_declare(
    //         "events",
    //         QueueDeclareOptions::default(),
    //         FieldTable::default(),
    //     )
    //     .await
    //     .unwrap();

    let _confirm = channel
        .basic_publish(
            "",
            "events",
            BasicPublishOptions::default(),
            &serde_json::to_vec(&note).unwrap(),
            BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
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
                        let note = fetch_remote_note(note_id).await;

                        if let Some(ap_note) = note {
                            if let Some(timeline_item) =
                                create_timeline_item((activity, ap_note.clone()).into())
                            {
                                add_to_timeline(
                                    Option::from(serde_json::to_value(ap_note.clone().to).unwrap()),
                                    {
                                        if let Some(cc) = ap_note.clone().cc {
                                            Option::from(serde_json::to_value(cc).unwrap())
                                        } else {
                                            Option::None
                                        }
                                    },
                                    timeline_item.clone(),
                                );

                                let mut ap_note: ApNote = timeline_item.into();
                                let links = get_links(ap_note.content.clone());

                                let metadata: Vec<Metadata> = {
                                    links
                                        .iter()
                                        .map(|link| {
                                            Webpage::from_url(link, WebpageOptions::default())
                                        })
                                        .filter(|metadata| metadata.is_ok())
                                        .map(|metadata| metadata.unwrap().html.meta.into())
                                        .collect()
                                };
                                ap_note.ephemeral_metadata = Some(metadata);
                                send_to_mq(ap_note).await;
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

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

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
                            let links = get_links(remote_note.content.clone());
                            log::debug!("{links:#?}");

                            let metadata: Vec<Metadata> = {
                                links
                                    .iter()
                                    .map(|link| Webpage::from_url(link, WebpageOptions::default()))
                                    .filter(|metadata| metadata.is_ok())
                                    .map(|metadata| metadata.unwrap().html.meta.into())
                                    .collect()
                            };

                            let note: ApNote = (remote_note.clone(), Some(metadata)).into();

                            if let Some(timeline_item) = create_timeline_item(note.clone().into()) {
                                add_to_timeline(
                                    remote_note.clone().ap_to,
                                    remote_note.clone().cc,
                                    timeline_item,
                                );

                                handle.block_on(async {
                                    send_to_mq(note.clone()).await;
                                });
                            }
                        } else if remote_note.kind == "EncryptedNote" {
                            // need to resolve ap_to to a profile_id for the command below
                            debug!("adding to processing queue");

                            if let Some(ap_to) = remote_note.clone().ap_to {
                                let to_vec: Vec<String> = {
                                    match serde_json::from_value(ap_to) {
                                        Ok(x) => x,
                                        Err(_e) => vec![],
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

fn get_note_by_uuid(uuid: String) -> Option<Note> {
    use enigmatick::schema::notes::dsl::{notes, uuid as u};

    if let Ok(conn) = POOL.get() {
        match notes.filter(u.eq(uuid)).first::<Note>(&conn).optional() {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

fn handle_note(note: &mut Note, inboxes: &mut HashSet<String>, sender: Profile) -> Option<ApNote> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    if let Ok(recipients) = serde_json::from_value::<Vec<String>>(note.clone().ap_to) {
        for recipient in recipients {
            // check if this is the special Public recipient
            if is_public(recipient.clone()) {
                // if it is, get all the inboxes for this sender's followers
                inboxes.extend(get_follower_inboxes(sender.clone()));

                // add the special followers address for the sending profile to the
                // note's cc field
                if let Some(cc) = note.clone().cc {
                    let mut cc: Vec<String> = serde_json::from_value(cc).unwrap();
                    if let Some(followers) = ApActor::from(sender.clone()).followers {
                        cc.push(followers)
                    };
                    note.cc = Option::from(serde_json::to_value(cc).unwrap());
                } else {
                    note.cc = Option::from(
                        serde_json::to_value(vec![ApActor::from(sender.clone()).followers])
                            .unwrap(),
                    );
                }

                update_note_cc(note.clone());
            } else if let Some(receiver) =
                handle.block_on(async { get_actor(sender.clone(), recipient.clone()).await })
            {
                inboxes.insert(receiver.0.inbox);
            }
        }
    }

    if let Some(_actor) = get_remote_actor_by_ap_id(note.clone().attributed_to) {
        if let Some(timeline_item) = create_timeline_item(ApNote::from(note.clone()).into()) {
            add_to_timeline(
                Option::from(note.clone().ap_to),
                note.clone().cc,
                timeline_item,
            );
        }
    }

    Some(note.clone().into())
}

pub fn create_olm_session(session: NewOlmSession) -> Option<OlmSession> {
    use enigmatick::schema::olm_sessions;

    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(olm_sessions::table)
            .values(&session)
            .get_result::<OlmSession>(&conn)
            .optional()
        {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        Option::None
    }
}

pub fn update_olm_session(
    uuid: String,
    session_data: String,
    session_hash: String,
) -> Option<OlmSession> {
    use enigmatick::schema::olm_sessions;

    if let Ok(conn) = POOL.get() {
        match diesel::update(olm_sessions::table.filter(olm_sessions::uuid.eq(uuid)))
            .set((
                olm_sessions::session_data.eq(session_data),
                olm_sessions::session_hash.eq(session_hash),
            ))
            .get_result::<OlmSession>(&conn)
            .optional()
        {
            Ok(x) => x,
            Err(_) => Option::None,
        }
    } else {
        None
    }
}

fn handle_encrypted_note(
    note: &mut Note,
    inboxes: &mut HashSet<String>,
    sender: Profile,
) -> Option<ApNote> {
    debug!("ENCRYPTED NOTE\n{note:#?}");

    fn do_it(
        instrument: ApInstrument,
        inboxes: &mut HashSet<String>,
        note: &mut Note,
        sender: Profile,
    ) {
        let rt = Runtime::new().unwrap();
        let handle = rt.handle();

        if let ApInstrumentType::OlmSession = instrument.kind {
            if let Ok(to) = serde_json::from_value::<Vec<String>>(note.ap_to.clone()) {
                // save encrypted session
                if let Some(encrypted_session) =
                    get_encrypted_session_by_profile_id_and_ap_to(sender.id, to[0].clone())
                {
                    if let (Some(uuid), Some(hash), Some(content)) = (
                        instrument.clone().uuid,
                        instrument.clone().hash,
                        instrument.clone().content,
                    ) {
                        log::debug!("FOUND UUID - UPDATING EXISTING SESSION");
                        if let Some(_session) = update_olm_session(uuid, content, hash) {
                            if let Some(receiver) = handle
                                .block_on(async { get_actor(sender.clone(), to[0].clone()).await })
                            {
                                inboxes.insert(receiver.0.inbox);
                            }
                        }
                    } else {
                        log::debug!("NO UUID - CREATING NEW SESSION");
                        if let Some(_session) =
                            create_olm_session((instrument, encrypted_session.id).into())
                        {
                            if let Some(receiver) = handle
                                .block_on(async { get_actor(sender.clone(), to[0].clone()).await })
                            {
                                inboxes.insert(receiver.0.inbox);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(instrument) = &note.instrument {
        if let Ok(instruments) = serde_json::from_value::<ApInstruments>(instrument.clone()) {
            match instruments {
                ApInstruments::Multiple(instruments) => {
                    for instrument in instruments {
                        do_it(instrument, inboxes, note, sender.clone());
                    }
                }
                ApInstruments::Single(instrument) => {
                    do_it(instrument, inboxes, note, sender);
                }
                _ => (),
            }

            Some(note.clone().into())
        } else {
            error!("INVALID INSTRUMENT\n{instrument:#?}");
            Option::None
        }
    } else {
        error!("NO instrument");
        Option::None
    }
}

fn retrieve_context(job: Job) -> io::Result<()> {
    debug!("running retrieve_context job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for ap_id in job.args() {
        let ap_id = ap_id.as_str().unwrap().to_string();
        handle.block_on(async {
            if let Some(note) = fetch_remote_note(ap_id.to_string()).await {
                log::debug!("REPLIES\n{:#?}", note.replies);

                if let Some(replies) = note.replies {
                    if let Some(MaybeReference::Actual(first)) = replies.first {}
                }
            }
        });
    }

    Ok(())
}

fn process_outbound_note(job: Job) -> io::Result<()> {
    debug!("running process_outbound_note job");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        let uuid = uuid.as_str().unwrap().to_string();

        if let Some(mut note) = get_note_by_uuid(uuid) {
            // this is the profile where the note was posted to the outbox
            if let Some(sender) = get_profile(note.profile_id) {
                let mut inboxes: HashSet<String> = HashSet::new();

                let create = match note.kind.as_str() {
                    "Note" => {
                        handle_note(&mut note, &mut inboxes, sender.clone()).map(ApActivity::from)
                    }
                    "EncryptedNote" => {
                        handle_encrypted_note(&mut note, &mut inboxes, sender.clone())
                            .map(ApActivity::from)
                    }
                    _ => None,
                };

                if let Some(create) = create {
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
        }
    }

    Ok(())
}

fn update_timeline_record(job: Job) -> io::Result<()> {
    use enigmatick::schema::remote_notes::dsl::{ap_id as rn_id, remote_notes};

    debug!("running update_timeline_record job");

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
                            update_timeline_items(remote_note.clone().into());
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

fn send_like(job: Job) -> io::Result<()> {
    debug!("SENDING LIKE");

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    for uuid in job.args() {
        if let Some(like) = get_like_by_uuid(uuid.as_str().unwrap().to_string()) {
            if let Some(profile_id) = like.profile_id {
                if let Some(sender) = get_profile(profile_id) {
                    if let Some(actor) = get_remote_actor_by_ap_id(like.ap_to.clone()) {
                        let ap_like = ApLike::from(like.clone());
                        let url = actor.inbox;
                        let body = Option::from(serde_json::to_string(&ap_like).unwrap());
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
                                    debug!("SEND SUCCESSFUL: {url}\n{text}");
                                }
                            }
                        })
                    }
                }
            }
        }
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
    consumer.register("update_timeline_record", update_timeline_record);
    consumer.register("retrieve_context", retrieve_context);
    consumer.register("send_like", send_like);

    consumer.register("test_job", |job| {
        debug!("{:#?}", job);
        Ok(())
    });

    let mut consumer = consumer.connect(Some(faktory_url)).unwrap();

    if let Err(e) = consumer.run(&["default"]) {
        error!("worker failed: {}", e);
    }
}
