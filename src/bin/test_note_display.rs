use jdt_activity_pub::{ApNote, ApNoteType, ApAddress, ApContext};
use chrono::Utc;

fn main() {
    println!("🧪 Testing ApNote Display and Debug implementations");
    println!();

    // Test 1: ApNote with name
    let note_with_name = ApNote {
        context: Some(ApContext::default()),
        kind: ApNoteType::Note,
        attributed_to: ApAddress::Address("https://example.com/users/alice".to_string()),
        id: Some("https://example.com/notes/123".to_string()),
        name: Some("Test Note with Name".to_string()),
        content: Some("This is a test note with a name field for question replies".to_string()),
        // Use default implementation for 'to' field
        published: Utc::now().into(),
        ..Default::default()
    };

    println!("📋 Note with name:");
    println!("  Display: {}", note_with_name);
    println!("  Debug: {:?}", note_with_name);
    println!();

    // Test 2: ApNote without name
    let note_without_name = ApNote {
        context: Some(ApContext::default()),
        kind: ApNoteType::Note,
        attributed_to: ApAddress::Address("https://example.com/users/bob".to_string()),
        id: Some("https://example.com/notes/456".to_string()),
        name: None,
        content: Some("This is a regular note without a name field".to_string()),
        // Use default implementation for 'to' field
        published: Utc::now().into(),
        ..Default::default()
    };

    println!("📋 Note without name:");
    println!("  Display: {}", note_without_name);
    println!("  Debug: {:?}", note_without_name);
    println!();

    // Test 3: ApNote with name but no ID
    let note_name_no_id = ApNote {
        context: Some(ApContext::default()),
        kind: ApNoteType::Note,
        attributed_to: ApAddress::Address("https://example.com/users/charlie".to_string()),
        id: None,
        name: Some("Vote: Option A".to_string()),
        content: Some("This represents a vote selection".to_string()),
        // Use default implementation for 'to' field
        published: Utc::now().into(),
        ..Default::default()
    };

    println!("📋 Note with name but no ID:");
    println!("  Display: {}", note_name_no_id);
    println!("  Debug: {:?}", note_name_no_id);
}