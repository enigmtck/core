use tantivy::schema::*;

/// Creates the schema for the objects (posts/notes/articles/questions) index
pub fn create_objects_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Stored fields for retrieval
    schema_builder.add_text_field("id", STRING | STORED);
    schema_builder.add_text_field("as_id", STRING | STORED);
    schema_builder.add_text_field("object_type", STRING | STORED);
    schema_builder.add_text_field("conversation_id", STRING | STORED);

    // Searchable text fields
    schema_builder.add_text_field("content", TEXT);
    schema_builder.add_text_field("name", TEXT);
    schema_builder.add_text_field("summary", TEXT);
    schema_builder.add_text_field("author_username", TEXT);

    // Faceted/filterable fields
    schema_builder.add_facet_field("author_id", INDEXED | STORED);
    schema_builder.add_facet_field("type_facet", INDEXED);
    schema_builder.add_facet_field("visibility", INDEXED);

    // Date field for sorting (FAST enables efficient sorting)
    schema_builder.add_date_field("published", INDEXED | STORED | FAST);

    schema_builder.build()
}

/// Creates the schema for the actors (users/profiles) index
pub fn create_actors_schema() -> Schema {
    let mut schema_builder = Schema::builder();

    // Stored fields for retrieval
    schema_builder.add_text_field("id", STRING | STORED);
    schema_builder.add_text_field("as_id", STRING | STORED);
    schema_builder.add_text_field("actor_type", STRING | STORED);

    // Searchable text fields
    schema_builder.add_text_field("username", TEXT);
    schema_builder.add_text_field("display_name", TEXT);
    schema_builder.add_text_field("summary", TEXT);
    schema_builder.add_text_field("tags", TEXT);  // Extracted from as_tag and ek_hashtags
    schema_builder.add_text_field("also_known_as", TEXT);  // Previous/alternate identities

    // Boolean fields for filtering
    schema_builder.add_bool_field("is_local", INDEXED | STORED);
    schema_builder.add_bool_field("is_discoverable", INDEXED | STORED);

    // Faceted field for actor type
    schema_builder.add_facet_field("type_facet", INDEXED);

    schema_builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objects_schema_creation() {
        let schema = create_objects_schema();
        assert!(schema.get_field("id").is_some());
        assert!(schema.get_field("content").is_some());
        assert!(schema.get_field("author_id").is_some());
    }

    #[test]
    fn test_actors_schema_creation() {
        let schema = create_actors_schema();
        assert!(schema.get_field("id").is_some());
        assert!(schema.get_field("username").is_some());
        assert!(schema.get_field("is_discoverable").is_some());
    }
}
