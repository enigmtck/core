use jdt_activity_pub::ApNote;
use serde_json;
use std::env;
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        print_help();
        return;
    }

    let json_input = if args.len() > 1 {
        // Use command line argument as JSON
        args[1].clone()
    } else {
        // Read from stdin
        let mut buffer = String::new();
        match io::stdin().read_to_string(&mut buffer) {
            Ok(_) => buffer.trim().to_string(),
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                std::process::exit(1);
            }
        }
    };

    if json_input.is_empty() {
        eprintln!("Error: No JSON input provided");
        print_help();
        std::process::exit(1);
    }

    println!("📝 Testing JSON deserialization to ApNote");
    println!("🔧 Input JSON:");
    println!("{}", json_input);
    println!();

    // Try to parse as JSON first
    match serde_json::from_str::<serde_json::Value>(&json_input) {
        Ok(value) => {
            println!("✅ Valid JSON structure");
            println!("🔍 Pretty printed:");
            println!("{}", serde_json::to_string_pretty(&value).unwrap_or_else(|_| "Error pretty printing".to_string()));
            println!();
        }
        Err(e) => {
            eprintln!("❌ Invalid JSON: {}", e);
            std::process::exit(1);
        }
    }

    // Try to deserialize to ApNote
    match serde_json::from_str::<ApNote>(&json_input) {
        Ok(note) => {
            println!("🎉 Successfully deserialized to ApNote!");
            println!();
            println!("📋 ApNote fields:");
            println!("  ID: {:?}", note.id);
            println!("  Type: {:?}", note.kind);
            println!("  Name: {:?}", note.name);
            println!("  Content: {:?}", note.content);
            println!("  Published: {:?}", note.published);
            println!("  URL: {:?}", note.url);
            println!("  To: {:?}", note.to);
            println!("  CC: {:?}", note.cc);
            println!("  Tag: {:?}", note.tag);
            println!("  Attributed To: {:?}", note.attributed_to);
            println!("  In Reply To: {:?}", note.in_reply_to);
            println!("  Conversation: {:?}", note.conversation);
            println!("  Attachment: {:?}", note.attachment);
            println!("  Summary: {:?}", note.summary);
            println!("  Sensitive: {:?}", note.sensitive);
            println!("  Ephemeral: {:?}", note.ephemeral);
            println!();
            
            // Show the serialized version
            match serde_json::to_string_pretty(&note) {
                Ok(serialized) => {
                    println!("🔄 Re-serialized ApNote:");
                    println!("{}", serialized);
                }
                Err(e) => {
                    eprintln!("⚠️  Could not re-serialize ApNote: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to deserialize to ApNote: {}", e);
            println!();
            println!("💡 This might help debug the issue:");
            println!("   - Check if required fields are present (type, to, published, attributedTo)");
            println!("   - Verify field names match expected camelCase format");
            println!("   - Ensure URL field format is compatible with ApUrl enum");
            println!("   - Check that the 'type' field is a valid note type");
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("🧪 ApNote JSON Deserialization Tester");
    println!();
    println!("USAGE:");
    println!("  {} [JSON_STRING]", env::args().next().unwrap_or_else(|| "test_note_json".to_string()));
    println!("  echo 'JSON' | {}", env::args().next().unwrap_or_else(|| "test_note_json".to_string()));
    println!();
    println!("EXAMPLES:");
    println!("  # Test with inline JSON:");
    println!("  {} '{{\"type\":\"Note\",\"to\":[\"https://www.w3.org/ns/activitystreams#Public\"],\"published\":\"2023-01-01T00:00:00Z\",\"attributedTo\":\"https://example.com/user\",\"content\":\"Hello world!\"}}'", env::args().next().unwrap_or_else(|| "test_note_json".to_string()));
    println!();
    println!("  # Test with file input:");
    println!("  cat note.json | {}", env::args().next().unwrap_or_else(|| "test_note_json".to_string()));
    println!();
    println!("  # Test URL handling:");
    println!("  {} '{{\"type\":\"Note\",\"to\":[\"https://www.w3.org/ns/activitystreams#Public\"],\"published\":\"2023-01-01T00:00:00Z\",\"attributedTo\":\"https://example.com/user\",\"url\":[\"https://example.com/note/1\",{{\"type\":\"Link\",\"href\":\"https://mirror.com/note/1\",\"rel\":\"canonical\"}}]}}'", env::args().next().unwrap_or_else(|| "test_note_json".to_string()));
    println!();
    println!("OPTIONS:");
    println!("  -h, --help    Show this help message");
}