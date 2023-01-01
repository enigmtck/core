extern crate log;
extern crate rocket;

#[macro_use]
extern crate diesel;

use dotenvy::dotenv;
use lazy_static::lazy_static;
use std::env;

pub mod activity_pub;
pub mod admin;
pub mod db;
pub mod models;
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
}
