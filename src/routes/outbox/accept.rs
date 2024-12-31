use crate::activity_pub::ApAccept;
use crate::routes::Outbox;

impl Outbox for Box<ApAccept> {}
