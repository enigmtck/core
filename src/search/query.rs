use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{Index, ReloadPolicy};
use serde::{Deserialize, Serialize};

/// Search context for privacy filtering
#[derive(Clone, Debug)]
pub struct SearchContext {
    /// The authenticated user making the search (None for anonymous)
    pub user_id: Option<String>,
    /// Whether this user is an admin
    pub is_admin: bool,
    /// List of blocked actor IDs
    pub blocked_actors: Vec<String>,
}

impl Default for SearchContext {
    fn default() -> Self {
        Self {
            user_id: None,
            is_admin: false,
            blocked_actors: Vec::new(),
        }
    }
}

/// Sort order for search results
#[derive(Clone, Debug, Default, PartialEq)]
pub enum SortOrder {
    #[default]
    Relevance,
    Asc,
    Desc,
}

/// Search filters
#[derive(Clone, Debug, Default)]
pub struct SearchFilters {
    /// Filter by object/actor type (multiple types allowed)
    pub content_type: Option<Vec<String>>,
    /// Minimum date
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    /// Maximum date
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by author
    pub author_id: Option<String>,
    /// Only local content
    pub local_only: bool,
    /// Sort order
    pub sort_order: SortOrder,
}

/// Search result for an object
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObjectSearchResult {
    pub id: String,
    pub as_id: String,
    pub object_type: String,
    pub score: f32,
}

/// Search result for an actor
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorSearchResult {
    pub id: String,
    pub as_id: String,
    pub actor_type: String,
    pub score: f32,
}

/// Combined search results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResults {
    pub objects: Vec<ObjectSearchResult>,
    pub actors: Vec<ActorSearchResult>,
}

/// Search objects index
pub fn search_objects(
    index: &Index,
    query_str: &str,
    context: &SearchContext,
    filters: &SearchFilters,
    limit: usize,
    offset: usize,
) -> Result<Vec<ObjectSearchResult>> {
    let schema = index.schema();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;
    let searcher = reader.searcher();

    // Build query parser for text fields
    let content_field = schema.get_field("content").unwrap();
    let name_field = schema.get_field("name").unwrap();
    let summary_field = schema.get_field("summary").unwrap();
    let author_username_field = schema.get_field("author_username").unwrap();

    let query_parser = QueryParser::for_index(
        index,
        vec![content_field, name_field, summary_field, author_username_field],
    );

    let text_query = query_parser.parse_query(query_str)?;

    // Build filter queries
    let mut boolean_queries: Vec<(Occur, Box<dyn Query>)> = vec![];
    boolean_queries.push((Occur::Must, text_query));

    // Filter by type if specified (supports multiple types with OR logic)
    if let Some(ref obj_types) = filters.content_type {
        if !obj_types.is_empty() {
            log::debug!("Filtering objects by types: {:?}", obj_types);
            let type_field = schema.get_field("type_facet").unwrap();

            if obj_types.len() == 1 {
                // Single type: add directly as Must
                let facet = Facet::from(&format!("/{}", obj_types[0]));
                log::debug!("Created facet filter: {:?}", facet);
                let term = Term::from_facet(type_field, &facet);
                boolean_queries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
            } else {
                // Multiple types: create OR query (Should)
                let mut type_queries: Vec<(Occur, Box<dyn Query>)> = vec![];
                for obj_type in obj_types {
                    let facet = Facet::from(&format!("/{}", obj_type));
                    log::debug!("Created facet filter: {:?}", facet);
                    let term = Term::from_facet(type_field, &facet);
                    type_queries.push((Occur::Should, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
                }
                let type_query = BooleanQuery::new(type_queries);
                boolean_queries.push((Occur::Must, Box::new(type_query)));
            }
        }
    } else {
        log::debug!("No type filter applied - searching all object types");
    }

    // Filter by author if specified
    if let Some(ref author) = filters.author_id {
        let author_field = schema.get_field("author_id").unwrap();
        let facet = Facet::from(&format!("/{}", author));
        let term = Term::from_facet(author_field, &facet);
        boolean_queries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
    }

    // Exclude blocked actors
    for blocked_id in &context.blocked_actors {
        let author_field = schema.get_field("author_id").unwrap();
        let facet = Facet::from(&format!("/{}", blocked_id));
        let term = Term::from_facet(author_field, &facet);
        boolean_queries.push((Occur::MustNot, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
    }

    // Exclude Tombstone objects (deleted content)
    let type_field = schema.get_field("type_facet").unwrap();
    let tombstone_facet = Facet::from("/Tombstone");
    let tombstone_term = Term::from_facet(type_field, &tombstone_facet);
    boolean_queries.push((Occur::MustNot, Box::new(TermQuery::new(tombstone_term, IndexRecordOption::Basic))));

    let final_query = BooleanQuery::new(boolean_queries);

    // Extract results - different handling based on sort order
    let id_field = schema.get_field("id").unwrap();
    let as_id_field = schema.get_field("as_id").unwrap();
    let type_field = schema.get_field("object_type").unwrap();

    let mut results = Vec::new();

    match &filters.sort_order {
        SortOrder::Relevance => {
            // Default: sort by relevance (BM25 score)
            let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit).and_offset(offset))?;
            for (score, doc_address) in top_docs {
                let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

                let id = retrieved_doc
                    .get_first(id_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let as_id = retrieved_doc
                    .get_first(as_id_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let object_type = retrieved_doc
                    .get_first(type_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                results.push(ObjectSearchResult {
                    id,
                    as_id,
                    object_type,
                    score,
                });
            }
        }
        SortOrder::Asc | SortOrder::Desc => {
            // Sort by published date (DateTime is Tantivy's date type)
            use tantivy::Order;
            let order = if filters.sort_order == SortOrder::Asc {
                Order::Asc
            } else {
                Order::Desc
            };

            let top_docs = searcher.search(
                &final_query,
                &TopDocs::with_limit(limit)
                    .and_offset(offset)
                    .order_by_fast_field::<tantivy::DateTime>("published", order),
            )?;

            for (_timestamp, doc_address) in top_docs {
                let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

                let id = retrieved_doc
                    .get_first(id_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let as_id = retrieved_doc
                    .get_first(as_id_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let object_type = retrieved_doc
                    .get_first(type_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                results.push(ObjectSearchResult {
                    id,
                    as_id,
                    object_type,
                    score: 0.0, // No relevance score when sorting by date
                });
            }
        }
    }

    Ok(results)
}

/// Search actors index
pub fn search_actors(
    index: &Index,
    query_str: &str,
    _context: &SearchContext,
    filters: &SearchFilters,
    limit: usize,
    offset: usize,
) -> Result<Vec<ActorSearchResult>> {
    let schema = index.schema();
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;
    let searcher = reader.searcher();

    // Build query parser for text fields
    let username_field = schema.get_field("username").unwrap();
    let display_name_field = schema.get_field("display_name").unwrap();
    let summary_field = schema.get_field("summary").unwrap();
    let tags_field = schema.get_field("tags").unwrap();
    let also_known_as_field = schema.get_field("also_known_as").unwrap();

    let query_parser = QueryParser::for_index(
        index,
        vec![username_field, display_name_field, summary_field, tags_field, also_known_as_field],
    );

    let text_query = query_parser.parse_query(query_str)?;

    // Build filter queries
    let mut boolean_queries: Vec<(Occur, Box<dyn Query>)> = vec![];
    boolean_queries.push((Occur::Must, text_query));

    // Only show discoverable actors (privacy filter)
    let discoverable_field = schema.get_field("is_discoverable").unwrap();
    let term = Term::from_field_bool(discoverable_field, true);
    boolean_queries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));

    // Filter by local only if specified
    if filters.local_only {
        let local_field = schema.get_field("is_local").unwrap();
        let term = Term::from_field_bool(local_field, true);
        boolean_queries.push((Occur::Must, Box::new(TermQuery::new(term, IndexRecordOption::Basic))));
    }

    let final_query = BooleanQuery::new(boolean_queries);

    // Execute search with offset
    let top_docs = searcher.search(&final_query, &TopDocs::with_limit(limit).and_offset(offset))?;

    // Extract results
    let id_field = schema.get_field("id").unwrap();
    let as_id_field = schema.get_field("as_id").unwrap();
    let type_field = schema.get_field("actor_type").unwrap();

    let mut results = Vec::new();
    for (score, doc_address) in top_docs {
        let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

        let id = retrieved_doc
            .get_first(id_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let as_id = retrieved_doc
            .get_first(as_id_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let actor_type = retrieved_doc
            .get_first(type_field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        results.push(ActorSearchResult {
            id,
            as_id,
            actor_type,
            score,
        });
    }

    Ok(results)
}
