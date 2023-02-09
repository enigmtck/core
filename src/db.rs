use crate::models::encrypted_sessions::{EncryptedSession, NewEncryptedSession};
use crate::models::followers::{Follower, NewFollower};
use crate::models::leaders::{Leader, NewLeader};
use crate::models::notes::{NewNote, Note};
use crate::models::profiles::{NewProfile, Profile};
use crate::models::remote_activities::{NewRemoteActivity, RemoteActivity};
use crate::models::remote_actors::{NewRemoteActor, RemoteActor};
use crate::models::remote_encrypted_sessions::{NewRemoteEncryptedSession, RemoteEncryptedSession};
use crate::models::remote_notes::{NewRemoteNote, RemoteNote};
use crate::schema;
use diesel::prelude::*;
use diesel::sql_types::{Array, Jsonb, Text};
use rocket_sync_db_pools::{database, diesel};

// this is a reference to the value in Rocket.toml, not the actual
// database name
#[database("enigmatick")]
pub struct Db(diesel::PgConnection);

sql_function! {
    fn jsonb_set(target: Jsonb, path: Array<Text>, new_value: Jsonb) -> Jsonb
}

pub async fn get_leader_by_profile_id_and_ap_id(
    conn: &Db,
    profile_id: i32,
    leader_ap_id: String,
) -> Option<Leader> {
    use schema::leaders::dsl::{leader_ap_id as lid, leaders, profile_id as pid};

    match conn
        .run(move |c| {
            leaders
                .filter(pid.eq(profile_id).and(lid.eq(leader_ap_id)))
                .first::<Leader>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn update_password_by_username(
    conn: &Db,
    username: String,
    password: String,
) -> Option<Profile> {
    use schema::profiles::dsl::{password as p, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(p.eq(password))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_avatar_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Profile> {
    use schema::profiles::dsl::{avatar_filename as a, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(a.eq(filename))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_banner_by_username(
    conn: &Db,
    username: String,
    filename: String,
) -> Option<Profile> {
    use schema::profiles::dsl::{banner_filename as b, profiles, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(b.eq(filename))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_summary_by_username(
    conn: &Db,
    username: String,
    summary: String,
) -> Option<Profile> {
    use schema::profiles::dsl::{profiles, summary as s, username as u};

    match conn
        .run(move |c| {
            diesel::update(profiles.filter(u.eq(username)))
                .set(s.eq(summary))
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn update_leader_by_uuid(
    conn: &Db,
    leader_uuid: String,
    accept_ap_id: String,
) -> Option<Leader> {
    use schema::leaders::dsl::{accept_ap_id as aapid, accepted, leaders, uuid};

    match conn
        .run(move |c| {
            diesel::update(leaders.filter(uuid.eq(leader_uuid)))
                .set((aapid.eq(accept_ap_id), accepted.eq(true)))
                .get_result::<Leader>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn create_remote_encrypted_session(
    conn: &Db,
    remote_encrypted_session: NewRemoteEncryptedSession,
) -> Option<RemoteEncryptedSession> {
    use schema::remote_encrypted_sessions;

    match conn
        .run(move |c| {
            diesel::insert_into(remote_encrypted_sessions::table)
                .values(&remote_encrypted_session)
                .get_result::<RemoteEncryptedSession>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("{:#?}", e);
            Option::None
        }
    }
}

pub async fn get_remote_encrypted_session_by_ap_id(
    conn: &Db,
    apid: String,
) -> Option<RemoteEncryptedSession> {
    use self::schema::remote_encrypted_sessions::dsl::{ap_id, remote_encrypted_sessions};

    match conn
        .run(move |c| {
            remote_encrypted_sessions
                .filter(ap_id.eq(apid))
                .first::<RemoteEncryptedSession>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn create_encrypted_session(
    conn: &Db,
    encrypted_session: NewEncryptedSession,
) -> Option<EncryptedSession> {
    use schema::encrypted_sessions;

    match conn
        .run(move |c| {
            diesel::insert_into(encrypted_sessions::table)
                .values(&encrypted_session)
                .get_result::<EncryptedSession>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("{:#?}", e);
            Option::None
        }
    }
}

pub async fn get_encrypted_sessions_by_profile_id(conn: &Db, id: i32) -> Vec<EncryptedSession> {
    use self::schema::encrypted_sessions::dsl::{encrypted_sessions, profile_id};

    match conn
        .run(move |c| {
            encrypted_sessions
                .filter(profile_id.eq(id))
                .get_results::<EncryptedSession>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}

pub async fn create_leader(conn: &Db, leader: NewLeader) -> Option<Leader> {
    use schema::leaders;

    match conn
        .run(move |c| {
            diesel::insert_into(leaders::table)
                .values(&leader)
                .get_result::<Leader>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn delete_leader(conn: &Db, leader_id: i32) -> Result<(), ()> {
    use schema::leaders::dsl::leaders;

    match conn
        .run(move |c| diesel::delete(leaders.find(leader_id)).execute(c))
        .await
    {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

pub async fn create_follower(conn: &Db, follower: NewFollower) -> Option<Follower> {
    use schema::followers;

    match conn
        .run(move |c| {
            diesel::insert_into(followers::table)
                .values(&follower)
                .get_result::<Follower>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn get_follower_by_uuid(conn: &Db, uuid: String) -> Option<Follower> {
    use self::schema::followers::dsl::{followers, uuid as uid};

    match conn
        .run(move |c| followers.filter(uid.eq(uuid)).first::<Follower>(c))
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

// pub async fn get_remote_notes_by_profile_id(conn: &Db, id: i32) -> Vec<RemoteNote> {
//     use self::schema::remote_notes::dsl::{profile_id, remote_notes};

//     match conn
//         .run(move |c| {
//             remote_notes
//                 .filter(profile_id.eq(id))
//                 .get_results::<RemoteNote>(c)
//         })
//         .await
//     {
//         Ok(x) => x,
//         Err(_) => vec![],
//     }
// }

pub async fn create_note(conn: &Db, note: NewNote) -> Option<Note> {
    use schema::notes;

    match conn
        .run(move |c| {
            diesel::insert_into(notes::table)
                .values(&note)
                .get_result::<Note>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn create_remote_activity(
    conn: &Db,
    remote_activity: NewRemoteActivity,
) -> Option<RemoteActivity> {
    use schema::remote_activities;

    match conn
        .run(move |c| {
            diesel::insert_into(remote_activities::table)
                .values(&remote_activity)
                .on_conflict(remote_activities::ap_id)
                .do_nothing()
                .get_result::<RemoteActivity>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("failed to create remote_activity (probably a duplicate): {e:#?}");
            Option::None
        }
    }
}

pub async fn create_remote_note(conn: &Db, remote_note: NewRemoteNote) -> Option<RemoteNote> {
    use schema::remote_notes;

    match conn
        .run(move |c| {
            diesel::insert_into(remote_notes::table)
                .values(&remote_note)
                .on_conflict(remote_notes::ap_id)
                .do_nothing()
                .get_result::<RemoteNote>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

pub async fn create_profile(conn: &Db, profile: NewProfile) -> Option<Profile> {
    use schema::profiles;

    match conn
        .run(move |c| {
            diesel::insert_into(profiles::table)
                .values(&profile)
                .get_result::<Profile>(c)
        })
        .await
    {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("database failure: {:#?}", e);
            Option::None
        }
    }
}

pub async fn get_profile(conn: &Db, id: i32) -> Option<Profile> {
    use self::schema::profiles::dsl::profiles;

    match conn
        .run(move |c| profiles.find(id).first::<Profile>(c))
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn get_profile_by_username(conn: &Db, username: String) -> Option<Profile> {
    use self::schema::profiles::dsl::{profiles, username as uname};

    match conn
        .run(move |c| profiles.filter(uname.eq(username)).first::<Profile>(c))
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn get_remote_activity_by_ap_id(conn: &Db, ap_id: String) -> Option<RemoteActivity> {
    use self::schema::remote_activities::dsl::{ap_id as aid, remote_activities};

    match conn
        .run(move |c| {
            remote_activities
                .filter(aid.eq(ap_id))
                .first::<RemoteActivity>(c)
        })
        .await
    {
        Ok(x) => Option::from(x),
        Err(_) => Option::None,
    }
}

pub async fn delete_follower_by_ap_id(conn: &Db, ap_id: String) -> bool {
    use self::schema::followers::dsl::{ap_id as aid, followers};

    conn.run(move |c| diesel::delete(followers).filter(aid.eq(ap_id)).execute(c))
        .await
        .is_ok()
}

pub async fn get_followers_by_profile_id(conn: &Db, profile_id: i32) -> Vec<Follower> {
    use self::schema::followers::dsl::{created_at, followers, profile_id as pid};

    match conn
        .run(move |c| {
            followers
                .filter(pid.eq(profile_id))
                .order_by(created_at.desc())
                .get_results::<Follower>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}

pub async fn get_leaders_by_profile_id(conn: &Db, profile_id: i32) -> Vec<Leader> {
    use self::schema::leaders::dsl::{created_at, leaders, profile_id as pid};

    match conn
        .run(move |c| {
            leaders
                .filter(pid.eq(profile_id))
                .order_by(created_at.desc())
                .get_results::<Leader>(c)
        })
        .await
    {
        Ok(x) => x,
        Err(_) => vec![],
    }
}
