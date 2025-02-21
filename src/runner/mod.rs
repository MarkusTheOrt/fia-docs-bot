use std::{error::Error, str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use f1_bot_types::{Event, EventStatus};
use libsql::{de, params, Connection};
use notifbot_macros::notifbot_enum;
use serenity::{
    all::{
        ChannelId, Context, CreateActionRow, CreateButton, CreateEmbed,
        CreateMessage, GuildId,
    },
    model::channel,
};
use tracing::{error, info};

use crate::{
    database::{
        create_new_thread, fetch_events_by_status, fetch_guilds,
        fetch_thread_for_guild_and_event,
    },
    model::guild,
};

const REQUEST_CHANNEL_ID: u64 = 1338180150906327120;

notifbot_enum!(AllowRequestStatus {
    Open,
    Allowed,
    Denied
});

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct AllowRequest {
    id: i64,
    event_id: u64,
    response: AllowRequestStatus,
    created_at: DateTime<Utc>,
    approved_by: Option<String>,
    approved_at: Option<DateTime<Utc>>,
}

pub async fn has_allow_request(
    db_conn: &Connection,
    event: &Event,
) -> Result<Option<AllowRequest>, crate::error::Error> {
    let mut rows = db_conn
        .query("SELECT * FROM allow_requests WHERE event_id = ?", [event.id])
        .await?;
    Ok(match rows.next().await?.map(|f| de::from_row::<AllowRequest>(&f)) {
        Some(r) => Some(r?),
        None => None,
    })
}

pub async fn create_allow_request(
    db_conn: &Connection,
    event: &Event,
    ctx: &Context,
) -> Result<AllowRequest, crate::error::Error> {
    db_conn
        .execute(
            "INSERT INTO allow_requests (event_id, response) VALUES (?, ?)",
            params![event.id, AllowRequestStatus::Open.to_str()],
        )
        .await?;
    let row_id = db_conn.last_insert_rowid();
    create_discord_allow_request(ctx, event, row_id).await?;
    Ok(AllowRequest {
        id: row_id,
        event_id: event.id,
        response: AllowRequestStatus::Open,
        created_at: Utc::now(),
        approved_by: None,
        approved_at: None,
    })
}

pub async fn create_discord_allow_request(
    ctx: &Context,
    event: &Event,
    request_id: i64,
) -> Result<(), crate::error::Error> {
    ChannelId::new(REQUEST_CHANNEL_ID)
        .send_message(
            ctx,
            CreateMessage::new()
                .embed(
                    CreateEmbed::new().title("New Event Found!").description(
                        format!(
                            "## {} {} {}\n\nPlease allow or deny.",
                            event.year, event.series, event.title,
                        ),
                    ),
                )
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("allow-{request_id}"))
                        .label("Allow")
                        .style(serenity::all::ButtonStyle::Success),
                    CreateButton::new(format!("deny-{request_id}"))
                        .label("Deny")
                        .style(serenity::all::ButtonStyle::Danger),
                ])]),
        )
        .await?;
    Ok(())
}

pub async fn runner(
    db_conn: &Connection,
    ctx: &Context,
) -> Result<(), crate::error::Error> {
    info!("Runner running");
    loop {
        let not_allowed_events =
            fetch_events_by_status(db_conn, EventStatus::NotAllowed).await?;

        for event in not_allowed_events.into_iter() {
            if has_allow_request(db_conn, &event).await?.is_none() {
                create_allow_request(db_conn, &event, ctx).await?;
            }
        }

        let allowed_events =
            fetch_events_by_status(db_conn, EventStatus::Allowed).await?;

        // TODO: Check for Completed documents in this event, post new documents.
        for event in allowed_events.into_iter() {
            info!("allowed event: {}", event.title);
            for guild in fetch_guilds(db_conn).await? {
                let (role, channel, use_threads) =
                    guild.settings_for_series(event.series);

                let Some(channel) = channel else {
                    continue;
                };

                let channel_to_post = if use_threads {
                    channel.to_owned()
                } else {
                    match fetch_thread_for_guild_and_event(
                        db_conn,
                        guild.id,
                        event.id as i64,
                    )
                    .await?
                    {
                        Some(c) => c.discord_id,
                        None => {
                            create_new_thread(db_conn, &ctx, &guild, &event)
                                .await?
                                .discord_id
                        },
                    }
                };

                let channel_id = ChannelId::new(channel_to_post.parse()?);
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

//#[tokio::main]
//pub async fn runner(
//    ctx: Context,
//    db_conn: &mut MySqlConnection,
//    guild_cache: Arc<Mutex<GuildCache>>,
//) {
//    let mut thread_cache = ThreadCache::default();
//    loop {
//        populate_cache(db_conn, &mut thread_cache).await;
//        populate_guild_cache(db_conn, &guild_cache).await;
//        std::thread::sleep(Duration::from_secs(5));
//        let guilds = {
//            let cache = guild_cache.lock().unwrap();
//            cache.cache.clone()
//        };
//
//        create_threads(db_conn, &guilds, &ctx, &mut thread_cache).await;
//        run_internal(
//            db_conn,
//            Series::F1,
//            &guilds,
//            &ctx,
//            &mut thread_cache,
//        )
//        .await;
//        run_internal(
//            db_conn,
//            Series::F2,
//            &guilds,
//            &ctx,
//            &mut thread_cache,
//        )
//        .await;
//        run_internal(
//            db_conn,
//            Series::F3,
//            &guilds,
//            &ctx,
//            &mut thread_cache,
//        )
//        .await;
//    }
//}

//async fn create_threads(
//    db_conn: &mut MySqlConnection,
//    guilds: &Vec<CachedGuild>,
//    ctx: &Context,
//    thread_cache: &mut ThreadCache,
//) {
//    let events = match new_events(db_conn).await {
//        Ok(data) => data,
//        Err(_) => return,
//    };
//    for event in events.into_iter() {
//        if let Err(why) =
//            sqlx::query!("UPDATE events SET new = 0 WHERE id = ?", event.id)
//                .execute(&mut *db_conn)
//                .await
//        {
//            error!("Error marking event as done: {why}");
//            continue;
//        }
//        for guild in guilds {
//            let guild_data = match Series::from(event.series) {
//                Series::F1 => &guild.f1,
//                Series::F2 => &guild.f2,
//                Series::F3 => &guild.f3,
//                Series::F1Academy => panic!("F1A Not Supported"),
//            };
//
//            let Some(guild_channel) = guild_data.channel else {
//                continue;
//            };
//
//            if !guild_data.use_threads {
//                continue;
//            }
//
//            let channel = ChannelId::new(guild_channel);
//            let thread = match channel
//                .create_thread(
//                    ctx,
//                    CreateThread::new(format!(
//                        "{} {} {}",
//                        event.year, event.series, event.name
//                    ))
//                    .audit_log_reason("New event thread")
//                    .kind(PublicThread)
//                    .auto_archive_duration(AutoArchiveDuration::ThreeDays),
//                )
//                .await
//            {
//                Ok(data) => data,
//                Err(why) => {
//                    error!(
//                        "Error creating thread for guild {}: {why}",
//                        guild.id
//                    );
//                    continue;
//                },
//            };
//            let thread = MinThread {
//                id: thread.id.get(),
//                guild: guild.id,
//                event: event.id,
//                year: event.year,
//            };
//            if let Err(why) = insert_thread(db_conn, &thread).await {
//                error!("Error adding thread to database: {why}");
//            }
//            thread_cache.cache.push(thread);
//        }
//    }
//}

//async fn new_events(
//    db_conn: &mut MySqlConnection
//) -> Result<Vec<NewEvent>, sqlx::Error> {
//    sqlx::query_as_unchecked!(
//        NewEvent,
//        r#"
//    SELECT `id` as `id!`, name, year, series FROM events WHERE new = 1
//    "#
//    )
//    .fetch_all(db_conn)
//    .await
//}

//async fn run_internal(
//    db_conn: &mut MySqlConnection,
//    series: Series,
//    guild_cache: &[CachedGuild],
//    ctx: &Context,
//    thread_cache: &mut ThreadCache,
//) {
//    let docs = match unposted_documents(db_conn, series).await {
//        Ok(data) => join_to_doc(data),
//        Err(why) => {
//            error!("Error reading unposted docs from db:\n{why}");
//            return;
//        },
//    };
//
//    for doc in docs.into_iter() {
//        if let Err(why) = mark_doc_done(db_conn, &doc).await {
//            error!("Error marking doc as done:\n{why}");
//            continue;
//        }
//        for guild in guild_cache.iter() {
//            let guild_data = match series {
//                Series::F1 => &guild.f1,
//                Series::F2 => &guild.f2,
//                Series::F3 => &guild.f3,
//                _ => panic!("F1Academy Not Supported!"),
//            };
//            // skip not-set up guilds.
//            if guild_data.channel.is_none() {
//                continue;
//            }
//            if guild_data.use_threads {
//                let msg = create_message(&doc, guild_data.role);
//                if let Some(thread) = thread_cache
//                    .cache
//                    .iter()
//                    .find(|p| p.guild == guild.id && p.event == doc.event)
//                {
//                    let id = ChannelId::new(thread.id);
//                    if let Err(why) = id.send_message(&ctx, msg).await {
//                        error!(
//                            "Error sending msg in thread: [{}] {why}",
//                            guild.id
//                        );
//                        continue;
//                    }
//                } else {
//                    error!("thread for guild {} not found!", guild.id);
//                }
//            } else {
//                let id = ChannelId::new(guild_data.channel.unwrap());
//                if let Err(why) = id
//                    .send_message(&ctx, create_message(&doc, guild_data.role))
//                    .await
//                {
//                    error!("Error sending channel embed: {why}");
//                }
//            }
//        }
//    }
//}

//async fn populate_guild_cache(
//    db_conn: &mut MySqlConnection,
//    guild_cache: &Arc<Mutex<GuildCache>>,
//) {
//    {
//        let cache = guild_cache.lock().unwrap();
//        if (Utc::now() - cache.last_populated).num_days() < 1 {
//            return;
//        }
//    }
//    let data = match sqlx::query_as_unchecked!(
//        AllGuild,
//        r#"
//    SELECT id, f1_channel, f1_role, f1_threads,
//    f2_channel, f2_role, f2_threads,
//    f3_channel, f3_role, f3_threads FROM guilds
//    "#
//    )
//    .fetch_all(db_conn)
//    .await
//    {
//        Ok(data) => data,
//        Err(why) => {
//            error!("Error fetching guilds: {why}");
//            return;
//        },
//    };
//    let mut cache_mut = guild_cache.lock().unwrap();
//    cache_mut.cache.clear();
//    for guild in data.into_iter() {
//        cache_mut.cache.push(guild.into());
//    }
//    info!("guilds cache populated!");
//    cache_mut.last_populated = Utc::now();
//}

//async fn insert_thread(
//    db_conn: &mut MySqlConnection,
//    thread: &MinThread,
//) -> Result<(), sqlx::Error> {
//    sqlx::query!(
//        "INSERT INTO threads (id, guild, event, year) VALUES (?, ?, ?, ?)",
//        thread.id,
//        thread.guild,
//        thread.event,
//        thread.year
//    )
//    .execute(db_conn)
//    .await?;
//    Ok(())
//}

//async fn mark_doc_done(
//    db_conn: &mut MySqlConnection,
//    doc: &ImageDoc,
//) -> Result<(), Box<dyn Error>> {
//    let t =
//        sqlx::query!("UPDATE documents SET notified = 1 WHERE id = ?", doc.id)
//            .execute(db_conn)
//            .await?;
//    if t.rows_affected() == 0 {
//        return Err(String::from("Rows affected = 0").into());
//    }
//    Ok(())
//}

//async fn populate_cache(
//    db_conn: &mut MySqlConnection,
//    cache: &mut ThreadCache,
//) {
//    if (Utc::now() - cache.last_populated).num_days() < 1 {
//        return;
//    }
//    let year = Utc::now().year();
//    let data = match sqlx::query_as!(
//        MinThread,
//        r#"
//        SELECT id, guild, event, year FROM threads WHERE year = ?
//    "#,
//        year
//    )
//    .fetch_all(db_conn)
//    .await
//    {
//        Ok(data) => data,
//        Err(why) => {
//            error!("Error populating threads: {why}");
//            return;
//        },
//    };
//    info!("populated thread cache");
//    cache.last_populated = Utc::now();
//    cache.cache = data;
//}
