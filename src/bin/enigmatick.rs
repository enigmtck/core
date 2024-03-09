use clap::{Args, Parser, Subcommand};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use enigmatick::admin::create_user;
use enigmatick::{admin::NewUser, server};
use enigmatick::{POOL, SYSTEM_USER};
use rand::distributions::{Alphanumeric, DistString};
use serde::Deserialize;
use tokio::runtime::Runtime;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

#[derive(Deserialize, Subcommand)]
#[serde(rename_all = "lowercase")]
pub enum SetupCommands {
    SystemUser,
}

#[derive(Deserialize, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct SetupArgs {
    #[command(subcommand)]
    command: Option<SetupCommands>,
}

#[derive(Deserialize, Subcommand)]
#[serde(rename_all = "lowercase")]
pub enum Commands {
    Setup(SetupArgs),
    Server,
    Migrations,
    //Test,
}

#[derive(Parser)] // requires `derive` feature
#[command(name = "enigamtick")]
#[command(about = "Enigmatick Federated Communications Platform", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Setup(args) => handle_setup(args),
        Commands::Server => server::start(),
        Commands::Migrations => handle_migrations(),
        //Commands::Test => handle_test(),
    }
}

// fn handle_test() {

// }

fn handle_migrations() {
    println!("running database migrations.");
    let conn = &mut POOL.get().expect("failed to retrieve database connection");

    conn.run_pending_migrations(MIGRATIONS).ok();
}

fn handle_setup(args: SetupArgs) {
    let system_user = (*SYSTEM_USER).clone();

    if let Some(command) = args.command {
        match command {
            SetupCommands::SystemUser => {
                println!("setup system user: {system_user}");
                let rt = Runtime::new().unwrap();
                let handle = rt.handle();
                handle.block_on(async {
                    if create_user(
                        None,
                        NewUser {
                            username: system_user.clone(),
                            password: Alphanumeric.sample_string(&mut rand::thread_rng(), 16),
                            display_name: "System User".to_string(),
                            client_public_key: None,
                            client_private_key: None,
                            olm_pickled_account: None,
                            olm_pickled_account_hash: None,
                            olm_identity_key: None,
                            salt: None,
                        },
                    )
                    .await
                    .is_some()
                    {
                        println!("system user created.");
                    } else {
                        println!("failed to create system user.");
                    }
                })
            }
        }
    }
}
