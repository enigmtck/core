use anyhow::Result;
use tantivy::{IndexWriter, TantivyDocument};
use tantivy::schema::*;

use crate::models::actors::Actor;
use crate::models::objects::Object;

/// Helper to convert Object to Tantivy document
pub fn object_to_document(object: &Object, schema: &Schema) -> Result<TantivyDocument> {
    let id = schema.get_field("id").unwrap();
    let as_id = schema.get_field("as_id").unwrap();
    let content = schema.get_field("content").unwrap();
    let name = schema.get_field("name").unwrap();
    let summary = schema.get_field("summary").unwrap();
    let _author_username = schema.get_field("author_username").unwrap();
    let author_id = schema.get_field("author_id").unwrap();
    let object_type = schema.get_field("object_type").unwrap();
    let type_facet = schema.get_field("type_facet").unwrap();
    let visibility = schema.get_field("visibility").unwrap();
    let published = schema.get_field("published").unwrap();
    let conversation_id = schema.get_field("conversation_id").unwrap();

    let mut doc = TantivyDocument::default();

    // Required fields
    doc.add_text(id, &object.id.to_string());
    doc.add_text(as_id, &object.as_id);
    doc.add_text(object_type, &format!("{:?}", object.as_type));
    doc.add_facet(type_facet, Facet::from(&format!("/{:?}", object.as_type)));

    // Optional text content
    if let Some(ref content_text) = object.as_content {
        doc.add_text(content, content_text);
    }
    if let Some(ref name_text) = object.as_name {
        doc.add_text(name, name_text);
    }
    if let Some(ref summary_text) = object.as_summary {
        doc.add_text(summary, summary_text);
    }

    // Author information - extract from as_attributed_to if available
    if let Some(ref attributed_to) = object.as_attributed_to {
        // Try to extract author username from the JSONB field
        if let Some(author_id_str) = attributed_to.as_str() {
            doc.add_facet(author_id, Facet::from(&format!("/{}", author_id_str)));
        } else if let Some(author_obj) = attributed_to.as_object() {
            if let Some(author_id_str) = author_obj.get("id").and_then(|v| v.as_str()) {
                doc.add_facet(author_id, Facet::from(&format!("/{}", author_id_str)));
            }
        }
    }

    // Visibility - default to public if not specified
    // This will need to be enhanced based on your ActivityPub to/cc fields
    doc.add_facet(visibility, Facet::from("/public"));

    // Publication date - use as_published if available, otherwise fall back to created_at
    let pub_date = object.as_published.unwrap_or(object.created_at);
    doc.add_date(published, tantivy::DateTime::from_timestamp_secs(pub_date.timestamp()));

    // Conversation ID for threading
    if let Some(ref conv_id) = object.ap_conversation {
        doc.add_text(conversation_id, conv_id);
    }

    Ok(doc)
}

/// Helper to convert Actor to Tantivy document
pub fn actor_to_document(actor: &Actor, schema: &Schema) -> Result<TantivyDocument> {
    let id = schema.get_field("id").unwrap();
    let as_id = schema.get_field("as_id").unwrap();
    let username = schema.get_field("username").unwrap();
    let display_name = schema.get_field("display_name").unwrap();
    let summary = schema.get_field("summary").unwrap();
    let tags = schema.get_field("tags").unwrap();
    let also_known_as = schema.get_field("also_known_as").unwrap();
    let actor_type = schema.get_field("actor_type").unwrap();
    let type_facet = schema.get_field("type_facet").unwrap();
    let is_local = schema.get_field("is_local").unwrap();
    let is_discoverable = schema.get_field("is_discoverable").unwrap();

    let mut doc = TantivyDocument::default();

    // Required fields
    doc.add_text(id, &actor.id.to_string());
    doc.add_text(as_id, &actor.as_id);
    doc.add_text(actor_type, &format!("{:?}", actor.as_type));
    doc.add_facet(type_facet, Facet::from(&format!("/{:?}", actor.as_type)));

    // Username
    if let Some(ref username_text) = actor.as_preferred_username {
        doc.add_text(username, username_text);
    }

    // Display name
    if let Some(ref name_text) = actor.as_name {
        doc.add_text(display_name, name_text);
    }

    // Bio/summary
    if let Some(ref summary_text) = actor.as_summary {
        doc.add_text(summary, summary_text);
    }

    // Extract and index tags from as_tag and ek_hashtags
    let mut tag_texts = Vec::new();

    // Extract from as_tag (JSONB array of tag objects)
    if let Some(as_tag_array) = actor.as_tag.as_array() {
        for tag in as_tag_array {
            if let Some(tag_name) = tag.get("name").and_then(|v| v.as_str()) {
                tag_texts.push(tag_name.to_string());
            }
        }
    }

    // Extract from ek_hashtags (JSONB array of strings)
    if let Some(hashtags_array) = actor.ek_hashtags.as_array() {
        for hashtag in hashtags_array {
            if let Some(hashtag_str) = hashtag.as_str() {
                tag_texts.push(hashtag_str.to_string());
            }
        }
    }

    // Add featured_tags if present
    if let Some(ref featured_tags_str) = actor.as_featured_tags {
        tag_texts.push(featured_tags_str.clone());
    }

    if !tag_texts.is_empty() {
        doc.add_text(tags, &tag_texts.join(" "));
    }

    // Extract and index also_known_as (JSONB array of actor IDs/URIs)
    if let Some(aka_array) = actor.as_also_known_as.as_array() {
        let mut aka_texts = Vec::new();
        for aka in aka_array {
            if let Some(aka_str) = aka.as_str() {
                aka_texts.push(aka_str.to_string());
            }
        }
        if !aka_texts.is_empty() {
            doc.add_text(also_known_as, &aka_texts.join(" "));
        }
    }

    // Flags - determine if actor is local by checking if as_id contains the current server domain
    let is_local_value = actor.as_id.contains(&*crate::SERVER_NAME);
    doc.add_bool(is_local, is_local_value);
    doc.add_bool(is_discoverable, actor.as_discoverable);

    Ok(doc)
}

/// Index an object
pub fn index_object(writer: &mut IndexWriter, object: &Object, schema: &Schema) -> Result<()> {
    // Delete any existing document with this ID to prevent duplicates
    let id_field = schema.get_field("id").unwrap();
    let term = Term::from_field_text(id_field, &object.id.to_string());
    writer.delete_term(term);

    // Add the document
    let doc = object_to_document(object, schema)?;
    writer.add_document(doc)?;
    Ok(())
}

/// Index an actor
pub fn index_actor(writer: &mut IndexWriter, actor: &Actor, schema: &Schema) -> Result<()> {
    // Delete any existing document with this ID to prevent duplicates
    let id_field = schema.get_field("id").unwrap();
    let term = Term::from_field_text(id_field, &actor.id.to_string());
    writer.delete_term(term);

    // Add the document
    let doc = actor_to_document(actor, schema)?;
    writer.add_document(doc)?;
    Ok(())
}

/// Delete an object from the index
pub fn delete_object(writer: &mut IndexWriter, object_id: &str, schema: &Schema) -> Result<()> {
    let id_field = schema.get_field("id").unwrap();
    let term = Term::from_field_text(id_field, object_id);
    writer.delete_term(term);
    Ok(())
}

/// Delete an actor from the index
pub fn delete_actor(writer: &mut IndexWriter, actor_id: &str, schema: &Schema) -> Result<()> {
    let id_field = schema.get_field("id").unwrap();
    let term = Term::from_field_text(id_field, actor_id);
    writer.delete_term(term);
    Ok(())
}
