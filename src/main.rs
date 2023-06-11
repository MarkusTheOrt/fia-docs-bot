use std::sync::Arc;

use tokio::sync::Mutex;
use event_manager::BotEvents;
use serenity::prelude::GatewayIntents;
use serenity::Client;
use sqlx::{MySql, MySqlPool, Pool};

mod event_manager;
mod model;
mod state;
mod commands;

pub async fn create_sqlx_client(connection: &str) -> Result<Pool<MySql>, sqlx::Error> {
    return MySqlPool::connect(connection).await;
}

#[tokio::main]
async fn main() {
    // I don't really care if the .env file can be loaded or not, its important that we have the
    // env variable
    let _ = dotenvy::dotenv();

    let (discord_token, sqlx_connection) = match (
        std::env::var("DISCORD_TOKEN"),
        std::env::var("DATABASE_URL"),
    ) {
        (Ok(token), Ok(connection)) => (token, connection),
        (Err(why), _) | (_, Err(why)) => {
            println!("Error reading from environment: {why}");
            return;
        }
    };

    let pool = match create_sqlx_client(&sqlx_connection).await {
        Ok(pool) => pool,
        Err(why) => {
            println!("Error connecting to database: {why}");
            return;
        }
    };

    let event_manager = BotEvents {
        pool: pool.clone(),
        f1_crawler_enabled: Arc::new(Mutex::new(true)),
        f2_crawler_enabled: Arc::new(Mutex::new(true)),
        f3_crawler_enabled: Arc::new(Mutex::new(true)),
        wrc_crawler_enabled: Arc::new(Mutex::new(true)),
        wrx_crawler_enabled: Arc::new(Mutex::new(true))
    };

    let mut client = Client::builder(discord_token, GatewayIntents::GUILDS)
        .event_handler(event_manager)
        .await
        .expect("Client to be created");

    if let Err(why) = client.start().await {
        println!("Error whilst running client: {why}");
    }
}
