use crate::axum_server::extractors::AxumSigned;
use crate::axum_server::AppState;
use crate::models::activities::{TimelineFilters, TimelineView};
use crate::models::follows::get_leaders_by_follower_actor_id;
use crate::retriever;
use crate::routes::inbox::{add_hash_to_tags, convert_hashtags_to_query_string, InboxView};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json as AxumJson;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct InboxQuery {
    pub min: Option<i64>,
    pub max: Option<i64>,
    pub limit: u8,
    pub view: Option<InboxView>,
    pub hashtags: Option<Vec<String>>,
}

pub async fn axum_shared_inbox_get(
    State(app_state): State<AppState>,
    AxumSigned(signed): AxumSigned,
    Query(params): Query<InboxQuery>,
) -> Response {
    let conn = match app_state.db_pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            log::error!("Failed to get DB connection from pool: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "Database unavailable").into_response();
        }
    };

    let profile = signed.profile();
    let server_url = format!("https://{}", *crate::SERVER_NAME);

    let view_query = {
        if let Some(view) = params.view.clone() {
            format!("&view={view}")
        } else {
            String::new()
        }
    };

    let hashtags_query = {
        if let Some(hashtags) = params.hashtags.clone() {
            convert_hashtags_to_query_string(&hashtags)
        } else {
            String::new()
        }
    };

    let base_url = format!(
        "{server_url}/inbox?page=true&limit={}{view_query}{hashtags_query}",
        params.limit
    );

    let hashtags = if let Some(hashtags) = params.hashtags.clone() {
        add_hash_to_tags(&hashtags)
    } else {
        vec![]
    };

    let filters = if let Some(view) = params.view {
        match view {
            InboxView::Global => TimelineFilters {
                view: Some(view.into()),
                hashtags,
                username: None,
                conversation: None,
                excluded_words: vec![],
                direct: false,
            },
            InboxView::Home => TimelineFilters {
                view: if let Some(profile) = profile.clone() {
                    match get_leaders_by_follower_actor_id(&conn, profile.id, None).await {
                        Ok(leaders) => Some(TimelineView::Home(
                            leaders
                                .iter()
                                .filter_map(|leader| leader.1.clone()?.as_followers)
                                .collect(),
                        )),
                        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                } else {
                    Some(TimelineView::Global)
                },
                hashtags,
                username: None,
                conversation: None,
                excluded_words: vec![],
                direct: false,
            },
            InboxView::Local => TimelineFilters {
                view: Some(view.into()),
                hashtags,
                username: None,
                conversation: None,
                excluded_words: vec![],
                direct: false,
            },
            InboxView::Direct => TimelineFilters {
                view: Some(view.into()),
                hashtags,
                username: None,
                conversation: None,
                excluded_words: vec![],
                direct: true,
            },
        }
    } else {
        TimelineFilters {
            view: Some(TimelineView::Global),
            hashtags,
            username: None,
            conversation: None,
            excluded_words: vec![],
            direct: false,
        }
    };

    let result = retriever::activities(
        &conn,
        params.limit.into(),
        params.min,
        params.max,
        profile,
        filters,
        Some(base_url),
    )
    .await;

    (StatusCode::OK, AxumJson(result)).into_response()
}
