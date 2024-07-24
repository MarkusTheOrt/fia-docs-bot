use std::{sync::{atomic::AtomicBool, Arc, Mutex}, time::Duration};

use event_manager::{BotEvents, GuildCache};
use sqlx::{MySql, MySqlPool, Pool};

use serenity::{all::Settings, client::ClientBuilder, prelude::*};

use tracing::{error, info};

mod commands;
mod event_manager;
mod model;
mod runner;

pub async fn create_sqlx_client(
    connection: &str
) -> Result<Pool<MySql>, sqlx::Error> {
    MySqlPool::connect(connection).await
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    info!("Logging Enabled!");
    _ = dotenvy::dotenv();
    let discord_token =
        std::env::var("DISCORD_TOKEN").expect("Discord token empty");
    let db_url = std::env::var("DATABASE_URL").expect("Database url empty");

    let pool = Box::leak(Box::new(match create_sqlx_client(&db_url).await {
        Ok(pool) => pool,
        Err(why) => {
            error!("Error creating SQL client: {why}");
            return;
        },
    }));

    let event_manager = BotEvents {
        db: pool,
        guild_cache: Arc::new(Mutex::new(GuildCache::default())),
        thread_lock: AtomicBool::new(false),
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
    if let Err(why) = client.start().await {
        error!("Error starting Discord client: {why}");
    }
}
