use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use clap::Parser;

// Embed the compiled binaries at compile time
const ENIGMATICK_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/enigmatick"));
const PROXY_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/proxy"));
const TASKS_BIN: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/tasks"));

#[derive(Parser)]
#[command(name = "enigmatick")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A federated communication platform server", long_about = None)]
pub enum Commands {
    /// Start the web server and background tasks
    Server,
    /// Run database migrations
    Migrate,
    /// Initialize folder structure for media files
    Init,
    /// Generate .env.template file
    Template,
    /// Manage cached media files
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    /// Create or ensure system user exists
    SystemUser,
    /// Manage federated instances
    Instances {
        #[command(subcommand)]
        command: InstanceCommand,
    },
    /// Manage search index
    Search {
        #[command(subcommand)]
        command: SearchCommand,
    },
    /// Send various activities
    Send {
        #[command(subcommand)]
        command: SendCommand,
    },
    /// Manage user muted terms
    MutedTerms {
        #[command(subcommand)]
        command: MutedTermsCommand,
    },
}

#[derive(Parser)]
pub enum CacheCommand {
    /// Prune cached files older than specified duration (e.g., 30d, 2m, 1y)
    Prune {
        /// Duration string (e.g., "30d", "2m", "1y")
        duration: String,
    },
    /// Delete cached item by URL
    Delete {
        /// URL of the cached item to delete
        url: String,
    },
    /// Delete cached items from server/domain pattern
    DeleteServer {
        /// Server pattern to match (e.g., "domain.name")
        pattern: String,
    },
}

#[derive(Parser)]
pub enum InstanceCommand {
    List,
    Block { domain: String },
    Unblock { domain: String },
}

#[derive(Parser)]
pub enum SearchCommand {
    /// Rebuild search index from database (full reindex)
    Index,
    /// Update search index incrementally (only new/updated content)
    Update,
    /// Show search index statistics
    Status,
    /// Optimize search index
    Optimize,
}

#[derive(Parser)]
pub enum SendCommand {
    /// Send update activities
    Update {
        #[command(subcommand)]
        command: UpdateCommand,
    },
    /// Send delete activities
    Delete {
        #[command(subcommand)]
        command: DeleteCommand,
    },
}

#[derive(Parser)]
pub enum UpdateCommand {
    /// Send actor update to all known instances
    Actor { username: String },
}

#[derive(Parser)]
pub enum DeleteCommand {
    /// Send actor delete to all known instances
    Actor { username: String },
}

#[derive(Parser)]
pub enum MutedTermsCommand {
    /// List muted terms for user
    List {
        /// Username to list terms for
        username: String,
    },
    /// Add muted term for user
    Add {
        /// Username to modify
        username: String,
        /// Term to mute
        term: String,
    },
    /// Remove muted term for user
    Remove {
        /// Username to modify
        username: String,
        /// Term to unmute
        term: String,
    },
    /// Clear all muted terms for user
    Clear {
        /// Username to clear terms for
        username: String,
    },
}

fn extract_and_run_binary(binary_data: &[u8], name: &str, args: &[String]) -> std::io::Result<Child> {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    
    let temp_dir = env::temp_dir();
    let binary_path = temp_dir.join(format!("{}-{}", name, std::process::id()));
    
    // Write the binary to a temporary file
    fs::write(&binary_path, binary_data)?;
    
    // Make it executable
    let mut perms = fs::metadata(&binary_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&binary_path, perms)?;
    
    // Execute it
    let child = Command::new(&binary_path)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    
    // Clean up the temporary file in a separate thread
    let path_clone = binary_path.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let _ = fs::remove_file(path_clone);
    });
    
    Ok(child)
}

fn handle_server_command() {
    log::info!("[Launcher] Starting Enigmatick server...");
    
    // Extract and run proxy if needed
    let mut proxy_handle = if env::var("ACME_PROXY")
        .map(|x| x.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        Some(extract_and_run_binary(PROXY_BIN, "proxy", &[]).expect("Failed to start proxy"))
    } else {
        None
    };

    if let Some(handle) = proxy_handle.as_ref() {
        log::info!("[Launcher] Proxy process started with PID: {}", handle.id());
    }

    // Extract and run tasks
    let mut tasks_handle = extract_and_run_binary(TASKS_BIN, "tasks", &[])
        .expect("Failed to start tasks");
    log::info!("[Launcher] Tasks process started with PID: {}", tasks_handle.id());
    
    // Extract and run main app
    let mut app_handle = extract_and_run_binary(ENIGMATICK_BIN, "enigmatick", &["app".to_string()])
        .expect("Failed to start app");
    log::info!("[Launcher] Application process started with PID: {}", app_handle.id());

    // Graceful shutdown handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        log::info!("\n[Launcher] Shutdown signal received. Terminating child processes...");
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        if let Ok(Some(status)) = app_handle.try_wait() {
            log::info!("[Launcher] Application process exited with status: {status}. Shutting down.");
            break;
        }
        if let Some(ref mut proxy) = proxy_handle {
            if let Ok(Some(status)) = proxy.try_wait() {
                log::info!("[Launcher] Proxy process exited with status: {status}. Shutting down.");
                break;
            }
        }
        if let Ok(Some(status)) = tasks_handle.try_wait() {
            log::info!("[Launcher] Tasks process exited with status: {status}. Shutting down.");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    log::info!("[Launcher] Cleaning up child processes...");
    if let Some(ref mut proxy) = proxy_handle {
        if let Err(e) = proxy.kill() {
            log::error!("[Launcher] Error killing proxy process: {e}");
        }
        let _ = proxy.wait();
    }
    if let Err(e) = tasks_handle.kill() {
        log::error!("[Launcher] Error killing tasks process: {e}");
    }
    let _ = tasks_handle.wait();
    if let Err(e) = app_handle.kill() {
        log::error!("[Launcher] Error killing app process: {e}");
    }
    let _ = app_handle.wait();
    log::info!("[Launcher] Shutdown complete.");
}

fn delegate_to_enigmatick(args: Vec<String>) -> ! {
    let mut child = extract_and_run_binary(ENIGMATICK_BIN, "enigmatick", &args)
        .expect("Failed to run enigmatick");
    
    let status = child.wait().expect("Failed to wait for enigmatick");
    std::process::exit(status.code().unwrap_or(1));
}

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenvy::dotenv().ok();
    
    let args = Commands::parse();
    
    match args {
        Commands::Server => handle_server_command(),
        Commands::Migrate => delegate_to_enigmatick(vec!["migrate".to_string()]),
        Commands::Init => delegate_to_enigmatick(vec!["init".to_string()]),
        Commands::Template => delegate_to_enigmatick(vec!["template".to_string()]),
        Commands::SystemUser => delegate_to_enigmatick(vec!["system-user".to_string()]),
        Commands::Cache { command } => {
            let mut args = vec!["cache".to_string()];
            match command {
                CacheCommand::Prune { duration } => {
                    args.extend(vec!["prune".to_string(), duration]);
                }
                CacheCommand::Delete { url } => {
                    args.extend(vec!["delete".to_string(), url]);
                }
                CacheCommand::DeleteServer { pattern } => {
                    args.extend(vec!["delete-server".to_string(), pattern]);
                }
            }
            delegate_to_enigmatick(args);
        }
        Commands::Instances { command } => {
            let mut args = vec!["instances".to_string()];
            match command {
                InstanceCommand::List => args.push("list".to_string()),
                InstanceCommand::Block { domain } => {
                    args.push("block".to_string());
                    args.push(domain);
                }
                InstanceCommand::Unblock { domain } => {
                    args.push("unblock".to_string());
                    args.push(domain);
                }
            }
            delegate_to_enigmatick(args);
        }
        Commands::Search { command } => {
            let mut args = vec!["search".to_string()];
            match command {
                SearchCommand::Index => args.push("index".to_string()),
                SearchCommand::Update => args.push("update".to_string()),
                SearchCommand::Status => args.push("status".to_string()),
                SearchCommand::Optimize => args.push("optimize".to_string()),
            }
            delegate_to_enigmatick(args);
        }
        Commands::Send { command } => {
            let mut args = vec!["send".to_string()];
            match command {
                SendCommand::Update { command } => {
                    args.push("update".to_string());
                    match command {
                        UpdateCommand::Actor { username } => {
                            args.extend(vec!["actor".to_string(), username]);
                        }
                    }
                }
                SendCommand::Delete { command } => {
                    args.push("delete".to_string());
                    match command {
                        DeleteCommand::Actor { username } => {
                            args.extend(vec!["actor".to_string(), username]);
                        }
                    }
                }
            }
            delegate_to_enigmatick(args);
        }
        Commands::MutedTerms { command } => {
            let mut args = vec!["muted-terms".to_string()];
            match command {
                MutedTermsCommand::List { username } => {
                    args.extend(vec!["list".to_string(), username]);
                }
                MutedTermsCommand::Add { username, term } => {
                    args.extend(vec!["add".to_string(), username, term]);
                }
                MutedTermsCommand::Remove { username, term } => {
                    args.extend(vec!["remove".to_string(), username, term]);
                }
                MutedTermsCommand::Clear { username } => {
                    args.extend(vec!["clear".to_string(), username]);
                }
            }
            delegate_to_enigmatick(args);
        }
    }
}
