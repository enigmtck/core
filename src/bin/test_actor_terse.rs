use jdt_activity_pub::ApActorTerse;
use serde_json;
use std::env;
use std::io::{self, Read};

fn main() {
    let args: Vec<String> = env::args().collect();

    let json_input = if args.len() > 1 {
        args[1].clone()
    } else {
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
        std::process::exit(1);
    }

    println!("📝 Testing JSON deserialization to Vec<ApActorTerse>");
    println!("🔧 Input JSON:");
    println!("{}", json_input);
    println!();

    // Try to parse as JSON first
    match serde_json::from_str::<serde_json::Value>(&json_input) {
        Ok(value) => {
            println!("✅ Valid JSON structure");
            println!("🔍 Pretty printed:");
            println!(
                "{}",
                serde_json::to_string_pretty(&value)
                    .unwrap_or_else(|_| "Error pretty printing".to_string())
            );
            println!();
        }
        Err(e) => {
            eprintln!("❌ Invalid JSON: {}", e);
            std::process::exit(1);
        }
    }

    // Try to deserialize to Vec<ApActorTerse>
    match serde_json::from_str::<Vec<ApActorTerse>>(&json_input) {
        Ok(actors) => {
            println!(
                "🎉 Successfully deserialized to {} ApActorTerse objects!",
                actors.len()
            );
            println!();

            for (i, actor) in actors.iter().enumerate() {
                println!("📋 Actor {} fields:", i + 1);
                println!("  ID: {}", actor.id);
                println!("  Preferred Username: {}", actor.preferred_username);
                println!("  Name: {:?}", actor.name);
                println!("  Icon: {:?}", actor.icon);
                println!("  URL: {:?}", actor.url);
                println!("  Tag: {:?}", actor.tag);
                println!("  Webfinger: {:?}", actor.webfinger);
                println!();
            }

            // Show the serialized version
            match serde_json::to_string_pretty(&actors) {
                Ok(serialized) => {
                    println!("🔄 Re-serialized Vec<ApActorTerse>:");
                    println!("{}", serialized);
                }
                Err(e) => {
                    eprintln!("⚠️  Could not re-serialize Vec<ApActorTerse>: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Failed to deserialize to Vec<ApActorTerse>: {}", e);
            println!();
            println!("💡 This might help debug the issue:");
            println!("   - Check if required fields are present (id, preferredUsername)");
            println!("   - Verify field names match expected camelCase format");
            println!("   - Ensure icon field handles empty objects {{}} and missing fields");
            std::process::exit(1);
        }
    }
}
