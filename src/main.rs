use std::sync::{atomic::AtomicBool, Arc, Mutex};

use event_manager::{BotEvents, GuildCache};
use sqlx::{MySql, MySqlPool, Pool};

use serenity::{client::ClientBuilder, prelude::*};

mod commands;
mod event_manager;
mod model;
mod runner;
mod state;

pub async fn create_sqlx_client(
    connection: &str
) -> Result<Pool<MySql>, sqlx::Error> {
    MySqlPool::connect(connection).await
}

#[tokio::main]
async fn main() {
    // I don't really care if the .env file can be loaded or not, its important that we have the
    // env variable
    let _ = dotenvy::dotenv();

    let (discord_token, sqlx_connection) =
        match (std::env::var("DISCORD_TOKEN"), std::env::var("DATABASE_URL")) {
            (Ok(token), Ok(connection)) => (token, connection),
            (Err(why), _) | (_, Err(why)) => {
                println!("Error reading from environment: {why}");
                return;
            },
        };

    let pool = match create_sqlx_client(&sqlx_connection).await {
        Ok(pool) => pool,
        Err(why) => {
            println!("Error connecting to database: {why}");
            return;
        },
    };

    let event_manager = BotEvents {
        pool: pool.clone(),
        guild_cache: Arc::new(Mutex::new(GuildCache::default())),
        thread_lock: AtomicBool::new(false),
    };

    let mut client =
        match ClientBuilder::new(discord_token, GatewayIntents::GUILDS)
            .event_handler(event_manager)
            .await
        {
            Ok(client) => client,
            Err(why) => {
                println!("Error creting client: {why}");
                return;
            },
        };
    if let Err(why) = client.start().await {
        println!("Error running client: {why}");
    }
}
