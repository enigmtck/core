use rocket::{
    http::Status,
    post,
    response::stream::{Event, EventStream},
    serde::json::Error,
    serde::json::Json,
    tokio::select,
    tokio::time::{self, Duration},
    Shutdown,
};
use serde::{Deserialize, Serialize};

use crate::{
    fairings::{events::EventChannels, signatures::Signed},
    signing::VerificationType,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StreamAuthorization {
    uuid: String,
}

#[post(
    "/api/user/<username>/events/authorize",
    format = "application/activity+json",
    data = "<authorization>"
)]
pub async fn authorize_stream(
    signed: Signed,
    mut events: EventChannels,
    username: &str,
    authorization: Result<Json<StreamAuthorization>, Error<'_>>,
) -> Result<Json<StreamAuthorization>, Status> {
    if let Signed(true, VerificationType::Local) = signed {
        if let Ok(authorization) = authorization {
            events.authorize(authorization.uuid.clone(), username.to_string());
            Ok(authorization)
        } else {
            Err(Status::Forbidden)
        }
    } else {
        Err(Status::Forbidden)
    }
}

struct DropGuard {
    events: EventChannels,
    uuid: String,
}

impl Drop for DropGuard {
    fn drop(&mut self) {
        log::debug!("REMOVING EVENT LISTENER {}", self.uuid.clone());
        self.events.remove(self.uuid.clone());
    }
}

#[allow(unused_variables)]
#[post("/api/user/<username>/events", data = "<authorization>")]
pub fn stream(
    mut shutdown: Shutdown,
    mut events: EventChannels,
    username: String,
    authorization: Result<Json<StreamAuthorization>, Error<'_>>,
) -> EventStream![] {
    EventStream! {
        let (uuid, rx) = events.subscribe(username.clone());

        let _drop_guard = DropGuard { events: events.clone(), uuid: uuid.clone() };
        let mut i = 1;

        #[derive(Serialize)]
        struct StreamConnect {
            uuid: String
        }

        let message = serde_json::to_string(&StreamConnect { uuid: uuid.clone() }).unwrap();

        yield Event::data(message).event("message").id(i.to_string());
        i += 1;

        let mut interval = time::interval(Duration::from_secs(1));

        loop {
            select! {
                _ = interval.tick() => {
                    if let Ok(message) = rx.try_recv() {
                        log::debug!("SENDING MESSAGE TO {uuid}");
                        i += 1;
                        yield Event::data(message).event("message").id(i.to_string())
                    }
                },
                _ = &mut shutdown => {
                    log::debug!("SHUTDOWN {uuid}");
                    yield Event::data("goodbye").event("message");
                    break;
                }
            };
        }
    }
}
