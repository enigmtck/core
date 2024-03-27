use crate::activity_pub::{ApAddress, ApQuestion, ApQuestionType};
use crate::db::Db;
use crate::models::{to_serde, to_time};
use crate::schema::remote_questions;
use crate::{MaybeMultiple, POOL};
use anyhow::Result;

use diesel::prelude::*;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        use crate::models::pg::remote_questions::QuestionType;
        fn to_kind(kind: ApQuestionType) -> QuestionType {
            kind.into()
        }

        pub use crate::models::pg::remote_questions::RemoteQuestion;
        pub use crate::models::pg::remote_questions::NewRemoteQuestion;
        pub use crate::models::pg::remote_questions::create_or_update_remote_question;
    } else if #[cfg(feature = "sqlite")] {
        fn to_kind(kind: ApQuestionType) -> String {
            kind.to_string().to_lowercase()
        }

        pub use crate::models::sqlite::remote_questions::RemoteQuestion;
        pub use crate::models::sqlite::remote_questions::NewRemoteQuestion;
        pub use crate::models::sqlite::remote_questions::create_or_update_remote_question;
    }
}

impl From<ApQuestion> for NewRemoteQuestion {
    fn from(question: ApQuestion) -> Self {
        NewRemoteQuestion {
            kind: to_kind(question.kind),
            ap_id: question.id,
            ap_to: to_serde(question.to),
            cc: question.cc.and_then(to_serde),
            end_time: question.end_time.map(to_time),
            published: question.published.map(to_time),
            one_of: question.one_of.and_then(to_serde),
            any_of: question.any_of.and_then(to_serde),
            content: question.content,
            content_map: question.content_map.and_then(to_serde),
            summary: question.summary,
            voters_count: question.voters_count,
            url: question.url,
            conversation: question.conversation,
            tag: question.tag.and_then(to_serde),
            attachment: question.attachment.and_then(to_serde),
            ap_sensitive: question.sensitive,
            in_reply_to: question.in_reply_to,
            attributed_to: question.attributed_to.to_string(),
        }
    }
}

impl RemoteQuestion {
    pub fn is_public(&self) -> bool {
        if let Ok(to) =
            serde_json::from_value::<MaybeMultiple<ApAddress>>(self.ap_to.clone().into())
        {
            for address in to.multiple() {
                if address.is_public() {
                    return true;
                }
            }
        }

        if let Some(cc) = self.cc.clone() {
            if let Ok(cc) = serde_json::from_value::<MaybeMultiple<ApAddress>>(cc.into()) {
                for address in cc.multiple() {
                    if address.is_public() {
                        return true;
                    }
                }
            }
        }

        false
    }
}

pub async fn get_remote_question_by_ap_id(
    conn: Option<&Db>,
    ap_id: String,
) -> Option<RemoteQuestion> {
    match conn {
        Some(conn) => conn
            .run(move |c| {
                remote_questions::table
                    .filter(remote_questions::ap_id.eq(ap_id))
                    .first::<RemoteQuestion>(c)
            })
            .await
            .ok(),
        None => {
            let mut pool = POOL.get().ok()?;
            remote_questions::table
                .filter(remote_questions::ap_id.eq(ap_id))
                .first::<RemoteQuestion>(&mut pool)
                .ok()
        }
    }
}
