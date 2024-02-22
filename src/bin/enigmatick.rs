use clap::{Args, Parser, Subcommand};
use enigmatick::{admin::NewUser, runner, server};
use rand::distributions::{Alphanumeric, DistString};
use serde::Deserialize;
use tokio::runtime::Runtime;

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
    Runner,
    Server,
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
        Commands::Runner => runner::start(),
        Commands::Server => {
            let rt = Runtime::new().unwrap();
            let handle = rt.handle();
            handle.block_on(async { server::start().await });
        }
    }
}

fn handle_setup(args: SetupArgs) {
    let server_name = &*enigmatick::SERVER_NAME;

    if let Some(command) = args.command {
        match command {
            SetupCommands::SystemUser => {
                println!("setup system user: {server_name}");
                let rt = Runtime::new().unwrap();
                let handle = rt.handle();
                handle.block_on(async {
                    runner::user::create(NewUser {
                        username: server_name.clone(),
                        password: Alphanumeric.sample_string(&mut rand::thread_rng(), 16),
                        display_name: "System User".to_string(),
                        client_public_key: None,
                        client_private_key: None,
                        olm_pickled_account: None,
                        olm_pickled_account_hash: None,
                        olm_identity_key: None,
                        salt: None,
                    })
                    .await;
                })
            }
        }
    }
}
