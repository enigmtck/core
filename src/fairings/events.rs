use async_mutex::Mutex;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;

use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Build, Rocket};
use std::collections::HashMap;

#[derive(Clone)]
pub struct EventChannels {
    // there's no cleanup for these maps - probable something to keep an eye on
    pub receiving_channels: Arc<Mutex<HashMap<String, Receiver<String>>>>,
    pub sending_channels: Arc<Mutex<HashMap<String, Sender<String>>>>,
}

impl EventChannels {
    pub fn fairing() -> impl Fairing {
        EventChannelsFairing
    }

    pub fn subscribe(&mut self, username: String) -> Receiver<String> {
        log::debug!("subscribe called");
        let (tx, rx) = unbounded::<String>();

        if let Some(mut x) = self.receiving_channels.try_lock() {
            x.insert(username.clone(), rx.clone());
        }

        if let Some(mut x) = self.sending_channels.try_lock() {
            x.insert(username, tx);
        }

        rx
    }

    pub fn send(&mut self, message: String) {
        log::debug!("send called");
        if let Some(x) = self.sending_channels.try_lock() {
            for (username, tx) in (*x).clone() {
                log::debug!("trying to send {message}");

                match tx.try_send(message.clone()) {
                    Ok(x) => log::debug!("sent"),
                    Err(e) => log::error!("send failed: {e:#?}"),
                };
            }
        }
    }
}

struct EventChannelsFairing;

#[rocket::async_trait]
impl Fairing for EventChannelsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Event Channels",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> fairing::Result {
        Ok(rocket.manage({
            //let queue = VecDeque::<String>::new();

            EventChannels {
                receiving_channels: Arc::new(Mutex::new(HashMap::new())),
                sending_channels: Arc::new(Mutex::new(HashMap::new())),
            }
        }))
    }
}

#[derive(Debug)]
pub enum EventChannelsError {
    Failed,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for EventChannels {
    type Error = EventChannelsError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(channels) = request.rocket().state::<EventChannels>() {
            Outcome::Success(channels.clone())
        } else {
            Outcome::Failure((Status::BadRequest, EventChannelsError::Failed))
        }
    }
}
