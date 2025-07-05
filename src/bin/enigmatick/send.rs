use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use enigmatick::models::activities::NewActivity;
use enigmatick::models::actors as actor_model_ops;
use enigmatick::runner::{get_inboxes, send_to_inboxes};
use jdt_activity_pub::{ApActivity, ApActor, ApDelete, ApUpdate};
use tokio::runtime::Runtime;

#[derive(Parser)]
pub struct SendArgs {
    #[command(subcommand)]
    pub command: SendCommands,
}

#[derive(Subcommand)]
pub enum SendCommands {
    /// Send update activities
    Update(UpdateArgs),
    /// Send delete activities
    Delete(DeleteArgs),
}

#[derive(Parser)]
pub struct UpdateArgs {
    #[command(subcommand)]
    pub command: UpdateCommands,
}

#[derive(Subcommand)]
pub enum UpdateCommands {
    /// Send actor update to all known instances
    Actor { username: String },
}

#[derive(Parser)]
pub struct DeleteArgs {
    #[command(subcommand)]
    pub command: DeleteCommands,
}

#[derive(Subcommand)]
pub enum DeleteCommands {
    /// Send actor delete to all known instances
    Actor { username: String },
}

pub fn handle_send_command(args: SendArgs) -> Result<()> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match args.command {
        SendCommands::Update(update_args) => handle_update_command(update_args, &handle),
        SendCommands::Delete(delete_args) => handle_delete_command(delete_args, &handle),
    }
}

fn handle_update_command(args: UpdateArgs, handle: &tokio::runtime::Handle) -> Result<()> {
    match args.command {
        UpdateCommands::Actor { username } => {
            println!("Attempting to send actor update for user: {username}...");
            handle.block_on(async {
                match execute_send_actor_update(username).await {
                    Ok(_) => println!("Successfully processed sending of actor update."),
                    Err(e) => eprintln!("Error sending actor update: {e:?}"),
                }
            });
        }
    }
    Ok(())
}

async fn execute_send_actor_delete(username: String) -> Result<()> {
    // Get the actor record
    let actor_record = actor_model_ops::get_actor_by_username(None, username.clone()).await?;

    let ap_actor_to_delete: ApActor = ApActor::from(actor_record.clone());
    let mut delete_ap_object =
        ApDelete::try_from(ap_actor_to_delete).map_err(anyhow::Error::msg)?;
    let delete_activity = ApActivity::Delete(Box::new(delete_ap_object.clone()));

    let new_activity_to_save =
        NewActivity::try_from((delete_activity.clone(), Some(actor_record.clone().into())))?;

    let save_activity_result = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(enigmatick::models::activities::create_activity(
            None,
            new_activity_to_save,
        ))
    })
    .await?;

    match save_activity_result {
        Ok(activity) => {
            delete_ap_object.id = Some(
                activity
                    .ap_id
                    .ok_or(anyhow!("CLI: Saved Activity does not have an ID."))?,
            );
            println!("CLI: Successfully saved Delete activity for '{username}' locally.")
        }
        Err(e) => eprintln!("CLI: Failed to save Delete activity for '{username}' locally: {e:?}"),
    }

    let delivery_inboxes = get_inboxes(None, delete_activity.clone(), actor_record.clone()).await;

    if delivery_inboxes.is_empty() {
        println!("CLI: No active instance inboxes found to send the delete to.");
        return Ok(());
    }
    println!(
        "CLI: Preparing to send actor delete for '{}' to {} instance inboxes.",
        username,
        delivery_inboxes.len()
    );

    // Capture the updated ID in delete_ap_object as an ApActivity enum (shadows previous definition)
    let delete_activity = ApActivity::Delete(Box::new(delete_ap_object.clone()));
    match send_to_inboxes(
        None,
        delivery_inboxes,
        actor_record.clone(),
        delete_activity,
    )
    .await
    {
        Ok(_) => {
            println!(
                "CLI: Actor delete for '{username}' has been successfully queued for sending to instance inboxes."
            );
            if enigmatick::models::actors::tombstone_actor_by_as_id(None, actor_record.as_id)
                .await
                .is_ok()
            {
                println!(
                    "CLI: Actor delete for '{username}' has been successfully executed locally."
                );
            }
        }
        Err(e) => eprintln!("CLI: Error queueing actor delete for '{username}' for sending: {e:?}"),
    }

    Ok(())
}

fn handle_delete_command(args: DeleteArgs, handle: &tokio::runtime::Handle) -> Result<()> {
    match args.command {
        DeleteCommands::Actor { username } => {
            println!("Attempting to send actor delete for user: {username}...");
            handle.block_on(async {
                match execute_send_actor_delete(username).await {
                    Ok(_) => println!("Successfully processed sending of actor delete."),
                    Err(e) => eprintln!("Error sending actor delete: {e:?}"),
                }
            });
        }
    }
    Ok(())
}

async fn execute_send_actor_update(username: String) -> Result<()> {
    // Call the async function directly. It will use its internal spawn_blocking for the DB query.
    let actor_record = actor_model_ops::get_actor_by_username(None, username.clone()).await?;

    let ap_actor_to_update: ApActor = ApActor::from(actor_record.clone());
    let mut update_ap_object =
        ApUpdate::try_from(ap_actor_to_update).map_err(anyhow::Error::msg)?;
    let update_activity = ApActivity::Update(update_ap_object.clone());

    let new_activity_to_save =
        NewActivity::try_from((update_activity.clone(), Some(actor_record.clone().into())))?;

    let save_activity_result = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        rt.block_on(enigmatick::models::activities::create_activity(
            None,
            new_activity_to_save,
        ))
    })
    .await?;

    match save_activity_result {
        Ok(activity) => {
            update_ap_object.id = Some(
                activity
                    .ap_id
                    .ok_or(anyhow!("CLI: Saved Activity does not have an ID."))?,
            );
            println!("CLI: Successfully saved Update activity for '{username}' locally.")
        }
        Err(e) => eprintln!("CLI: Failed to save Update activity for '{username}' locally: {e:?}"),
    }

    let delivery_inboxes = get_inboxes(None, update_activity.clone(), actor_record.clone()).await;

    if delivery_inboxes.is_empty() {
        println!("CLI: No active instance inboxes found to send the update to.");
        return Ok(());
    }
    println!(
        "CLI: Preparing to send actor update for '{}' to {} instance inboxes.",
        username,
        delivery_inboxes.len()
    );

    // Capture the updated ID in update_ap_object as an ApActivity enum (shadows previous definition)
    let update_activity = ApActivity::Update(update_ap_object.clone());
    match send_to_inboxes(None, delivery_inboxes, actor_record, update_activity).await {
        Ok(_) => println!(
            "CLI: Actor update for '{username}' has been successfully queued for sending to instance inboxes."
        ),
        Err(e) => eprintln!(
            "CLI: Error queueing actor update for '{username}' for sending: {e:?}"
        ),
    }

    Ok(())
}
