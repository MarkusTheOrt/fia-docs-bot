use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serenity::model::prelude::{Guild, PartialGuild};
use sqlx::{Pool, MySql};

#[derive(Serialize, Deserialize)]
pub struct DbGuild {
    pub id: u64,
    pub name: String,
    pub channel: Option<u64>,
    pub notify_role: Option<u64>,
    pub joined: DateTime<Utc>,
}

pub async fn insert_new_guild(
    guild: &Guild,
    pool: &Pool<MySql>,
) -> Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> {
    let new_guild = DbGuild {
        id: guild.id.as_u64().clone(),
        name: guild.name.clone(),
        channel: None,
        notify_role: None,
        joined: Utc::now(),
    };

    return sqlx::query!(
        "INSERT INTO guilds(id, name, channel, notify_role, joined) VALUES (?, ?, ?, ?, ?)",
        new_guild.id,
        new_guild.name,
        new_guild.channel,
        new_guild.notify_role,
        new_guild.joined
    )
    .execute(pool)
    .await;
}

pub async fn update_guild_name(
    guild: &PartialGuild,
    pool: &Pool<MySql>,
) -> Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> {
    return sqlx::query!(
        "UPDATE guilds SET name = ? WHERE id = ?",
        guild.name,
        guild.id.as_u64()
    )
    .execute(pool)
    .await;
}

