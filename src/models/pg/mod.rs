pub mod activities;
pub mod actors;
pub mod cache;
pub mod coalesced_activity;
pub mod encrypted_sessions;
pub mod followers;
pub mod instances;
pub mod leaders;
pub mod notifications;
pub mod objects;
pub mod olm_one_time_keys;
pub mod olm_sessions;
pub mod processing_queue;
pub mod remote_encrypted_sessions;
pub mod unprocessable;
pub mod vault;

pub fn parameter_generator() -> impl FnMut() -> String {
    let mut counter = 1;
    move || {
        let param = format!("${}", counter);
        counter += 1;
        param
    }
}
