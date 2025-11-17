use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use axum_extra::extract::Query;
use serde::{Deserialize, Serialize};

use crate::models::actors::Actor;
use crate::models::objects::Object;
use crate::db::runner::DbRunner;
use crate::search::{SearchContext, SearchFilters};
use crate::server::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    /// Search query string
    pub q: String,
    /// Filter by type: "actor", "article", "note", or "question" (can specify multiple)
    #[serde(rename = "type")]
    pub search_type: Option<Vec<String>>,
    /// Maximum number of results per type
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Whether to resolve remote resources (not applicable for Tantivy)
    #[allow(dead_code)]
    pub resolve: Option<bool>,
    /// Filter by account ID
    pub account_id: Option<String>,
    /// Only local results
    pub local: Option<bool>,
    /// Sort order: "relevance" (default), "asc", "desc"
    pub order: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ActorResult {
    pub id: String,
    pub username: String,
    pub acct: String,
    pub display_name: String,
    pub url: String,
    pub avatar: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ObjectResult {
    pub id: String,
    pub uri: String,
    pub url: String,
    pub object_type: String,
    pub content: String,
    pub published: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResults {
    pub actors: Vec<ActorResult>,
    pub objects: Vec<ObjectResult>,
}

/// Convert Actor to search result format
fn actor_to_actor_result(actor: &Actor) -> ActorResult {
    let username = actor.as_preferred_username.as_deref().unwrap_or("unknown");
    let display_name = actor.as_name.as_deref().unwrap_or(username);

    // Use the stored webfinger if available, otherwise construct it
    let acct = if let Some(ref webfinger) = actor.ek_webfinger {
        webfinger.clone()
    } else {
        // Fallback: construct from username and server name
        let is_local = actor.as_id.contains(&*crate::SERVER_NAME);
        if is_local {
            format!("{}@{}", username, *crate::SERVER_NAME)
        } else if let Some(domain) = crate::helper::get_domain_from_url(actor.as_id.clone()) {
            format!("{}@{}", username, domain)
        } else {
            username.to_string()
        }
    };

    // Extract avatar URL from as_icon JSONB field
    let avatar = actor.as_icon
        .as_object()
        .and_then(|obj| obj.get("url"))
        .and_then(|url| url.as_str())
        .map(|s| s.to_string());

    ActorResult {
        id: actor.id.to_string(),
        username: username.to_string(),
        acct,
        display_name: display_name.to_string(),
        url: actor.as_id.clone(),
        avatar,
    }
}

/// Convert Object to search result format
fn object_to_object_result(object: &Object) -> ObjectResult {
    let content = object.as_content.as_deref().unwrap_or("");

    // Extract URL from as_url field (JSONB - could be string, array, or object)
    let url = if let Some(ref as_url) = object.as_url {
        // Try to extract as string
        if let Some(url_str) = as_url.as_str() {
            url_str.to_string()
        } else if let Some(url_array) = as_url.as_array() {
            // If it's an array, take the first URL
            url_array
                .first()
                .and_then(|v| v.as_str())
                .unwrap_or(&object.as_id)
                .to_string()
        } else if let Some(url_obj) = as_url.as_object() {
            // If it's an object with href field
            url_obj
                .get("href")
                .and_then(|v| v.as_str())
                .unwrap_or(&object.as_id)
                .to_string()
        } else {
            object.as_id.clone()
        }
    } else {
        // Fallback to as_id if as_url is not set
        object.as_id.clone()
    };

    // Use as_published if available, otherwise fall back to created_at
    let published = object
        .as_published
        .unwrap_or(object.created_at)
        .to_rfc3339();

    ObjectResult {
        id: object.id.to_string(),
        uri: object.as_id.clone(),
        url,
        object_type: format!("{:?}", object.as_type),
        content: content.to_string(),
        published,
    }
}

/// Search API
/// Note: Authentication is optional for search
pub async fn search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResults>, StatusCode> {
    // Build search context (unauthenticated for now)
    // TODO: Add authentication support to filter results based on user permissions
    let context = SearchContext::default();

    // Parse sort order
    use crate::search::SortOrder;
    let sort_order = match query.order.as_deref() {
        Some("asc") => SortOrder::Asc,
        Some("desc") => SortOrder::Desc,
        _ => SortOrder::Relevance, // Default to relevance
    };

    // Parse type filters
    let (search_actors, search_objects, object_types) = if let Some(ref types) = query.search_type {
        let mut search_actors = false;
        let mut object_types = Vec::new();

        for type_str in types {
            match type_str.to_lowercase().as_str() {
                "actor" => search_actors = true,
                "article" => object_types.push("Article".to_string()),
                "note" => object_types.push("Note".to_string()),
                "question" => object_types.push("Question".to_string()),
                _ => {
                    // Ignore unknown types
                    log::warn!("Unknown search type: {}", type_str);
                }
            }
        }

        let search_objects = !object_types.is_empty();
        (search_actors, search_objects, if object_types.is_empty() { None } else { Some(object_types) })
    } else {
        // No type filter - search everything (actors and all objects without filtering)
        (true, true, None)
    };

    // Build search filters
    let filters = SearchFilters {
        content_type: object_types,
        since: None,
        until: None,
        author_id: query.account_id.clone(),
        local_only: query.local.unwrap_or(false),
        sort_order,
    };

    let limit = query.limit.unwrap_or(20).min(40); // Cap at 40
    let offset = query.offset.unwrap_or(0);

    // Perform search based on type filter
    let (actors, objects) = if search_actors && search_objects {
        // Search both actors and objects
        let actor_results = state
            .search_index
            .search_actors(&query.q, &context, &filters, limit, offset)
            .map_err(|e| {
                log::error!("Search error: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let object_results = state
            .search_index
            .search_objects(&query.q, &context, &filters, limit, offset)
            .map_err(|e| {
                log::error!("Search error: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let actors = hydrate_actors(&state, actor_results).await?;
        let objects = hydrate_objects(&state, object_results).await?;

        (actors, objects)
    } else if search_actors {
        // Only search actors
        let actor_results = state
            .search_index
            .search_actors(&query.q, &context, &filters, limit, offset)
            .map_err(|e| {
                log::error!("Search error: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let actors = hydrate_actors(&state, actor_results).await?;
        (actors, Vec::new())
    } else if search_objects {
        // Only search objects
        let object_results = state
            .search_index
            .search_objects(&query.q, &context, &filters, limit, offset)
            .map_err(|e| {
                log::error!("Search error: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let objects = hydrate_objects(&state, object_results).await?;
        (Vec::new(), objects)
    } else {
        // No valid type specified - search nothing
        (Vec::new(), Vec::new())
    };

    Ok(Json(SearchResults {
        actors,
        objects,
    }))
}

/// Hydrate actor results from database
async fn hydrate_actors(
    state: &AppState,
    results: Vec<crate::search::ActorSearchResult>,
) -> Result<Vec<ActorResult>, StatusCode> {
    let mut actors = Vec::new();

    for result in results {
        // Parse ID as integer
        let actor_id: i32 = result.id.parse().map_err(|e| {
            log::error!("Failed to parse actor ID: {:#?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Fetch from database
        let actor: Option<Actor> = state
            .db_pool
            .get()
            .await
            .map_err(|e| {
                log::error!("Failed to get DB connection: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .run(move |c| {
                use diesel::prelude::*;
                use crate::schema::actors;

                actors::table
                    .filter(actors::id.eq(actor_id))
                    .first::<Actor>(c)
                    .optional()
            })
            .await
            .map_err(|e| {
                log::error!("Failed to fetch actor: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(actor) = actor {
            actors.push(actor_to_actor_result(&actor));
        }
    }

    Ok(actors)
}

/// Hydrate object results from database
async fn hydrate_objects(
    state: &AppState,
    results: Vec<crate::search::ObjectSearchResult>,
) -> Result<Vec<ObjectResult>, StatusCode> {
    let mut objects = Vec::new();

    for result in results {
        // Parse ID as integer
        let object_id: i32 = result.id.parse().map_err(|e| {
            log::error!("Failed to parse object ID: {:#?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Fetch from database
        let object: Option<Object> = state
            .db_pool
            .get()
            .await
            .map_err(|e| {
                log::error!("Failed to get DB connection: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
            .run(move |c| {
                use diesel::prelude::*;
                use crate::schema::objects;

                objects::table
                    .filter(objects::id.eq(object_id))
                    .first::<Object>(c)
                    .optional()
            })
            .await
            .map_err(|e| {
                log::error!("Failed to fetch object: {:#?}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(object) = object {
            objects.push(object_to_object_result(&object));
        }
    }

    Ok(objects)
}
