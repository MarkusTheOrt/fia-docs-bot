use std::sync::{atomic::AtomicBool, Arc, Mutex};

use event_manager::{BotEvents, GuildCache};
use sqlx::{MySql, MySqlPool, Pool};

use serenity::{client::ClientBuilder, prelude::*};

use anyhow::anyhow;

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

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_secrets::Secrets] secrets: shuttle_secrets::SecretStore
) -> shuttle_serenity::ShuttleSerenity {
    let (discord_token, sqlx_connection) =
        match (secrets.get("DISCORD_TOKEN"), secrets.get("DATABASE_URL")) {
            (Some(token), Some(connection)) => (token, connection),
            _ => return Err(anyhow!("Secrets not found.").into()),
        };

    let pool =
        Box::leak(Box::new(match create_sqlx_client(&sqlx_connection).await {
            Ok(pool) => pool,
            Err(why) => {
                return Err(anyhow!(why).into());
            },
        }));

    let event_manager = BotEvents {
        db: pool,
        guild_cache: Arc::new(Mutex::new(GuildCache::default())),
        thread_lock: AtomicBool::new(false),
    };

    let client = match ClientBuilder::new(discord_token, GatewayIntents::GUILDS)
        .event_handler(event_manager)
        .await
    {
        Ok(client) => client,
        Err(why) => {
            return Err(anyhow!(why).into());
        },
    };

    Ok(client.into())
}
