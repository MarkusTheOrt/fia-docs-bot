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

use tracing::{error, level_filters::LevelFilter, Level};
use tracing_subscriber::{
    layer::SubscriberExt, util::SubscriberInitExt, Layer,
};

mod commands;
mod database;
mod error;
mod event_manager;
mod model;
mod runner;

pub struct ShardManagerBox;

impl TypeMapKey for ShardManagerBox {
    type Value = Arc<ShardManager>;
}

#[tokio::main]
async fn main() {
    _ = dotenvy::dotenv();
    let guard = sentry::init((
        std::env::var("SENTRY_DSN").expect("Sentry DSR not found!"),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            traces_sample_rate: 1.0,
            ..Default::default()
        },
    ));
    tracing_subscriber::registry()
        .with(sentry::integrations::tracing::layer())
        .with(
            tracing_subscriber::fmt::layer()
                .with_filter(LevelFilter::from_level(Level::INFO)),
        )
        .init();
    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Discord token empty");
    let db_token = std::env::var("DATABASE_TOKEN").expect("db_token empty");
    let db_url = std::env::var("DATABASE_URL").expect("Database url empty");
    let db_client =
        libsql::Builder::new_remote_replica("./local.db", db_url, db_token)
            .sync_interval(Duration::from_secs(60))
            .build()
            .await
            .unwrap();

    db_client.sync().await.unwrap();

    let conn = db_client.connect().unwrap();

    let event_manager = BotEvents {
        thread_lock: AtomicBool::new(false),
        conn: Box::leak(Box::new(conn)),
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
    drop(guard);
}
