use clap::Parser;
use std::env;
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod cache;
mod display;
mod instances;
mod muted_terms;
mod send;
mod system;

use cache::{handle_cache_command, CacheArgs};
use instances::{handle_instance_command, InstanceArgs};
use muted_terms::{handle_muted_terms_command, MutedTermsArgs};
use send::{handle_send_command, SendArgs};
use system::{handle_init, handle_migrations, handle_system_user, handle_template};

#[derive(Parser)]
pub enum Commands {
    /// Initialize folder structure for media files
    Init,
    /// Generate .env.template file
    Template,
    /// Run database migrations
    Migrate,
    /// Manage cached media files
    Cache(CacheArgs),
    /// Create or ensure system user exists
    SystemUser,
    /// Start the web server and background tasks
    Server,
    /// Manage federated instances
    Instances(InstanceArgs),
    /// Send various activities
    Send(SendArgs),
    /// Manage user muted terms
    MutedTerms(MutedTermsArgs),
    /// [Internal] Run the application server
    #[command(hide = true)]
    App,
}

#[derive(Parser)]
#[command(name = "enigmatick")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A federated communication platform server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Spawns and manages a child process.
fn spawn_process(path: &std::path::Path, name: &str, args: &[&str]) -> Child {
    Command::new(path)
        .args(args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to start {name} process at {path:?}: {e}"))
}

/// The process manager for the `server` command.
fn handle_server_command() {
    log::info!("[Manager] Starting Enigmatick server...");
    let current_exe = std::env::current_exe().expect("Failed to get current executable path");

    // Find the path to the proxy binary, assuming it's in the same directory
    let mut proxy_path = current_exe.clone();
    proxy_path.set_file_name("proxy");

    // Spawn the proxy process

    let mut proxy_handle = if env::var("ACME_PROXY")
        .map(|x| x.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
    {
        Some(spawn_process(&proxy_path, "proxy", &[]))
    } else {
        None
    };

    if let Some(handle) = proxy_handle.as_ref() {
        log::info!("[Manager] Proxy process started with PID: {}", handle.id());
    }

    // Find the path to the tasks binary, assuming it's in the same directory
    let mut tasks_path = current_exe.clone();
    tasks_path.set_file_name("tasks");

    // Spawn the tasks process
    let mut tasks_handle = spawn_process(&tasks_path, "tasks", &[]);
    log::info!("[Manager] Tasks process started with PID: {}", tasks_handle.id());

    // Spawn the application process
    let mut app_handle = spawn_process(&current_exe, "app", &["app"]);
    log::info!(
        "[Manager] Application process started with PID: {}",
        app_handle.id()
    );

    // Graceful shutdown handling
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        log::info!("\n[Manager] Shutdown signal received. Terminating child processes...");
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        if let Ok(Some(status)) = app_handle.try_wait() {
            log::info!(
                "[Manager] Application process exited with status: {status}. Shutting down."
            );
            break;
        }
        if let Some(ref mut proxy) = proxy_handle {
            if let Ok(Some(status)) = proxy.try_wait() {
                log::info!("[Manager] Proxy process exited with status: {status}. Shutting down.");
                break;
            }
        }
        if let Ok(Some(status)) = tasks_handle.try_wait() {
            log::info!("[Manager] Tasks process exited with status: {status}. Shutting down.");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    log::info!("[Manager] Cleaning up child processes...");
    if let Some(ref mut proxy) = proxy_handle {
        if let Err(e) = proxy.kill() {
            log::error!("[Manager] Error killing proxy process: {e}");
        }
        let _ = proxy.wait();
    }
    if let Err(e) = tasks_handle.kill() {
        log::error!("[Manager] Error killing tasks process: {e}");
    }
    let _ = tasks_handle.wait();
    if let Err(e) = app_handle.kill() {
        log::error!("[Manager] Error killing app process: {e}");
    }
    let _ = app_handle.wait();
    log::info!("[Manager] Shutdown complete.");
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    env_logger::init();
    dotenvy::dotenv().ok();

    match args.command {
        Commands::Init => handle_init().expect("init failed"),
        Commands::Template => handle_template().expect("template loading failed"),
        Commands::Migrate => handle_migrations().await.expect("migrate failed"),
        Commands::Cache(args) => handle_cache_command(args)
            .await
            .expect("cache command failed"),
        Commands::SystemUser => handle_system_user()
            .await
            .expect("failed to create system user"),
        Commands::Instances(args) => handle_instance_command(args)
            .await
            .expect("instance command failed"),
        Commands::Send(args) => handle_send_command(args).expect("send command failed"),
        Commands::MutedTerms(args) => handle_muted_terms_command(args)
            .await
            .expect("muted terms command failed"),
        Commands::Server => handle_server_command(),
        Commands::App => enigmatick::server::start().await,
    }
}
