use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Orbit, Rocket};
use tokio::task;

pub struct ProxyFairing;

impl ProxyFairing {
    pub fn fairing() -> ProxyFairing {
        ProxyFairing
    }
}

#[rocket::async_trait]
impl Fairing for ProxyFairing {
    fn info(&self) -> Info {
        Info {
            name: "Proxy and Axum Server Background Tasks",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, _rocket: &Rocket<Orbit>) {
        // Spawn the Axum server task
        task::spawn(async {
            crate::axum_server::start().await;
        });

        if *crate::ACME_PROXY {
            log::info!("Starting proxy background task...");

            task::spawn(async {
                if let Err(e) = crate::proxy::start().await {
                    log::error!("Unable to start proxy: {e}");
                }
            });
        } else {
            log::info!("Proxy service not enabled");
        }
    }
}
