use crate::fairings::events::EventChannels;
use futures_lite::stream::StreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Orbit, Rocket};

#[derive(Clone)]
pub struct MqConnection;

impl MqConnection {
    pub fn fairing() -> impl Fairing {
        MqConnectionFairing
    }
}

struct MqConnectionFairing;

#[rocket::async_trait]
impl Fairing for MqConnectionFairing {
    fn info(&self) -> Info {
        Info {
            name: "MQ Connection",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        if let Ok(conn) =
            Connection::connect(&crate::AMQP_URL, ConnectionProperties::default()).await
        {
            if let Ok(channel) = conn.create_channel().await {
                let _queue = channel
                    .queue_declare(
                        "events",
                        QueueDeclareOptions::default(),
                        FieldTable::default(),
                    )
                    .await
                    .unwrap();

                if let Ok(mut consumer) = channel
                    .basic_consume(
                        &crate::AMQP_QUEUE,
                        &crate::AMQP_CONSUMER_TAG,
                        BasicConsumeOptions::default(),
                        FieldTable::default(),
                    )
                    .await
                {
                    if let Some(state) = rocket.state::<EventChannels>() {
                        let mut state = state.clone();
                        tokio::spawn(async move {
                            while let Some(delivery) = consumer.next().await {
                                if let Ok(delivery) = delivery {
                                    if delivery.ack(BasicAckOptions::default()).await.is_ok() {
                                        state.send(
                                            String::from_utf8(delivery.data.clone()).unwrap(),
                                        );
                                    }
                                }
                            }
                        });
                    } else {
                        log::error!("FAILED TO RETRIEVE EventChannels");
                    }
                } else {
                    log::error!("FAILED TO CREATE AMQP CONSUMER");
                }
            } else {
                log::error!("FAILED TO CREATE AMQP CHANNEL");
            }
        } else {
            log::error!("FAILED TO CONNECT TO AMQP SERVER");
        }
    }
}
