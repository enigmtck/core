extern crate log;
extern crate rocket;

#[macro_use]
extern crate diesel;

use dotenvy::dotenv;
use lazy_static::lazy_static;
use std::env;

pub mod activity_pub;
pub mod admin;
pub mod api;
pub mod db;
pub mod fairings;
pub mod helper;
pub mod inbox;
pub mod models;
pub mod outbox;
pub mod schema;
pub mod signing;
pub mod webfinger;

lazy_static! {
    pub static ref DATABASE_URL: String = {
        dotenv().ok();
        env::var("DATABASE_URL").expect("DATABASE_URL must be set")
    };
    pub static ref SERVER_URL: String = {
        dotenv().ok();
        env::var("SERVER_URL").expect("SERVER_URL must be set")
    };
    pub static ref SERVER_NAME: String = {
        dotenv().ok();
        env::var("SERVER_NAME").expect("SERVER_NAME must be set")
    };
    pub static ref FAKTORY_URL: String = {
        dotenv().ok();
        env::var("FAKTORY_URL").expect("FAKTORY_URL must be set")
    };
    pub static ref RABBITMQ_URL: String = {
        dotenv().ok();
        env::var("RABBITMQ_URL").expect("RABBITMQ_URL must be set")
    };
    pub static ref REGISTRATION_ENABLED: bool = {
        dotenv().ok();
        env::var("REGISTRATION_ENABLED")
            .expect("REGISTRATION_ENABLED must be set")
            .parse()
            .expect("REGISTRATION_ENABLED must be true or false")
    };
    pub static ref REGISTRATION_APPROVAL_REQUIRED: bool = {
        dotenv().ok();
        env::var("REGISTRATION_APPROVAL_REQUIRED")
            .expect("REGISTRATION_APPROVAL_REQUIRED must be set")
            .parse()
            .expect("REGISTRATION_APPROVAL_REQUIRED must be true or false")
    };
    pub static ref REGISTRATION_MESSAGE: String = {
        dotenv().ok();
        env::var("REGISTRATION_MESSAGE").expect("REGISTRATION_MESSAGE must be set")
    };
    pub static ref INSTANCE_CONTACT: String = {
        dotenv().ok();
        env::var("INSTANCE_CONTACT").expect("INSTANCE_CONTACT must be set")
    };
    pub static ref INSTANCE_TITLE: String = {
        dotenv().ok();
        env::var("INSTANCE_TITLE").expect("INSTANCE_TITLE must be set")
    };
    pub static ref INSTANCE_VERSION: String = {
        dotenv().ok();
        env::var("INSTANCE_VERSION").expect("INSTANCE_VERSION must be set")
    };
    pub static ref INSTANCE_SOURCE_URL: String = {
        dotenv().ok();
        env::var("INSTANCE_SOURCE_URL").expect("INSTANCE_SOURCE_URL must be set")
    };
    pub static ref INSTANCE_DESCRIPTION: String = {
        dotenv().ok();
        env::var("INSTANCE_DESCRIPTION").expect("INSTANCE_DESCRIPTION must be set")
    };
}
