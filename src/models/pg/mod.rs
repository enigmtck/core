pub mod activities;
pub mod actors;
pub mod cache;
pub mod coalesced_activity;
pub mod encrypted_sessions;
pub mod followers;
pub mod instances;
pub mod leaders;
pub mod note_hashtags;
pub mod notes;
pub mod notifications;
pub mod objects;
pub mod olm_one_time_keys;
pub mod olm_sessions;
pub mod processing_queue;
//pub mod profiles;
pub mod remote_actor_hashtags;
pub mod remote_actors;
pub mod remote_encrypted_sessions;
//pub mod remote_note_hashtags;
//pub mod remote_notes;
//pub mod remote_questions;
pub mod vault;

pub fn parameter_generator() -> impl FnMut() -> String {
    let mut counter = 1;
    move || {
        let param = format!("${}", counter);
        counter += 1;
        param
    }
}
