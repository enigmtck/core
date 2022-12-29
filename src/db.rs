use rocket_sync_db_pools::{database, diesel};
use uuid::Uuid;
use diesel::prelude::*;
use crate::models::remote_notes::{NewRemoteNote, RemoteNote};
use crate::models::remote_actors::{NewRemoteActor, RemoteActor};
use crate::models::remote_activities::{NewRemoteActivity, RemoteActivity};
use crate::models::followers::{NewFollower, Follower};
use crate::models::leaders::{NewLeader, Leader};
use crate::models::notes::{NewNote, Note};
use crate::schema;
use crate::models::profiles::{Profile, NewProfile};

// this is a reference to the value in Rocket.toml, not the actual
// database name
#[database("enigmatick")]
pub struct Db(diesel::PgConnection);


pub async fn update_leader_by_uuid(conn: &Db, leader_uuid: String, accept_ap_id: String) -> Option<Leader> {
    use schema::leaders::dsl::{leaders, accept_ap_id as aapid, uuid, accepted};

    match conn.run(move |c| diesel::update(leaders.filter(uuid.eq(leader_uuid)))
                   .set((aapid.eq(accept_ap_id), accepted.eq(true)))
                   .get_result::<Leader>(c)).await {
        Ok(x) => Some(x),
        Err(_) => Option::None
    }
}

pub async fn create_leader(conn: &Db, leader: NewLeader) -> Option<Leader> {
    use schema::leaders;

    match conn.run(move |c| diesel::insert_into(leaders::table)
                   .values(&leader)
                   .get_result::<Leader>(c)).await {
        Ok(x) => Some(x),
        Err(_) => Option::None
    }
}

pub async fn create_follower(conn: &Db, follower: NewFollower) -> Option<Follower> {
    use schema::followers;

    match conn.run(move |c| diesel::insert_into(followers::table)
                   .values(&follower)
                   .get_result::<Follower>(c)).await {
        Ok(x) => Some(x),
        Err(_) => Option::None
    }
}

pub async fn get_follower_by_uuid(conn: &Db, uuid: String) -> Option<Follower> {
    use self::schema::followers::dsl::{followers, uuid as uid};

    match conn.run(move |c| followers.filter(uid.eq(uuid)).first::<Follower>(c)).await {
        Ok(x) => Option::from(x),
        Err(_) => Option::None
    }
}

pub async fn create_note(conn: &Db, note: NewNote) -> Option<Note> {
    use schema::notes;

    match conn.run(move |c| diesel::insert_into(notes::table)
                   .values(&note)
                   .get_result::<Note>(c)).await {
        Ok(x) => Some(x),
        Err(_) => Option::None
    }
}

pub async fn create_remote_activity(conn: &Db, remote_activity: NewRemoteActivity) -> Option<RemoteActivity> {
    use schema::remote_activities;

    match conn.run(move |c| diesel::insert_into(remote_activities::table)
                   .values(&remote_activity)
                   .get_result::<RemoteActivity>(c)).await {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("database failure: {:#?}", e);
            Option::None
        }
    }
}

pub async fn create_remote_note(conn: &Db, remote_note: NewRemoteNote) -> Option<RemoteNote> {
    use schema::remote_notes;

    match conn.run(move |c| diesel::insert_into(remote_notes::table)
                   .values(&remote_note)
                   .get_result::<RemoteNote>(c)).await {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("database failure: {:#?}", e);
            Option::None
        }
    }
}

pub async fn create_remote_actor(conn: &Db, actor: NewRemoteActor) -> Option<RemoteActor> {
    use schema::remote_actors;

    match conn.run(move |c| diesel::insert_into(remote_actors::table)
                   .values(&actor)
                   .get_result::<RemoteActor>(c)).await {
        Ok(x) => Some(x),
        Err(e) => {
            log::debug!("database failure: {:#?}", e);
            Option::None
        }
    }
}

pub async fn get_remote_actor_by_ap_id(conn: &Db, apid: String) -> Option<RemoteActor> {
    use self::schema::remote_actors::dsl::{remote_actors, ap_id};

    match conn.run(move |c| remote_actors.filter(ap_id.eq(apid)).first::<RemoteActor>(c)).await {
        Ok(x) => Option::from(x),
        Err(_) => Option::None
    }
}

pub async fn create_profile(conn: &Db,
                            username: String,
                            display_name: String,
                            summary: Option<String>,
                            private_key: String,
                            public_key: String)
                            -> Option<Profile> {
    use schema::profiles;

    let new_profile = NewProfile {
        uuid: Uuid::new_v4().to_string(),
        username,
        display_name,
        summary,
        private_key,
        public_key };

    match conn.run(move |c| diesel::insert_into(profiles::table)
                   .values(&new_profile)
                   .get_result::<Profile>(c)).await {
        Ok(x) => Some(x),
        Err(_) => Option::None
    }
}

pub async fn get_profile(conn: &Db, id: i32) -> Option<Profile> {
    use self::schema::profiles::dsl::profiles;

    match conn.run(move |c| profiles.find(id).first::<Profile>(c)).await {
        Ok(x) => Option::from(x),
        Err(_) => Option::None
    }
}

pub async fn get_profile_by_username(conn: &Db, username: String) -> Option<Profile> {
    use self::schema::profiles::dsl::{profiles, username as uname};

    match conn.run(move |c| profiles.filter(uname.eq(username)).first::<Profile>(c)).await {
        Ok(x) => Option::from(x),
        Err(_) => Option::None
    }
}

pub async fn get_remote_activity_by_ap_id(conn: &Db, ap_id: String) -> Option<RemoteActivity> {
    use self::schema::remote_activities::dsl::{remote_activities, ap_id as aid};

    match conn.run(move |c| remote_activities.filter(aid.eq(ap_id)).first::<RemoteActivity>(c)).await {
        Ok(x) => Option::from(x),
        Err(_) => Option::None
    }
}

pub async fn delete_follower_by_ap_id(conn: &Db, ap_id: String) -> bool {
    use self::schema::followers::dsl::{followers, ap_id as aid};

    conn.run(move |c| diesel::delete(followers)
             .filter(aid.eq(ap_id)).execute(c)).await.is_ok()
}

pub async fn get_followers_by_profile_id(conn: &Db, profile_id: i32) -> Vec<Follower> {
    use self::schema::followers::dsl::{followers, profile_id as pid};

    match conn.run(move |c| followers.filter(pid.eq(profile_id))
                   .get_results::<Follower>(c)).await {
        Ok(x) => x,
        Err(_) => vec![]
    }
}
