use core::fmt;
use std::fmt::Debug;

use crate::activity_pub::{
    ApActivity, ApActor, ApAttachment, ApContext, ApEndpoint, ApImage, ApTag, Outbox,
};
use crate::db::Db;
use crate::fairings::events::EventChannels;
use crate::models::actors::ActorType;
use crate::models::actors::{get_actor_by_as_id, Actor};
use crate::models::followers::get_follower_count_by_actor_id;
use crate::models::leaders::{get_leader_by_actor_id_and_ap_id, get_leader_count_by_actor_id};
use crate::routes::ActivityJson;
use crate::webfinger::retrieve_webfinger;
use crate::{MaybeMultiple, DOMAIN_RE};
use anyhow::{self, Result};
use lazy_static::lazy_static;
use rocket::http::Status;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::serde_as;

impl Outbox for ApActor {
    async fn outbox(
        &self,
        _conn: Db,
        _events: EventChannels,
        _profile: Actor,
        _raw: Value,
    ) -> Result<ActivityJson<ApActivity>, Status> {
        Err(Status::ServiceUnavailable)
    }
}
