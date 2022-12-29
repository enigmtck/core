use reqwest::Client;
use log::{debug, info, error};

use crate::activity_pub::{
    ApActivity, ApActivityType, ApContext, ApBaseObject, ApIdentifier, ApObject,
};
use crate::models::profiles::Profile;
use crate::models::remote_actors::RemoteActor;
use crate::signing::{sign, SignParams, Method};

pub async fn send_follower_accept(ap_id: String, profile: Profile, actor: RemoteActor) -> Result<(), ()> {
    debug!("in send_follower_accept");

    let activity = ApActivity {
        base: ApBaseObject {
            context: Option::from(ApContext::Plain(
                "https://www.w3.org/ns/activitystreams".to_string(),
            )),
            ..Default::default()
        },
        actor: format!("https://enigmatick.jdt.dev/user/{}", profile.username),
        kind: ApActivityType::Accept,
        object: ApObject::Identifier(ApIdentifier { id: ap_id }),
    };

    let accept_json = serde_json::to_string(&activity).unwrap();
    
    debug!("json: {}", accept_json);

    let url = actor.inbox.clone();
    let body = Option::from(accept_json.clone());
    let method = Method::Post;
    
    let signature = sign(
        SignParams { profile,
                     url,
                     body,
                     method }
    );

    let client = Client::new()
        .post(&actor.inbox)
        .header("Date", signature.date)
        .header("Digest", signature.digest.unwrap_or_default())
        .header("Signature", &signature.signature)
        .header(
            "Content-Type",
            "application/activity+json",
        )
        .body(accept_json);

    debug!("{:#?}", client);
    
    match client.send().await {
        Ok(resp) => {
            match resp.text().await {
                Ok(text) => info!("send successful to: {}\n{}", actor.inbox, text),
                Err(e) => error!("reqwest response error: {:#?}", e)
            }
        },
        Err(e) => error!("reqwest send error: {:#?}", e)
    }

    Ok(())
}


pub async fn send_follow(activity: ApActivity, profile: Profile, actor: RemoteActor) -> Result<(), ()> {
    debug!("in send_follow_request");

    let follow_json = serde_json::to_string(&activity).unwrap();
    
    debug!("json: {}", follow_json);

    let url = actor.inbox.clone();
    let body = Option::from(follow_json.clone());
    let method = Method::Post;
    
    let signature = sign(
        SignParams { profile,
                     url,
                     body,
                     method }
    );

    let client = Client::new()
        .post(&actor.inbox)
        .header("Date", signature.date)
        .header("Digest", signature.digest.unwrap_or_default())
        .header("Signature", &signature.signature)
        .header(
            "Content-Type",
            "application/activity+json",
        )
        .body(follow_json);

    debug!("{:#?}", client);
    
    match client.send().await {
        Ok(resp) => {
            match resp.text().await {
                Ok(text) => info!("send successful to: {}\n{}", actor.inbox, text),
                Err(e) => error!("reqwest response error: {:#?}", e)
            }
        },
        Err(e) => error!("reqwest send error: {:#?}", e)
    }

    Ok(())
}
