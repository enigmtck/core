use rocket_sync_db_pools::{database, diesel};
use uuid::Uuid;
use diesel::prelude::*;
use crate::schema;
use crate::models::profiles::{Profile, NewProfile};

// this is a reference to the value in Rocket.toml, not the actual
// database name
#[database("enigmatick")]
pub struct Db(diesel::PgConnection);

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

pub async fn get_profile_by_username(conn: &Db, username: String) -> Option<Profile> {
    use self::schema::profiles::dsl::{profiles, username as uname};

    match conn.run(move |c| profiles.filter(uname.eq(username)).first::<Profile>(c)).await {
        Ok(x) => Option::from(x),
        Err(_) => Option::None
    }
}
