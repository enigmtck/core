use anyhow::Result;
use clap::{Parser, Subcommand};

use enigmatick::server;

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
}

#[derive(Parser)] // requires `derive` feature
#[command(name = "enigmatick")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A federated communication platform server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Init => handle_init().expect("init failed"),
        Commands::Template => handle_template().expect("template loading failed"),
        Commands::Migrate => handle_migrations().expect("migrate failed"),
        Commands::Cache(args) => handle_cache_command(args).expect("cache command failed"),
        Commands::SystemUser => handle_system_user().expect("failed to create system user"),
        Commands::Instances(args) => {
            handle_instance_command(args).expect("instance command failed")
        }
        Commands::Send(args) => handle_send_command(args).expect("send command failed"),
        Commands::MutedTerms(args) => {
            handle_muted_terms_command(args).expect("muted terms command failed")
        }
        Commands::Server => server::start(),
    }
}

