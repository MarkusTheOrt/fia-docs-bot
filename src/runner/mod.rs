use std::error::Error;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use chrono::DateTime;
use chrono::Datelike;
use chrono::Utc;
use serenity::all::ChannelType::PublicThread;
use serenity::all::{AutoArchiveDuration, ChannelId};
use serenity::builder::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use serenity::builder::CreateMessage;
use serenity::builder::CreateThread;
use serenity::prelude::Context;
use sqlx::{MySql, Pool};

use crate::event_manager::{CachedGuild, GuildCache};
use crate::model::series::RacingSeries;

#[derive(Debug, Clone)]
pub struct JoinImage {
    id: u64,
    event: u64,
    event_name: String,
    created: DateTime<Utc>,
    title: String,
    url: String,
    mirror: String,
    image: String,
    page: u32,
    //   thread_id: u64,
}

pub struct AllGuild {
    pub id: u64,
    pub f1_channel: Option<u64>,
    pub f1_threads: bool,
    pub f1_role: Option<u64>,
    pub f2_channel: Option<u64>,
    pub f2_threads: bool,
    pub f2_role: Option<u64>,
    pub f3_channel: Option<u64>,
    pub f3_threads: bool,
    pub f3_role: Option<u64>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct MinImg {
    url: String,
    page: u32,
}

#[derive(Debug, Clone)]
pub struct ImageDoc {
    id: u64,
    event: u64,
    #[allow(unused)]
    event_name: String,
    title: String,
    url: String,
    mirror: String,
    images: Vec<MinImg>,
    created: DateTime<Utc>,
}

impl From<ImageDoc> for Vec<CreateEmbed> {
    fn from(value: ImageDoc) -> Self {
        let mut return_array = vec![];
        let main_embed = CreateEmbed::new()
            .title(value.title)
            .url(value.url.clone())
            .description(format!("[mirror]({})", value.mirror))
            .colour(0x003063)
            .thumbnail("https://static.ort.dev/fiadontsueme/fia_logo.png")
            .timestamp(value.created)
            .author(CreateEmbedAuthor::new("FIA Document"));

        if let Some(image) = value.images.first() {
            return_array.push(main_embed.image(image.url.clone()));
        } else {
            return_array.push(main_embed);
        }

        for image in value.images.into_iter().skip(1).take(3) {
            return_array.push(
                CreateEmbed::new().url(value.url.clone()).image(image.url),
            );
        }
        drop(value.url);
        return_array
    }
}

pub struct MinThread {
    id: u64,
    guild: u64,
    event: u64,
    year: i32,
}

struct ThreadCache {
    last_populated: DateTime<Utc>,
    cache: Vec<MinThread>,
}

impl Default for ThreadCache {
    fn default() -> Self {
        Self {
            last_populated: DateTime::from(UNIX_EPOCH),
            cache: Vec::with_capacity(100),
        }
    }
}

#[tokio::main]
pub async fn runner(
    ctx: Context,
    pool: Pool<MySql>,
    guild_cache: Arc<Mutex<GuildCache>>,
) {
    let mut thread_cache = ThreadCache::default();
    loop {
        populate_cache(&pool, &mut thread_cache).await;
        populate_guild_cache(&pool, &guild_cache).await;
        std::thread::sleep(Duration::from_secs(5));
        let guilds = {
            let cache = guild_cache.lock().unwrap();
            cache.cache.clone()
        };

        create_threads(&pool, &guilds, &ctx, &mut thread_cache).await;
        run_internal(&pool, RacingSeries::F1, &guilds, &ctx, &mut thread_cache)
            .await;
        run_internal(&pool, RacingSeries::F2, &guilds, &ctx, &mut thread_cache)
            .await;
        run_internal(&pool, RacingSeries::F3, &guilds, &ctx, &mut thread_cache)
            .await;
    }
}

#[derive(Debug, Clone)]
pub struct NewEvent {
    pub id: u64,
    pub name: String,
    pub year: i32,
    pub series: RacingSeries,
}

async fn create_threads(
    pool: &Pool<MySql>,
    guilds: &Vec<CachedGuild>,
    ctx: &Context,
    thread_cache: &mut ThreadCache,
) {
    let events = match new_events(pool).await {
        Ok(data) => data,
        Err(_) => return,
    };
    for event in events.into_iter() {
        if let Err(why) =
            sqlx::query!("UPDATE events SET new = 0 WHERE id = ?", event.id)
                .execute(pool)
                .await
        {
            println!("Error marking event as done: {why}");
            continue;
        }
        for guild in guilds {
            let guild_data = match event.series {
                RacingSeries::F1 => &guild.f1,
                RacingSeries::F2 => &guild.f2,
                RacingSeries::F3 => &guild.f3,
            };

            if guild_data.channel.is_none() {
                continue;
            }

            if !guild_data.use_threads {
                continue;
            }

            let channel = ChannelId::new(guild_data.channel.unwrap());
            let thread = match channel
                .create_thread(
                    ctx,
                    CreateThread::new(format!(
                        "{} {} {}",
                        event.year, event.series, event.name
                    ))
                    .audit_log_reason("New event thread")
                    .kind(PublicThread)
                    .auto_archive_duration(AutoArchiveDuration::ThreeDays),
                )
                .await
            {
                Ok(data) => data,
                Err(why) => {
                    println!(
                        "Error creating thread for guild {}: {why}",
                        guild.id
                    );
                    continue;
                },
            };
            let thread = MinThread {
                id: thread.id.get(),
                guild: guild.id,
                event: event.id,
                year: event.year,
            };
            if let Err(why) = insert_thread(pool, &thread).await {
                println!("Error adding thread to database: {why}");
            }
            thread_cache.cache.push(thread);
        }
    }
}

async fn new_events(pool: &Pool<MySql>) -> Result<Vec<NewEvent>, sqlx::Error> {
    sqlx::query_as_unchecked!(
        NewEvent,
        r#"
    SELECT `id` as `id!`, name, year, series FROM events WHERE new = 1
    "#
    )
    .fetch_all(pool)
    .await
}

async fn run_internal(
    pool: &Pool<MySql>,
    series: RacingSeries,
    guild_cache: &[CachedGuild],
    ctx: &Context,
    thread_cache: &mut ThreadCache,
) {
    let docs = match unposted_documents(pool, series).await {
        Ok(data) => join_to_doc(data),
        Err(why) => {
            eprintln!("Error reading unposted docs from db:\n{why}");
            return;
        },
    };

    for doc in docs.into_iter() {
        if let Err(why) = mark_doc_done(pool, &doc).await {
            println!("Error marking doc as done:\n{why}");
            continue;
        }
        for guild in guild_cache.iter() {
            let guild_data = match series {
                RacingSeries::F1 => &guild.f1,
                RacingSeries::F2 => &guild.f2,
                RacingSeries::F3 => &guild.f3,
            };
            // skip not-set up guilds.
            if guild_data.channel.is_none() {
                continue;
            }
            if guild_data.use_threads {
                let msg = create_message(&doc, guild_data.role);
                if let Some(thread) = thread_cache
                    .cache
                    .iter()
                    .find(|p| p.guild == guild.id && p.event == doc.event)
                {
                    let id = ChannelId::new(thread.id);
                    if let Err(why) = id.send_message(&ctx, msg).await {
                        println!(
                            "Error sending msg in thread: [{}] {why}",
                            guild.id
                        );
                        continue;
                    }
                } else {
                    println!("thread for guild {} not found!", guild.id);
                }
            } else {
                let id = ChannelId::new(guild_data.channel.unwrap());
                if let Err(why) = id
                    .send_message(&ctx, create_message(&doc, guild_data.role))
                    .await
                {
                    println!("Error sending channel embed: {why}");
                }
            }
        }
    }
}

async fn populate_guild_cache(
    pool: &Pool<MySql>,
    guild_cache: &Arc<Mutex<GuildCache>>,
) {
    {
        let cache = guild_cache.lock().unwrap();
        if (Utc::now() - cache.last_populated).num_days() < 1 {
            return;
        }
    }
    let data = match sqlx::query_as_unchecked!(
        AllGuild,
        r#"
    SELECT id, f1_channel, f1_role, f1_threads,
    f2_channel, f2_role, f2_threads,
    f3_channel, f3_role, f3_threads FROM guilds
    "#
    )
    .fetch_all(pool)
    .await
    {
        Ok(data) => data,
        Err(why) => {
            println!("Error fetching guilds: {why}");
            return;
        },
    };
    let mut cache_mut = guild_cache.lock().unwrap();
    cache_mut.cache.clear();
    for guild in data.into_iter() {
        cache_mut.cache.push(guild.into());
    }
    println!("guilds cache populated!");
    cache_mut.last_populated = Utc::now();
}

async fn insert_thread(
    pool: &Pool<MySql>,
    thread: &MinThread,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO threads (id, guild, event, year) VALUES (?, ?, ?, ?)",
        thread.id,
        thread.guild,
        thread.event,
        thread.year
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn mark_doc_done(
    pool: &Pool<MySql>,
    doc: &ImageDoc,
) -> Result<(), Box<dyn Error>> {
    let t =
        sqlx::query!("UPDATE documents SET notified = 1 WHERE id = ?", doc.id)
            .execute(pool)
            .await?;
    if t.rows_affected() == 0 {
        return Err(String::from("Rows affected = 0").into());
    }
    Ok(())
}

fn create_message(
    doc: &ImageDoc,
    role: Option<u64>,
) -> CreateMessage {
    let embeds: Vec<CreateEmbed> = doc.clone().into();
    let message = CreateMessage::new().embeds(embeds);
    if let Some(role) = role {
        return message.content(format!("<@&{}>", role));
    }
    message
}

async fn populate_cache(
    pool: &Pool<MySql>,
    cache: &mut ThreadCache,
) {
    if (Utc::now() - cache.last_populated).num_days() < 1 {
        return;
    }
    let year = Utc::now().year();
    let data = match sqlx::query_as!(
        MinThread,
        r#"
        SELECT id, guild, event, year FROM threads WHERE year = ?
    "#,
        year
    )
    .fetch_all(pool)
    .await
    {
        Ok(data) => data,
        Err(why) => {
            println!("Error populating threads: {why}");
            return;
        },
    };
    println!("poopulated thread cache");
    cache.last_populated = Utc::now();
    cache.cache = data;
}

fn join_to_doc(join_data: Vec<JoinImage>) -> Vec<ImageDoc> {
    let mut docs: Vec<ImageDoc> = Vec::with_capacity(join_data.len());

    for doc_with_img in join_data.into_iter() {
        if let Some(last_doc) = docs.last_mut() {
            if last_doc.id == doc_with_img.id {
                last_doc.images.push(MinImg {
                    url: doc_with_img.image,
                    page: doc_with_img.page,
                });
                continue;
            }
        }

        docs.push(ImageDoc {
            id: doc_with_img.id,
            event: doc_with_img.event,
            event_name: doc_with_img.event_name,
            title: doc_with_img.title,
            url: doc_with_img.url,
            mirror: doc_with_img.mirror,
            images: vec![MinImg {
                url: doc_with_img.image,
                page: doc_with_img.page,
            }],
            created: doc_with_img.created,
        });
    }
    docs
}

async fn unposted_documents(
    pool: &Pool<MySql>,
    racing_series: RacingSeries,
) -> Result<Vec<JoinImage>, sqlx::Error> {
    sqlx::query_as_unchecked!(
        JoinImage,
        r#"
    SELECT
    documents.`id` as `id!`,
    documents.event as event,
    documents.title,
    documents.url,
    documents.mirror,
    documents.created,
    images.url as image,
    images.pagenum as page,
    events.name as event_name
    FROM documents
    JOIN images ON document = documents.id
    JOIN events ON events.id = documents.event
    WHERE documents.series = ? 
    AND notified = 0
    AND done = 1"#,
        Into::<String>::into(racing_series)
    )
    .fetch_all(pool)
    .await
}
