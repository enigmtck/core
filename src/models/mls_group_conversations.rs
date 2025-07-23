use crate::db::runner::DbRunner;
use crate::helper::get_instrument_as_id_from_uuid;
use crate::schema::mls_group_conversations;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel::Insertable;
use diesel::{AsChangeset, Identifiable, Queryable};
use jdt_activity_pub::{ApInstrument, ApInstrumentType};
use serde::{Deserialize, Serialize};

#[derive(Identifiable, Queryable, AsChangeset, Serialize, Clone, Default, Debug)]
#[diesel(table_name = mls_group_conversations)]
pub struct MlsGroupConversation {
    #[serde(skip_serializing)]
    pub id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub uuid: String,
    pub actor_id: i32,
    pub conversation: String,
    pub mls_group: String,
}

#[derive(Serialize, Deserialize, Insertable, Default, Debug, Clone)]
#[diesel(table_name = mls_group_conversations)]
pub struct NewMlsGroupConversation {
    pub uuid: String,
    pub actor_id: i32,
    pub conversation: String,
    pub mls_group: String,
}

impl From<MlsGroupConversation> for ApInstrument {
    fn from(mgc: MlsGroupConversation) -> Self {
        Self {
            kind: ApInstrumentType::MlsGroupId,
            id: Some(get_instrument_as_id_from_uuid(mgc.uuid.clone())),
            content: Some(mgc.mls_group),
            uuid: None,
            hash: None,
            name: None,
            url: None,
            mutation_of: None,
            conversation: Some(mgc.conversation),
            activity: None,
        }
    }
}

// profile_id, mls_group, conversation
type GroupTuple = (i32, String, String);

impl From<GroupTuple> for NewMlsGroupConversation {
    fn from((actor_id, mls_group, conversation): GroupTuple) -> NewMlsGroupConversation {
        NewMlsGroupConversation {
            actor_id,
            uuid: uuid::Uuid::new_v4().to_string(),
            conversation,
            mls_group,
        }
    }
}

pub async fn create_mls_group_conversation<C: DbRunner>(
    conn: &C,
    mls_group_conversation: NewMlsGroupConversation,
) -> Result<MlsGroupConversation> {
    conn.run(move |c| {
        diesel::insert_into(mls_group_conversations::table)
            .values(&mls_group_conversation)
            .get_result::<MlsGroupConversation>(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}

pub async fn get_mls_group_conversations_by_actor_id<C: DbRunner>(
    conn: &C,
    id: i32,
    limit: i64,
    offset: i64,
) -> Vec<MlsGroupConversation> {
    conn.run(move |c| {
        let query = mls_group_conversations::table
            .filter(mls_group_conversations::actor_id.eq(id))
            .order(mls_group_conversations::created_at.asc())
            .limit(limit)
            .offset(offset)
            .into_boxed();

        query.get_results::<MlsGroupConversation>(c)
    })
    .await
    .unwrap_or(vec![])
}

pub async fn get_mls_group_conversation_by_conversation_and_actor_id<C: DbRunner>(
    conn: &C,
    conversation: String,
    actor_id: i32,
) -> Result<i64> {
    conn.run(move |c| {
        mls_group_conversations::table
            .filter(
                mls_group_conversations::conversation
                    .eq(conversation)
                    .and(mls_group_conversations::actor_id.eq(actor_id)),
            )
            .count()
            .get_result(c)
    })
    .await
    .map_err(anyhow::Error::msg)
}
