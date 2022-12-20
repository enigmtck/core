use diesel::prelude::*;
use enigmatick::models::profiles::{NewProfile, Profile};
use rsa::{
    pkcs8::EncodePrivateKey, pkcs8::EncodePublicKey, pkcs8::LineEnding, RsaPrivateKey, RsaPublicKey,
};

use std::error::Error;
use uuid::Uuid;

pub fn establish_connection() -> diesel::PgConnection {
    let database_url = &*enigmatick::DATABASE_URL;

    diesel::PgConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

struct KeyPair {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

fn get_key_pair() -> KeyPair {
    let mut rng = rand::thread_rng();
    let bits = 2048;
    let private_key = RsaPrivateKey::new(&mut rng, bits).expect("failed");
    let public_key = RsaPublicKey::from(&private_key);

    KeyPair {
        private_key,
        public_key,
    }
}

pub fn create_profile(
    conn: &PgConnection,
    username: String,
    display_name: String,
    summary: Option<String>,
) -> Option<Profile> {
    use enigmatick::schema::profiles;

    let key_pair = get_key_pair();

    let new_profile = NewProfile {
        uuid: Uuid::new_v4().to_string(),
        username,
        display_name,
        summary,
        private_key: key_pair
            .private_key
            .to_pkcs8_pem(LineEnding::default())
            .unwrap()
            .to_string(),
        public_key: key_pair
            .public_key
            .to_public_key_pem(LineEnding::default())
            .unwrap(),
    };

    match diesel::insert_into(profiles::table)
        .values(&new_profile)
        .get_result::<Profile>(conn)
    {
        Ok(x) => Some(x),
        Err(_) => Option::None,
    }
}

fn main() {
    let connection = &mut establish_connection();

    let p = create_profile(
        connection,
        "justin".to_string(),
        "Justin Thomas".to_string(),
        Option::from("The Only".to_string()),
    );

    println!("{}", p.unwrap().uuid);
}
