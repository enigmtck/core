use std::{collections::HashSet, error::Error, fmt::Debug};

use anyhow::Result;
use diesel::{r2d2::ConnectionManager, PgConnection};
use futures_lite::Future;
use reqwest::Client;
use std::fmt;
use url::Url;

use crate::{
    activity_pub::{ApActivity, ApActor, ApAddress},
    db::Db,
    fairings::events::EventChannels,
    models::profiles::Profile,
    signing::{Method, SignParams},
    MaybeMultiple, MaybeReference,
};

use self::{actor::get_actor, user::get_follower_inboxes};

pub mod actor;
pub mod announce;
pub mod cache;
pub mod encrypted;
pub mod follow;
pub mod like;
pub mod note;
pub mod question;
pub mod timeline;
pub mod undo;
pub mod user;

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn clean_text(text: String) -> String {
    let ammonia = ammonia::Builder::default();

    ammonia.clean(&text).to_string()
}

pub async fn send_to_inboxes(
    inboxes: Vec<ApAddress>,
    profile: Profile,
    message: ApActivity,
) -> Result<()> {
    log::debug!("INBOXES\n{inboxes:#?}");

    for inbox in inboxes {
        log::debug!("SENDING TO {inbox}");

        let body = serde_json::to_string(&message).map_err(anyhow::Error::msg)?;

        let url = Url::parse(&inbox.clone().to_string()).map_err(anyhow::Error::msg)?;

        let signature = crate::signing::sign(SignParams {
            profile: profile.clone(),
            url,
            body: Some(body.clone()),
            method: Method::Post,
        })
        .map_err(anyhow::Error::msg)?;

        let client = Client::builder()
            .user_agent("Enigmatick/0.1")
            .build()
            .unwrap();

        let client = client
            .post(inbox.clone().to_string())
            .timeout(std::time::Duration::new(5, 0))
            .header("Date", signature.date)
            .header("Digest", signature.digest.unwrap())
            .header("Signature", &signature.signature)
            .header("Content-Type", "application/activity+json")
            .body(body.clone());

        log::debug!("{client:#?}");
        log::debug!("{body:#?}");

        if let Ok(resp) = client.send().await {
            log::debug!(
                "SEND RESULT FOR {inbox}: {} {:#?}",
                resp.status(),
                resp.text().await
            );
        }
    }

    Ok(())
}

async fn handle_recipients(
    conn: Option<&Db>,
    inboxes: &mut HashSet<ApAddress>,
    sender: &Profile,
    address: &ApAddress,
) {
    let actor = ApActor::from(sender.clone());

    if address.is_public() {
        inboxes.extend(get_follower_inboxes(sender.clone()).await);
        // instead of the above, consider sending to shared inboxes of known instances
        // the duplicate code is temporary because some operations (e.g., Delete) do not have
        // the followers in cc, so until there's logic to send more broadly to all instances,
        // this will need to suffice
    } else if let Some(followers) = actor.followers {
        if address.to_string() == followers {
            inboxes.extend(get_follower_inboxes(sender.clone()).await);
        } else if let Some((actor, _)) =
            get_actor(conn, sender.clone(), address.clone().to_string()).await
        {
            inboxes.insert(ApAddress::Address(actor.inbox));
        }
    }
}

pub async fn get_inboxes(
    conn: Option<&Db>,
    activity: ApActivity,
    sender: Profile,
) -> Vec<ApAddress> {
    let mut inboxes = HashSet::<ApAddress>::new();

    let (to, cc) = match activity {
        ApActivity::Create(activity) => (Some(activity.to), activity.cc),
        ApActivity::Delete(activity) => (Some(activity.to), activity.cc),
        ApActivity::Announce(activity) => (Some(activity.to), activity.cc),
        ApActivity::Like(activity) => (activity.to, None),
        ApActivity::Follow(activity) => {
            if let MaybeReference::Reference(id) = activity.object {
                (Some(MaybeMultiple::Single(ApAddress::Address(id))), None)
            } else {
                (None, None)
            }
        }
        ApActivity::Undo(activity) => {
            if let MaybeReference::Actual(target_activity) = activity.object {
                match target_activity {
                    ApActivity::Follow(follow) => {
                        if let MaybeReference::Reference(target) = follow.object {
                            (
                                Some(MaybeMultiple::Single(ApAddress::Address(target))),
                                None,
                            )
                        } else {
                            (None, None)
                        }
                    }
                    ApActivity::Like(like) => (like.to, None),
                    ApActivity::Announce(announce) => (
                        announce.cc,
                        Some(MaybeMultiple::Single(ApAddress::get_public())),
                    ),
                    _ => (None, None),
                }
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    };

    let consolidated = match (to, cc) {
        (Some(to), Some(MaybeMultiple::Multiple(cc))) => to.extend(cc),
        (Some(to), Some(MaybeMultiple::Single(cc))) => to.extend(vec![cc]),
        (Some(to), Some(MaybeMultiple::None)) => to,
        (Some(to), None) => to,
        (None, Some(cc)) => cc,
        (None, None) => MaybeMultiple::None,
    };

    match consolidated {
        MaybeMultiple::Single(to) => {
            handle_recipients(conn, &mut inboxes, &sender, &to).await;
        }
        MaybeMultiple::Multiple(to) => {
            for address in to.iter() {
                handle_recipients(conn, &mut inboxes, &sender, address).await;
            }
        }
        MaybeMultiple::None => {}
    }

    inboxes.into_iter().collect()
}

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum TaskError {
    TaskFailed,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for TaskError {}

pub async fn run<Fut, F>(
    f: F,
    conn: Option<Db>,
    channels: Option<EventChannels>,
    params: Vec<String>,
) where
    F: Fn(Option<Db>, Option<EventChannels>, Vec<String>) -> Fut,
    Fut: Future<Output = Result<(), TaskError>> + Send + 'static,
{
    tokio::spawn(f(conn, channels, params));
}
