#[macro_use]
extern crate log;

use enigmatick::runner::{
    announce::{process_announce, send_announce},
    encrypted::{process_join, provide_one_time_key, send_kexinit},
    follow::{
        acknowledge_followers, process_accept, process_follow, process_remote_undo_follow,
        process_undo_follow,
    },
    like::send_like,
    note::{delete_note, process_outbound_note, process_remote_note, retrieve_context},
    timeline::update_timeline_record,
};
use faktory::ConsumerBuilder;

fn main() {
    env_logger::init();

    let faktory_url = &*enigmatick::FAKTORY_URL;

    info!("STARTING FAKTORY CONSUMER: {}", faktory_url);

    let mut consumer = ConsumerBuilder::default();
    consumer.register("acknowledge_followers", acknowledge_followers);
    consumer.register("provide_one_time_key", provide_one_time_key);
    consumer.register("process_remote_note", process_remote_note);
    consumer.register("process_join", process_join);
    consumer.register("process_outbound_note", process_outbound_note);
    consumer.register("process_announce", process_announce);
    consumer.register("send_kexinit", send_kexinit);
    consumer.register("update_timeline_record", update_timeline_record);
    consumer.register("retrieve_context", retrieve_context);
    consumer.register("send_like", send_like);
    consumer.register("send_announce", send_announce);
    consumer.register("delete_note", delete_note);
    consumer.register("process_follow", process_follow);
    consumer.register("process_accept", process_accept);
    consumer.register("process_undo_follow", process_undo_follow);
    consumer.register("process_remote_undo_follow", process_remote_undo_follow);

    let mut consumer = consumer.connect(Some(faktory_url)).unwrap();

    if let Err(e) = consumer.run(&["default"]) {
        error!("worker failed: {}", e);
    }
}
