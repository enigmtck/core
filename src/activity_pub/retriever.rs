use reqwest::Client;
use reqwest::StatusCode;

use crate::activity_pub::{ApActor, ApObject};
use crate::models::remote_actors::{NewRemoteActor, RemoteActor};
use crate::signing::{sign, SignParams, Method};
use crate::models::profiles::Profile;
use crate::db::{Db, create_remote_actor, get_remote_actor_by_ap_id};

pub async fn get_actor(conn: &Db, profile: Profile, id: String) -> Option<RemoteActor> {
    match get_remote_actor_by_ap_id(conn, id.clone()).await {
        Some(remote_actor) => {
            log::debug!("actor retrieved from storage");
            Option::from(remote_actor)
        },
        None => {
            log::debug!("performing remote lookup for actor");

            let url = id.clone();
            let body = Option::None;
            let method = Method::Get;
            
            let signature = sign(
                SignParams { profile,
                             url,
                             body,
                             method }
            );
            
            let client = Client::new();
            match client.get(&id)
                .header("Signature", &signature.signature)
                .header("Date", signature.date)
                .header("Accept", "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"")
                .send()
                .await {
                    Ok(resp) => {
                        match resp.status() {
                            StatusCode::ACCEPTED | StatusCode::OK => {
                                let actor: ApActor = resp.json().await.unwrap();
                                create_remote_actor(conn, NewRemoteActor::from(actor)).await
                            },
                            StatusCode::GONE => {
                                log::debug!("GONE: {:#?}", resp.status());
                                Option::None
                            },
                            _ => {
                                log::debug!("STATUS: {:#?}", resp.status());
                                Option::None
                            }
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

pub async fn get_followers(conn: &Db, profile: Profile, id: String, page: Option<usize>) {
    if let Some(actor) = get_actor(conn, profile.clone(), id.clone()).await {
        log::debug!("performing remote lookup for actor's followers");

        let page = match page {
            Some(x) => format!("{}?page={}", actor.followers, x),
            None => actor.followers.to_string()
        };
            
        let url = page.clone();
        let body = Option::None;
        let method = Method::Get;
    
        let signature = sign(
            SignParams { profile,
                         url,
                         body,
                         method }
        );
        
        let client = Client::new();
        match client.get(&page)
            .header("Signature", &signature.signature)
            .header("Date", signature.date)
            .header("Accept", "application/ld+json; profile=\"http://www.w3.org/ns/activitystreams\"")
            .send()
            .await
        {
            Ok(resp) => {
                match resp.status() {
                    StatusCode::ACCEPTED | StatusCode::OK => {
                        let j: ApObject = serde_json::from_str(&resp.text().await.unwrap()).unwrap();
                        log::debug!("followers\n{:#?}", j);
                    },
                    StatusCode::GONE => {
                        log::debug!("GONE: {:#?}", resp.status());
                        //Option::None;
                    },
                    _ => {
                        log::debug!("STATUS: {:#?}", resp.status());
                        //Option::None;
                    }
                }                        
            },
            Err(e) => {
                log::debug!("{:#?}", e);
                //Option::None;
            }
        }
    }
}
