use crate::db::runner::DbRunner;
use crate::models::activities::{create_activity, NewActivity};
use crate::models::actors::Actor;
use crate::models::objects::{get_object_by_as_id, Object};
use crate::runner;
use crate::server::routes::Outbox;
use crate::server::AppState;
use axum::http::StatusCode;
use chrono::Utc;
use jdt_activity_pub::{
    ApActivity, ApAddress, ApArticle, ApContext, ApDateTime, ApHashtag, ApNote, ApObject,
    ApQuestion, ApUpdate, MaybeMultiple, MaybeReference,
};
use serde_json::{json, Value};

use super::ActivityJson;

impl Outbox for ApUpdate {
    async fn outbox<C: DbRunner>(
        &self,
        conn: &C,
        state: AppState,
        profile: Actor,
        raw: Value,
    ) -> Result<ActivityJson<ApActivity>, StatusCode> {
        update_outbox(conn, state, self.clone(), profile, raw).await
    }
}

async fn update_outbox<C: DbRunner>(
    conn: &C,
    state: AppState,
    mut update: ApUpdate,
    profile: Actor,
    raw: Value,
) -> Result<ActivityJson<ApActivity>, StatusCode> {
    // Extract the embedded object from the Update activity
    let object_to_update = match &update.object {
        MaybeReference::Actual(obj) => obj.clone(),
        MaybeReference::Reference(id) => {
            log::error!("Update activity must contain an embedded object, not a reference: {id}");
            return Err(StatusCode::BAD_REQUEST);
        }
        MaybeReference::None | MaybeReference::Identifier(_) => {
            log::error!("Update activity must contain an embedded object");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Get the object ID from the embedded object
    let object_id = match &object_to_update {
        ApObject::Note(note) => note.id.clone(),
        ApObject::Article(article) => article.id.clone(),
        ApObject::Question(question) => question.id.clone(),
        _ => {
            log::error!("Update only supports Note, Article, and Question objects");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let object_id = object_id.ok_or_else(|| {
        log::error!("Object must have an id for Update");
        StatusCode::BAD_REQUEST
    })?;

    // Fetch the existing object from the database
    let existing_object = get_object_by_as_id(conn, object_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to find object {object_id}: {e}");
            StatusCode::NOT_FOUND
        })?;

    // Verify that the user owns this object (attributed_to matches the profile)
    let username = profile.ek_username.as_ref().ok_or_else(|| {
        log::error!("Profile has no username");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let profile_ap_id = format!("https://{}/user/{}", *crate::SERVER_NAME, username);
    let attributed_to = existing_object.attributed_to();

    if !attributed_to.contains(&profile_ap_id) {
        log::error!(
            "User {} is not authorized to update object {} (attributed_to: {:?})",
            profile_ap_id,
            object_id,
            attributed_to
        );
        return Err(StatusCode::FORBIDDEN);
    }

    // Apply the update based on object type
    let updated_object = match object_to_update {
        ApObject::Note(note) => {
            update_note(conn, &existing_object, note).await?;
            get_object_by_as_id(conn, object_id.clone())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        ApObject::Article(article) => {
            update_article(conn, &existing_object, article).await?;
            get_object_by_as_id(conn, object_id.clone())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        ApObject::Question(question) => {
            update_question(conn, &existing_object, question).await?;
            get_object_by_as_id(conn, object_id.clone())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
        _ => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Reconstruct the Update activity with the updated object for federation
    log::debug!("DB Object as_updated: {:?}", updated_object.as_updated);
    let federated_object = build_federated_object(&updated_object)?;

    // Debug: Check if updated field is in the ApObject
    if let ApObject::Note(ref note) = federated_object {
        log::debug!("ApNote updated field: {:?}", note.updated);
    }

    // Set addressing from the object's to and cc fields for proper federation
    let mut recipients: Vec<ApAddress> = Vec::new();

    // Add 'to' recipients
    let to_addresses: MaybeMultiple<ApAddress> = updated_object.as_to.clone().into();
    for addr in to_addresses.multiple() {
        recipients.push(addr);
    }

    // Add 'cc' recipients
    let cc_addresses: MaybeMultiple<ApAddress> = updated_object.as_cc.clone().into();
    for addr in cc_addresses.multiple() {
        if !recipients.contains(&addr) {
            recipients.push(addr);
        }
    }

    update.context = Some(ApContext::default());
    update.object = MaybeReference::Actual(federated_object);
    update.to = recipients.into();
    update.published = Some(ApDateTime::now());

    // Create the activity record
    let mut activity = NewActivity::try_from((
        ApActivity::from(update.clone()),
        Some(updated_object.into()),
    ))
    .map_err(|e| {
        log::error!("Failed to create NewActivity: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    activity.raw = Some(raw);

    let created_activity = create_activity(conn, activity).await.map_err(|e| {
        log::error!("Failed to create activity: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    update.id = created_activity.ap_id.clone();
    let ap_id = created_activity.ap_id.ok_or_else(|| {
        log::error!("Activity ap_id cannot be None");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    log::debug!("Created Update activity: {update:?}");

    let final_activity = ApActivity::Update(update);

    // Send the Update activity to federated servers
    runner::run(runner::send_activity_task, state.db_pool, None, vec![ap_id]).await;

    Ok(ActivityJson(final_activity))
}

/// Update a Note with allowed fields: content, source, tags, attachments
/// Published is updated to current time
async fn update_note<C: DbRunner>(
    conn: &C,
    existing: &Object,
    note: ApNote,
) -> Result<(), StatusCode> {
    use crate::schema::objects;
    use diesel::prelude::*;

    use crate::AMMONIA_BUILDER;

    let now = Utc::now();
    let object_id = existing.id;

    // Extract hashtags for ek_hashtags field (clone before partial moves)
    let hashtags: Vec<ApHashtag> = note.clone().into();
    let ek_hashtags = json!(hashtags
        .iter()
        .map(|x| x.name.clone().to_lowercase())
        .collect::<Vec<String>>());

    // Sanitize content
    let clean_content = note.content.map(|c| AMMONIA_BUILDER.clean(&c).to_string());

    // Convert source to JSON if present
    let ap_source: Option<Value> = note.source.map(|s| json!(s));

    // Serialize tag and attachment before move into closure
    let tag_json = json!(note.tag);
    let attachment_json = json!(note.attachment);

    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(object_id)))
            .set((
                objects::as_content.eq(clean_content),
                objects::as_tag.eq(Some(tag_json)),
                objects::as_attachment.eq(Some(attachment_json)),
                objects::ek_hashtags.eq(ek_hashtags),
                objects::ap_source.eq(ap_source),
                objects::as_published.eq(Some(now)),
                objects::as_updated.eq(Some(now)),
            ))
            .execute(c)
    })
    .await
    .map_err(|e| {
        log::error!("Failed to update Note: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

/// Update an Article with allowed fields: name, content, source, summary, tags, attachments
/// Published is updated to current time
async fn update_article<C: DbRunner>(
    conn: &C,
    existing: &Object,
    article: ApArticle,
) -> Result<(), StatusCode> {
    use crate::schema::objects;
    use diesel::prelude::*;

    use crate::AMMONIA_BUILDER;

    let now = Utc::now();
    let object_id = existing.id;

    // Extract hashtags for ek_hashtags field (clone before partial moves)
    let hashtags: Vec<ApHashtag> = article.clone().into();
    let ek_hashtags = json!(hashtags
        .iter()
        .map(|x| x.name.clone().to_lowercase())
        .collect::<Vec<String>>());

    // Sanitize content and summary
    let clean_content = article.content.map(|c| AMMONIA_BUILDER.clean(&c).to_string());
    let clean_summary = article.summary.map(|s| AMMONIA_BUILDER.clean(&s).to_string());

    // Convert source to JSON if present
    let ap_source: Option<Value> = article.source.map(|s| json!(s));

    // Serialize tag and attachment before move into closure
    let tag_json = json!(article.tag);
    let attachment_json = json!(article.attachment);
    let name = article.name;

    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(object_id)))
            .set((
                objects::as_name.eq(name),
                objects::as_content.eq(clean_content),
                objects::as_summary.eq(clean_summary),
                objects::as_tag.eq(Some(tag_json)),
                objects::as_attachment.eq(Some(attachment_json)),
                objects::ek_hashtags.eq(ek_hashtags),
                objects::ap_source.eq(ap_source),
                objects::as_published.eq(Some(now)),
                objects::as_updated.eq(Some(now)),
            ))
            .execute(c)
    })
    .await
    .map_err(|e| {
        log::error!("Failed to update Article: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

/// Update a Question with allowed fields: content, tags, attachments
/// Notably, voting options (one_of, any_of) are NOT updatable
/// Published is updated to current time
async fn update_question<C: DbRunner>(
    conn: &C,
    existing: &Object,
    question: ApQuestion,
) -> Result<(), StatusCode> {
    use crate::schema::objects;
    use diesel::prelude::*;

    use crate::AMMONIA_BUILDER;

    let now = Utc::now();
    let object_id = existing.id;

    // Extract hashtags for ek_hashtags field (clone before partial moves)
    let hashtags: Vec<ApHashtag> = question.clone().into();
    let ek_hashtags = json!(hashtags
        .iter()
        .map(|x| x.name.clone().to_lowercase())
        .collect::<Vec<String>>());

    // Sanitize content
    let clean_content = question.content.map(|c| AMMONIA_BUILDER.clean(&c).to_string());

    // Convert source to JSON if present
    let ap_source: Option<Value> = question.source.map(|s| json!(s));

    // Serialize tag and attachment before move into closure
    let tag_json = json!(question.tag);
    let attachment_json = json!(question.attachment);

    conn.run(move |c| {
        diesel::update(objects::table.filter(objects::id.eq(object_id)))
            .set((
                objects::as_content.eq(clean_content),
                objects::as_tag.eq(Some(tag_json)),
                objects::as_attachment.eq(Some(attachment_json)),
                objects::ek_hashtags.eq(ek_hashtags),
                objects::ap_source.eq(ap_source),
                objects::as_published.eq(Some(now)),
                objects::as_updated.eq(Some(now)),
            ))
            .execute(c)
    })
    .await
    .map_err(|e| {
        log::error!("Failed to update Question: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

/// Build a federated ApObject from the updated database Object
fn build_federated_object(object: &Object) -> Result<ApObject, StatusCode> {
    ApObject::try_from(object.clone()).map_err(|e| {
        log::error!("Failed to convert Object to ApObject: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}
