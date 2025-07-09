use async_mutex::Mutex;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;

use rocket::fairing::{self, Fairing, Info, Kind};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Build, Rocket};
use std::collections::HashMap;

#[derive(Clone)]
pub struct IdentifiedReceiver {
    _username: String,
    _receiver: Receiver<String>,
}

type ReceiverTuple = (String, Receiver<String>);

impl From<ReceiverTuple> for IdentifiedReceiver {
    fn from(t: ReceiverTuple) -> Self {
        IdentifiedReceiver {
            _username: t.0,
            _receiver: t.1,
        }
    }
}

#[derive(Clone)]
pub struct IdentifiedSender {
    username: String,
    sender: Sender<String>,
    authorized: bool,
}

type SenderTuple = (String, Sender<String>);

impl From<SenderTuple> for IdentifiedSender {
    fn from(t: SenderTuple) -> Self {
        IdentifiedSender {
            username: t.0,
            sender: t.1,
            authorized: false,
        }
    }
}

#[derive(Clone)]
pub struct EventChannels {
    // there's no cleanup for these maps - probable something to keep an eye on
    pub receiving_channels: Arc<Mutex<HashMap<String, IdentifiedReceiver>>>,
    pub sending_channels: Arc<Mutex<HashMap<String, IdentifiedSender>>>,
}

impl Default for EventChannels {
    fn default() -> Self {
        EventChannels::new()
    }
}

impl EventChannels {
    // Add this new function
    pub fn new() -> Self {
        EventChannels {
            receiving_channels: Arc::new(Mutex::new(HashMap::new())),
            sending_channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn fairing() -> impl Fairing {
        EventChannelsFairing
    }

    pub fn authorize(&mut self, uuid: String, username: String) {
        log::debug!("authorize called");

        if let Some(mut x) = self.sending_channels.try_lock() {
            if let Some(r) = x.get(&uuid) {
                if r.username == username {
                    let mut r = r.clone();
                    r.authorized = true;
                    x.insert(uuid.clone(), r);
                    log::debug!("sender for {username} authorized");
                }
            }
        }
    }

    pub fn remove(&mut self, uuid: String) {
        log::debug!("remove called");

        if let Some(mut x) = self.receiving_channels.try_lock() {
            x.remove(&uuid);
        }

        if let Some(mut x) = self.sending_channels.try_lock() {
            x.remove(&uuid);
        }
    }

    pub fn subscribe(&mut self, username: String) -> (String, Receiver<String>) {
        log::debug!("SUBSCRIBE CALLED");

        let uuid = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = unbounded::<String>();

        if let Some(mut x) = self.receiving_channels.try_lock() {
            x.insert(uuid.clone(), (username.clone(), rx.clone()).into());
        }

        if let Some(mut x) = self.sending_channels.try_lock() {
            x.insert(uuid.clone(), (username, tx).into());
        }

        (uuid, rx)
    }

    pub fn send(&mut self, stream: Option<String>, message: String) {
        log::debug!("send called");
        if let Some(mut x) = self.sending_channels.try_lock() {
            for (uuid, identified_sender) in (*x).clone() {
                if identified_sender.authorized {
                    match stream.clone() {
                        Some(stream) => {
                            if stream == uuid {
                                log::debug!("SENDING EVENT");

                                if identified_sender.sender.try_send(message.clone()).is_err() {
                                    x.remove(&uuid);
                                };
                            }
                        }
                        None => {
                            log::debug!("SENDING EVENT");

                            if identified_sender.sender.try_send(message.clone()).is_err() {
                                x.remove(&uuid);
                            };
                        }
                    }
                } else {
                    log::debug!("{uuid} NOT YET AUTHORIZED");
                }
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
        log::debug!("igniting EventsChannel");
        Ok(rocket.manage(EventChannels::new())) // Use the new function here
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
            Outcome::Error((Status::BadRequest, EventChannelsError::Failed))
        }
    }
}
