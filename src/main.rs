use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use event_manager::{BotEvents, GuildCache};

use serenity::{all::Settings, client::ClientBuilder, prelude::*};

use tracing::{error, info};

mod commands;
mod event_manager;
mod model;
mod runner;


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
        guild_cache: Arc::new(Mutex::new(GuildCache::default())),
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
    if let Err(why) = client.start().await {
        error!("Error starting Discord client: {why}");
    }
}
