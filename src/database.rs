#![allow(unused)]
use crate::{
    error::Result,
    model::{
        guild::{self, Guild},
        thread::Thread,
    },
};
use chrono::Utc;
use f1_bot_types::{Event, EventStatus, Series};
use libsql::{de, params, Connection};
use serde::Serialize;
use serenity::all::{CacheHttp, ChannelId, CreateThread};

pub async fn fetch_events_by_status(
    db_conn: &Connection,
    status: EventStatus,
) -> Result<Vec<Event>> {
    let mut res = db_conn
        .query(
            r#"SELECT * FROM events 
    WHERE status = ? 
    AND year = strftime('%Y', current_timestamp)"#,
            [status],
        )
        .await?;
    let mut return_value = Vec::new();
    while let Ok(Some(data)) = res.next().await {
        return_value.push(de::from_row::<Event>(&data)?);
    }
    Ok(return_value)
}

pub async fn get_event_by_id(
    db_conn: &Connection,
    id: u64,
) -> Result<Option<Event>> {
    let mut res =
        db_conn.query("SELECT * FROM events WHERE id = ?", [id]).await?;
    res.next()
        .await?
        .map(|f| libsql::de::from_row::<Event>(&f))
        .transpose()
        .map_err(|e| e.into())
}

pub async fn update_event_status(
    db_conn: &Connection,
    event: &Event,
    new_status: EventStatus,
) -> Result {
    db_conn
        .execute(
            r#"UPDATE events SET status = ? WHERE id = ?"#,
            params![new_status, event.id],
        )
        .await?;
    Ok(())
}

pub async fn fetch_guilds(db_conn: &Connection) -> Result<Vec<Guild>> {
    let mut cursor = db_conn.query("SELECT * FROM guilds", ()).await?;
    let mut return_value = vec![];
    while let Ok(Some(row)) = cursor.next().await {
        return_value.push(libsql::de::from_row::<Guild>(&row)?);
    }

    Ok(return_value)
}

pub async fn fetch_thread_for_guild_and_event(
    db_conn: &Connection,
    guild_id: i64,
    event_id: i64,
) -> Result<Option<Thread>> {
    let mut cursor = db_conn
        .query(
            "SELECT * FROM threads WHERE guild_id = ? AND event_id = ?",
            params![guild_id, event_id],
        )
        .await?;

    if let Some(row) = cursor.next().await? {
        Ok(Some(libsql::de::from_row(&row)?))
    } else {
        Ok(None)
    }
}

pub async fn create_new_thread(
    db_conn: &Connection,
    http: impl CacheHttp,
    guild: &Guild,
    event: &Event,
) -> Result<Thread> {
    let (_, Some(channel), true) = guild.settings_for_series(event.series)
    else {
        return Err(crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Invalid Guild Settings",
        )));
    };
    let channel_id = ChannelId::new(channel.parse()?);
    let new_thread = channel_id
        .create_thread(
            http,
            CreateThread::new(format!(
                "{} {} {}",
                event.series, event.year, event.title
            )),
        )
        .await?;

    let thread_id = insert_new_thread(db_conn, &new_thread.id.to_string(), guild.id, event.id as i64, channel).await?;

    Ok(Thread {
        id: thread_id,
        guild_id: guild.id,
        event_id: event.id as i64,
        channel_id: channel.to_string(),
        discord_id: new_thread.id.to_string(),
        created_at: Utc::now(),
    })
}

pub async fn insert_new_thread(db_conn: &Connection, discord_id: &str, guild_id: i64, event_id: i64, channel_id: &str) -> Result<i64> {
    db_conn.execute("INSERT INTO threads (
        discord_id, channel_id, event_id, guild_id
    ) VALUES(?, ?, ?, ?)", 
    params![discord_id, channel_id, event_id, guild_id]).await?;
    Ok(db_conn.last_insert_rowid())
    
}
