#[macro_use]
extern crate log;

use enigmatick::models::{leaders::Leader, profiles::Profile, remote_actors::RemoteActor};

use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use lazy_static::lazy_static;

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

    if let Ok(mut conn) = POOL.get() {
        match remote_actors
            .inner_join(leaders.on(leader_ap_id.eq(ra_apid)))
            .left_join(profiles.on(profile_id.eq(pid)))
            .filter(followers.eq(endpoint))
            .get_results::<(RemoteActor, Leader, Option<Profile>)>(&mut conn)
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

pub fn get_leader_by_endpoint(endpoint: String) -> Option<(RemoteActor, Leader)> {
    use enigmatick::schema::leaders::dsl::{leader_ap_id, leaders};
    use enigmatick::schema::remote_actors::dsl::{ap_id as ra_apid, followers, remote_actors};

    if let Ok(mut conn) = POOL.get() {
        match remote_actors
            .inner_join(leaders.on(leader_ap_id.eq(ra_apid)))
            .filter(followers.eq(endpoint))
            .first::<(RemoteActor, Leader)>(&mut conn)
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

fn main() {
    env_logger::init();

    // let res =
    //     get_leader_by_followers_endpoint("https://me.dm/users/anildash/followers".to_string());

    let res = get_leader_by_endpoint("https://me.dm/users/anildash/followers".to_string());

    println!("{}", &format!("{res:#?}"));
}
