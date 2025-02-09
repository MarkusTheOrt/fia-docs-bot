use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

use event_manager::BotEvents;

use serenity::{
    all::{Settings, ShardManager},
    client::ClientBuilder,
    prelude::*,
};

use tracing::{error, info};

mod commands;
mod event_manager;
mod model;
mod runner;
mod error;
mod database;

pub struct ShardManagerBox;

impl TypeMapKey for ShardManagerBox {
    type Value = Arc<ShardManager>;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    info!("Logging Enabled!");
    _ = dotenvy::dotenv();
    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Discord token empty");
    let db_url = std::env::var("DATABASE_URL").expect("Database url empty");
    let db_client = libsql::Builder::new_remote_replica(
        "./local.db",
        db_url,
        "".to_string(),
    )
    .sync_interval(Duration::from_secs(60))
    .build()
    .await
    .unwrap();

    db_client.sync().await.unwrap();

    let conn = db_client.connect().unwrap();

    let event_manager = BotEvents {
        thread_lock: AtomicBool::new(false),
        conn: Box::leak(Box::new(conn)),
        shards: None,
    };
    let mut settings = Settings::default();
    settings.cache_users = false;
    settings.cache_channels = true;
    settings.cache_guilds = true;
    let mut client =
        match ClientBuilder::new(discord_token, GatewayIntents::GUILDS)
            .event_handler(event_manager)
            .cache_settings(settings)
            .await
        {
            Ok(client) => client,
            Err(why) => {
                error!("Error creating Discord client: {why}");
                return;
            },
        };

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerBox>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.expect("Error registering ctrlc handler");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start_autosharded().await {
        error!("Error starting Discord client: {why}");
    }

    // Final sync once the bot stops
    if let Err(why) = db_client.sync().await {
        error!("Error syncing Database: {why:#?}");
    }
}
