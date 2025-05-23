#![allow(unused)]
use crate::{
    error::Result,
    model::{
        guild::{self, Guild},
        thread::Thread,
    },
};
use chrono::Utc;
use f1_bot_types::{
    Document, DocumentStatus, Event, EventStatus, Image, Series,
};
use libsql::{
    Connection,
    de::{self, from_row},
    params,
};
use serde::Serialize;
use serenity::all::{
    AutoArchiveDuration, CacheHttp, ChannelId, ChannelType, CreateEmbed,
    CreateEmbedAuthor, CreateMessage, CreateThread,
};
use tracing::{Instrument, info};

pub async fn fetch_latest_event_by_series(
    db_conn: &Connection,
    series: Series,
) -> Result<Option<Event>> {
    let mut res = db_conn
        .query(
            r#"SELECT * FROM EVENTS 
    WHERE series = ? 
    ORDER BY created_at DESC 
    LIMIT 1"#,
            params![series],
        )
        .await?;
    Ok(match res.next().await? {
        None => None,
        Some(d) => Some(de::from_row::<Event>(&d)?),
    })
}

pub async fn clear_guild_settings(
    db_conn: &Connection,
    guild_id: i64,
    series: Series,
) -> Result {
    let _ = match series {
        Series::F1 => {
            db_conn
                .execute(
                    r#"UPDATE guilds
        SET f1_channel = NULL
        WHERE id = ?"#,
                    params![guild_id],
                )
                .await?
        },
        Series::F2 => {
            db_conn
                .execute(
                    r#"UPDATE guilds
        SET f2_channel = NULL
        WHERE id = ?"#,
                    params![guild_id],
                )
                .await?
        },
        Series::F3 => {
            db_conn
                .execute(
                    r#"UPDATE guilds
        SET f3_channel = NULL
        WHERE id = ?"#,
                    params![guild_id],
                )
                .await?
        },
        Series::F1Academy => unimplemented!(),
    };
    Ok(())
}

#[tracing::instrument(skip(db_conn))]
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

#[tracing::instrument(skip(db_conn))]
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

#[tracing::instrument(skip(db_conn))]
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

#[tracing::instrument(skip(db_conn))]
pub async fn fetch_guilds(db_conn: &Connection) -> Result<Vec<Guild>> {
    let mut cursor = db_conn.query("SELECT * FROM guilds", ()).await?;
    let mut return_value = vec![];
    while let Ok(Some(row)) = cursor.next().await {
        return_value.push(libsql::de::from_row::<Guild>(&row)?);
    }

    Ok(return_value)
}

pub async fn fetch_thread_for_discord_guild_and_event(
    db_conn: &Connection,
    guild_id: impl Into<String>,
    event_id: i64,
) -> Result<Option<Thread>> {
    Ok(None)
}

pub async fn fetch_guild_by_discord_id(
    db_conn: &Connection,
    guild_id: impl ToString,
) -> Result<Option<Guild>> {
    let mut cursor = db_conn
        .query(
            "SELECT * FROM guilds WHERE discord_id = ?",
            params![guild_id.to_string()],
        )
        .await?;

    Ok(cursor.next().await?.map(|f| de::from_row::<Guild>(&f)).transpose()?)
}

#[tracing::instrument(skip(db_conn))]
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

#[tracing::instrument(skip(db_conn, http))]
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
            ))
            .auto_archive_duration(AutoArchiveDuration::ThreeDays)
            .kind(ChannelType::PublicThread)
            .audit_log_reason("New Approved FIA Event"),
        )
        .await?;

    let thread_id = insert_new_thread(
        db_conn,
        &new_thread.id.to_string(),
        guild.id,
        event.id as i64,
        channel,
    )
    .await?;

    Ok(Thread {
        id: thread_id,
        guild_id: guild.id,
        event_id: event.id as i64,
        channel_id: channel.to_string(),
        discord_id: new_thread.id.to_string(),
        created_at: Utc::now(),
    })
}

#[tracing::instrument(skip(db_conn))]
pub async fn insert_new_thread(
    db_conn: &Connection,
    discord_id: &str,
    guild_id: i64,
    event_id: i64,
    channel_id: &str,
) -> Result<i64> {
    db_conn
        .execute(
            "INSERT INTO threads (
        discord_id, channel_id, event_id, guild_id
    ) VALUES(?, ?, ?, ?)",
            params![discord_id, channel_id, event_id, guild_id],
        )
        .await?;
    Ok(db_conn.last_insert_rowid())
}

#[tracing::instrument(skip(db_conn))]
pub async fn fetch_docs_for_event(
    db_conn: &Connection,
    event_id: i64,
) -> Result<Vec<Document>> {
    let mut cursor = db_conn
        .query(
            "SELECT * FROM documents WHERE event_id = ? AND status = ? ORDER BY created_at DESC",
            params![event_id, DocumentStatus::ReadyToPost],
        )
        .await?;

    let mut return_value = vec![];
    while let Ok(Some(doc)) = cursor.next().await {
        return_value.push(from_row(&doc)?);
    }

    Ok(return_value)
}

#[tracing::instrument(skip(db_conn))]
pub async fn fetch_images_for_document(
    db_conn: &Connection,
    document_id: i64,
) -> Result<Vec<Image>> {
    let mut cursor = db_conn.query("SELECT * FROM images WHERE document_id = ? ORDER BY page_number LIMIT 4", [document_id]).await?;
    let mut return_value = vec![];
    while let Ok(Some(row)) = cursor.next().await {
        return_value.push(from_row(&row)?);
    }

    Ok(return_value)
}

pub fn create_message(
    document: &f1_bot_types::Document,
    images: Vec<Image>,
) -> CreateMessage {
    let mut return_value = vec![];
    let main_embed = CreateEmbed::new()
        .title(&document.title)
        .url(&document.href)
        .description(format!("[mirror]({})", document.mirror))
        .color(0x003063)
        .thumbnail("https://static.ort.dev/fiadontsueme/fia_logo.png")
        .timestamp(document.created_at)
        .author(CreateEmbedAuthor::new("FIA Document"));

    let mut iter = images.into_iter();
    if let Some(image) = iter.next() {
        return_value.push(main_embed.image(image.url));
    } else {
        return_value.push(main_embed);
    };

    for image in iter {
        return_value
            .push(CreateEmbed::new().url(&document.href).image(image.url));
    }

    CreateMessage::new().embeds(return_value)
}

#[tracing::instrument(skip(db_conn))]
pub async fn mark_event_done(
    db_conn: &Connection,
    event_id: i64,
) -> Result {
    db_conn
        .execute(
            "UPDATE events SET status = ? WHERE id = ?",
            params![EventStatus::Posted, event_id],
        )
        .await?;
    Ok(())
}

#[tracing::instrument(skip(db_conn))]
pub async fn mark_doc_done(
    db_conn: &Connection,
    document_id: i64,
) -> Result {
    db_conn
        .execute(
            "UPDATE documents SET STATUS = ? WHERE id = ?",
            params![DocumentStatus::Posted, document_id],
        )
        .await?;
    Ok(())
}
