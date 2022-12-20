use reqwest::Client;
use url::{Url, ParseError};
use chrono::Utc;

use crate::activity_pub::Actor;
use crate::signing::sign;
use crate::models::profiles::Profile;

pub async fn get_actor(profile: Profile, id: String) -> Actor {
    let u = Url::parse(&id).unwrap();
    let host = u.host().unwrap().to_string();
    let path = u.path().to_string();
    let date = Utc::now().to_string();

    let signature = sign(profile, format!("get {}", path), host, date.clone());
    
    let client = Client::new();
    let actor: Actor = client.get(&id)
        .header("Signature", &signature)
        .header("Date", date)
        .header("Accept", "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"")
        .send().await.unwrap().json().await.unwrap();

    actor
}
