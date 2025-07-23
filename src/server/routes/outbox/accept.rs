use crate::server::routes::Outbox;
use jdt_activity_pub::ApAccept;

impl Outbox for Box<ApAccept> {}
