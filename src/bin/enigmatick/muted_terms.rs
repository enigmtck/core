use anyhow::Result;
use clap::{Parser, Subcommand};
use enigmatick::models::actors::{get_muted_terms_by_username, update_muted_terms_by_username};
use tokio::runtime::Runtime;

#[derive(Parser)]
pub struct MutedTermsArgs {
    #[command(subcommand)]
    pub command: MutedTermsCommands,
}

#[derive(Subcommand)]
pub enum MutedTermsCommands {
    /// List muted terms for user
    List { username: String },
    /// Add muted term for user
    Add { username: String, term: String },
    /// Remove muted term for user
    Remove { username: String, term: String },
    /// Clear all muted terms for user
    Clear { username: String },
}

pub fn handle_muted_terms_command(args: MutedTermsArgs) -> Result<()> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match args.command {
        MutedTermsCommands::List { username } => {
            println!("Listing muted terms for user: {username}...");
            handle.block_on(async {
                match get_muted_terms_by_username(None, username.clone()).await {
                    Ok(terms) => {
                        if terms.is_empty() {
                            println!("No muted terms found for user '{username}'.");
                        } else {
                            println!("Muted terms for user '{username}':");
                            for (index, term) in terms.iter().enumerate() {
                                println!("  {}. {}", index + 1, term);
                            }
                            println!("Total: {} term(s)", terms.len());
                        }
                    }
                    Err(e) => eprintln!("Error retrieving muted terms for user '{username}': {e}"),
                }
            });
        }
        MutedTermsCommands::Add { username, term } => {
            println!("Adding muted term '{term}' for user: {username}...");
            handle.block_on(async {
                match get_muted_terms_by_username(None, username.clone()).await {
                    Ok(mut current_terms) => {
                        if current_terms.contains(&term) {
                            println!("Term '{term}' is already muted for user '{username}'.");
                        } else {
                            current_terms.push(term.clone());
                            match update_muted_terms_by_username(
                                None,
                                username.clone(),
                                current_terms,
                            )
                            .await
                            {
                                Ok(_) => println!(
                                    "Successfully added muted term '{term}' for user '{username}'."
                                ),
                                Err(e) => eprintln!(
                                    "Error adding muted term '{term}' for user '{username}': {e}"
                                ),
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error retrieving current muted terms for user '{username}': {e}")
                    }
                }
            });
        }
        MutedTermsCommands::Remove { username, term } => {
            println!("Removing muted term '{term}' for user: {username}...");
            handle.block_on(async {
                match get_muted_terms_by_username(None, username.clone()).await {
                    Ok(mut current_terms) => {
                        if let Some(pos) = current_terms.iter().position(|x| x == &term) {
                            current_terms.remove(pos);
                            match update_muted_terms_by_username(None, username.clone(), current_terms).await {
                                Ok(_) => println!("Successfully removed muted term '{term}' for user '{username}'."),
                                Err(e) => eprintln!("Error removing muted term '{term}' for user '{username}': {e}"),
                            }
                        } else {
                            println!("Term '{term}' is not in the muted terms list for user '{username}'.");
                        }
                    }
                    Err(e) => eprintln!("Error retrieving current muted terms for user '{username}': {e}"),
                }
            });
        }
        MutedTermsCommands::Clear { username } => {
            println!("Clearing all muted terms for user: {username}...");
            handle.block_on(async {
                match update_muted_terms_by_username(None, username.clone(), vec![]).await {
                    Ok(_) => {
                        println!("Successfully cleared all muted terms for user '{username}'.")
                    }
                    Err(e) => eprintln!("Error clearing muted terms for user '{username}': {e}"),
                }
            });
        }
    }
    Ok(())
}
