use clap::Parser;
use std::env;

mod cache;
mod display;
mod instances;
mod muted_terms;
mod search;
mod send;
mod system;

use cache::{handle_cache_command, CacheArgs};
use instances::{handle_instance_command, InstanceArgs};
use muted_terms::{handle_muted_terms_command, MutedTermsArgs};
use search::{handle_search_command, SearchArgs};
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
    /// Manage federated instances
    Instances(InstanceArgs),
    /// Manage search index
    Search(SearchArgs),
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
        Commands::Search(args) => handle_search_command(args)
            .await
            .expect("search command failed"),
        Commands::Send(args) => handle_send_command(args)
            .await
            .expect("send command failed"),
        Commands::MutedTerms(args) => handle_muted_terms_command(args)
            .await
            .expect("muted terms command failed"),
        Commands::App => enigmatick::server::start().await,
    }
}
