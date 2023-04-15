use diesel::prelude::*;

use crate::{
    models::processing_queue::{NewProcessingItem, ProcessingItem},
    schema::processing_queue,
};

use super::POOL;

pub fn create_processing_item(processing_item: NewProcessingItem) -> Option<ProcessingItem> {
    if let Ok(conn) = POOL.get() {
        match diesel::insert_into(processing_queue::table)
            .values(&processing_item)
            .get_result::<ProcessingItem>(&conn)
        {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{:#?}", e);
                Option::None
            }
        }
    } else {
        Option::None
    }
}
