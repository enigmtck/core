use reqwest::Client;
use log::{debug, info, error};
use url::Url;

use crate::activity_pub::ApActivity;
use crate::models::profiles::Profile;
use crate::signing::{sign, Method, SignParams};

pub async fn send_activity(
    activity: ApActivity,
    profile: Profile,
    inbox: String,
) -> Result<(), ()> {
    debug!("in send_activity_request");

    let activity_json = serde_json::to_string(&activity).unwrap();

    debug!("json: {}", activity_json);

    let url = inbox.clone();
    let body = Option::from(activity_json.clone());
    let method = Method::Post;

    if let Ok(url) = Url::parse(&url.to_string()) {
        let signature = sign(SignParams {
            profile,
            url,
            body,
            method,
        });

        let client = Client::new()
            .post(&inbox)
            .header("Date", signature.date)
            .header("Digest", signature.digest.unwrap_or_default())
            .header("Signature", &signature.signature)
            .header("Content-Type", "application/activity+json")
            .body(activity_json);

        debug!("{:#?}", client);

        match client.send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => info!("send successful to: {}\n{}", inbox, text),
                Err(e) => error!("reqwest response error: {:#?}", e),
            },
            Err(e) => error!("reqwest send error: {:#?}", e),
        }

        Ok(())
    } else {
        Err(())
    }
}
