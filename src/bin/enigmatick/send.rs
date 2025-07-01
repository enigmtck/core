
use anyhow::Result;
use clap::{Parser, Subcommand};
use enigmatick::models::activities::{ActivityType, NewActivity};
use enigmatick::models::actors as actor_model_ops;
use enigmatick::{
    helper::get_activity_ap_id_from_uuid,
    runner::{get_inboxes, send_to_inboxes},
};
use jdt_activity_pub::{
    ApActivity, ApActor, ApAddress, ApContext, ApObject, ApUpdate, MaybeReference,
};
use serde_json::Value;
use tokio::runtime::Runtime;
use uuid::Uuid;

#[derive(Parser)]
pub struct SendArgs {
    #[command(subcommand)]
    pub command: SendCommands,
}

#[derive(Subcommand)]
pub enum SendCommands {
    /// Send actor update to all known instances
    #[clap(name = "actor-update")]
    ActorUpdate { username: String },
}

pub fn handle_send_command(args: SendArgs) -> Result<()> {
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    match args.command {
        SendCommands::ActorUpdate { username } => {
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

async fn execute_send_actor_update(username: String) -> Result<()> {
    // Call the async function directly. It will use its internal spawn_blocking for the DB query.
    let actor_record = actor_model_ops::get_actor_by_username(None, username.clone()).await?;

    let ap_actor_to_update: ApActor = ApActor::from(actor_record.clone());
    let actor_ap_id = ap_actor_to_update
        .id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("CLI: Actor {username} AP ID is missing"))?;

    let activity_uuid = Uuid::new_v4().to_string();
    let activity_ap_id = get_activity_ap_id_from_uuid(activity_uuid.clone());

    let update_ap_object = ApUpdate {
        context: Some(ApContext::default()),
        id: Some(activity_ap_id.clone()),
        actor: actor_ap_id.clone(),
        object: MaybeReference::Actual(ApObject::Actor(ap_actor_to_update.clone())),
        to: vec![ApAddress::get_public()].into(),
        ..Default::default()
    };
    let update_activity = ApActivity::Update(update_ap_object.clone());

    let body_string = serde_json::to_string(&update_activity)
        .map_err(|e| anyhow::anyhow!("CLI: Failed to serialize update activity: {e}"))?;

    println!("CLI: Saving Update activity for '{username}' locally...");
    let activity_data_value: Value = serde_json::from_str(&body_string)
        .map_err(|e| anyhow::anyhow!("CLI: Failed to parse body_string to JSON Value: {e}"))?;

    let new_activity_to_save = NewActivity {
        kind: ActivityType::Update,
        uuid: activity_uuid,
        actor: actor_ap_id.to_string(),
        ap_to: update_ap_object.to.into(),
        cc: None,
        target_activity_id: None,
        target_ap_id: Some(actor_ap_id.clone().to_string()),
        revoked: false,
        ap_id: Some(activity_ap_id.clone()),
        reply: false,
        raw: Some(activity_data_value),
        target_object_id: None,
        actor_id: Some(actor_record.id),
        target_actor_id: Some(actor_record.id),
        log: Some(serde_json::json!([])),
        instrument: None,
    };

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
        Ok(_) => println!("CLI: Successfully saved Update activity for '{username}' locally."),
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
