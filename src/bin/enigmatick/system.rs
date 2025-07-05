use anyhow::Result;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use enigmatick::admin::create_user;
use enigmatick::models::actors::ActorType;
use enigmatick::{admin::NewUser, POOL, SYSTEM_USER};
use rand::distributions::{Alphanumeric, DistString};
use rust_embed::RustEmbed;
use std::fs;
use tokio::runtime::Runtime;

cfg_if::cfg_if! {
    if #[cfg(feature = "pg")] {
        pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations.pg");
    } else if #[cfg(feature = "sqlite")] {
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

pub fn handle_migrations() -> Result<()> {
    println!("running database migrations...");
    let conn = &mut POOL.get().expect("failed to retrieve database connection");

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(anyhow::Error::msg)?;
    println!("complete.");

    Ok(())
}

pub fn handle_system_user() -> Result<()> {
    let system_user = (*SYSTEM_USER).clone();

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
