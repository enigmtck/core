use anyhow::{anyhow, Result};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use enigmatick::admin::create_user;
use enigmatick::models::actors::ActorType;
use enigmatick::{admin::NewUser, SYSTEM_USER};
use rand::distributions::{Alphanumeric, DistString};
use rust_embed::RustEmbed;
use std::fs;
use tokio::runtime::Runtime;

cfg_if::cfg_if! {
    if #[cfg(feature = "sqlite")] {
        pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations.sqlite");
    }
}

#[derive(RustEmbed)]
#[folder = "bundled/"]
pub struct Bundled;

pub fn handle_init() -> Result<()> {
    println!("creating folder structure...");
    fs::create_dir_all("media/avatars")?;
    fs::create_dir_all("media/banners")?;
    fs::create_dir_all("media/cache")?;
    fs::create_dir_all("media/uploads")?;
    fs::create_dir_all("acme")?;
    println!("complete.");

    Ok(())
}

pub fn handle_template() -> Result<()> {
    if let Some(template) = Bundled::get(".env.template") {
        fs::write(".env.template", template.data)?;
    }
    Ok(())
}

pub async fn handle_migrations() -> Result<()> {
    println!("running database migrations...");
    let conn = enigmatick::db::POOL.get().await?;

    conn.interact(|c| {
        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations.pg");
        // Run the migrations and handle the result inside the closure
        // to avoid lifetime issues with the return type.
        match c.run_pending_migrations(MIGRATIONS) {
            Ok(versions) => {
                for v in versions {
                    println!("  Applying migration {v}");
                }
                Ok(())
            }
            Err(e) => {
                // Propagate the error from the migration harness.
                Err(e)
            }
        }
    })
    .await
    // The `interact` method returns a nested Result, so we flatten it.
    // The outer error is from deadpool (e.g., pool is closed).
    .map_err(|e| anyhow!("Database pool interaction error: {}", e))?
    // The inner error is from the migration harness itself.
    .map_err(|e| anyhow!("Failed to run database migrations: {}", e))?;

    println!("complete.");

    Ok(())
}

pub fn handle_system_user() -> Result<()> {
    let system_user = (*SYSTEM_USER).clone();

    println!("setup system user: {system_user}");
    let rt = Runtime::new().unwrap();
    let handle = rt.handle();
    handle.block_on(async {
        let conn = match enigmatick::db::POOL.get().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to get DB connection: {e}");
                return;
            }
        };

        if create_user(
            &conn,
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
                kind: Some(ActorType::Application),
            },
        )
        .await
        .is_ok()
        {
            println!("system user created.");
        } else {
            println!("failed to create system user.");
        }
    });

    Ok(())
}
