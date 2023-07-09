use std::time::{Duration, UNIX_EPOCH};

use chrono::DateTime;
use chrono::Datelike;
use chrono::Utc;
use serenity::all::ChannelId;
use serenity::builder::CreateEmbed;
use serenity::builder::CreateEmbedAuthor;
use serenity::builder::CreateEmbedFooter;
use serenity::builder::CreateMessage;
use serenity::builder::CreateThread;
use serenity::prelude::Context;
use sqlx::{MySql, Pool};

use crate::model::series::RacingSeries;

#[derive(Debug)]
pub struct JoinRes {
    id: u64,
    channel: u64,
    role: Option<u64>,
    threads: bool,
    //   thread: Option<u64>
}

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

#[derive(Debug, Clone)]
struct MinImg {
    url: String,
    page: u32,
}

#[derive(Debug, Clone)]
pub struct ImageDoc {
    id: u64,
    event: u64,
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

        return_array.push(main_embed);

        for image in value.images.into_iter().take(4) {
            return_array.push(CreateEmbed::new().url(value.url.clone()).image(image.url));
        }
        drop(value.url);
        return return_array;
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
pub async fn runner(ctx: Context, pool: Pool<MySql>) {
    let mut thread_cache = ThreadCache::default();
    loop {
        populate_cache(&pool, &mut thread_cache).await;
        std::thread::sleep(Duration::from_secs(5));
        let f1_guild_data = match join_query(&pool, RacingSeries::F1).await {
            Ok(data) => data,
            Err(why) => {
                eprintln!("Error reading guilds from database:\n{why}");
                continue;
            }
        };
        let docs = match unposted_documents(&pool, RacingSeries::F1).await {
            Ok(data) => join_to_doc(data),
            Err(why) => {
                eprintln!("Error reading unposted docs from db:\n{why}");
                continue;
            }
        };

        for doc in docs.into_iter() {
            if let Err(why) = mark_doc_done(&pool, &doc).await {
                println!("Error marking doc as done:\n{why}");
                continue;
            }
            for guild in f1_guild_data.iter() {
                if guild.threads {
                    let msg = create_message(&doc, guild.role);
                    if let Some(thread) = thread_cache
                        .cache
                        .iter()
                        .find(|p| p.guild == guild.id && p.event == doc.event)
                    {
                        let id = ChannelId::new(thread.id);
                        let embeds: Vec<CreateEmbed> = doc.clone().into();

                        if let Err(why) = id.send_message(&ctx, msg).await {
                            println!("Error sending msg in thread: {why}");
                            continue;
                        }
                    } else {
                        let audit_log_reason = format!(
                            "Thread for event `{} {}` not found.",
                            doc.created.year(),
                            doc.event_name
                        );
                        let channel_id = ChannelId::new(guild.channel);
                        let create_thread = CreateThread::new(format!(
                            "{} {} {}",
                            doc.created.year(),
                            RacingSeries::F1,
                            doc.event_name
                        ))
                        .audit_log_reason(&audit_log_reason)
                        .kind(serenity::all::ChannelType::PublicThread)
                        .auto_archive_duration(serenity::all::AutoArchiveDuration::ThreeDays);

                        let thread = match channel_id.create_thread(&ctx, create_thread).await {
                            Err(why) => {
                                println!("Error creating thread: {why}");
                                continue;
                            }
                            Ok(data) => data,
                        };
                        let min_thread = MinThread {
                            id: thread.id.get(),
                            guild: guild.id,
                            event: doc.event,
                            year: doc.created.year(),
                        };
                        if let Err(why) = insert_thread(&pool, &min_thread).await {
                            println!("Error creating thread: {why}");
                        }
                        thread_cache.cache.push(min_thread);
                        if let Err(why) = thread.send_message(&ctx, msg).await {
                            println!("Couldn't send into new thread: {why}");
                            continue;
                        }
                    }
                } else {
                    let id = ChannelId::new(guild.id);
                    if let Err(why) = id
                        .send_message(&ctx, create_message(&doc, guild.role))
                        .await
                    {
                        println!("Error sending channel embed: {why}");
                    }
                }
            }
        }
    }
}

async fn insert_thread(pool: &Pool<MySql>, thread: &MinThread) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO threads (id, guild, event, year) VALUES (?, ?, ?, ?)",
        thread.id,
        thread.guild,
        thread.event,
        thread.year
    )
    .execute(pool)
    .await?;
    return Ok(());
}

async fn mark_doc_done(pool: &Pool<MySql>, doc: &ImageDoc) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE documents SET notified = 1 WHERE id = ?", doc.id)
        .execute(pool)
        .await?;
    return Ok(());
}

fn create_message(doc: &ImageDoc, role: Option<u64>) -> CreateMessage {
    let embeds: Vec<CreateEmbed> = doc.clone().into();
    let message = CreateMessage::new().embeds(embeds);
    if let Some(role) = role {
        return message.content(format!("<@&{}>", role));
    }
    return message;
}

async fn populate_cache(pool: &Pool<MySql>, cache: &mut ThreadCache) {
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
        }
    };

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
    return docs;
}

async fn unposted_documents(
    pool: &Pool<MySql>,
    racing_series: RacingSeries,
) -> Result<Vec<JoinImage>, sqlx::Error> {
    return sqlx::query_as_unchecked!(
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
    .await;
}

async fn join_query(
    pool: &Pool<MySql>,
    racing_series: RacingSeries,
) -> Result<Vec<JoinRes>, sqlx::Error> {
    match racing_series {
        RacingSeries::F1 => {
            return sqlx::query_as_unchecked!(
                JoinRes,
                r#"
    SELECT
    id,
    f1_channel as `channel!`,
    f1_threads as threads,
    f1_role as role
    FROM guilds
    WHERE
    f1_channel IS NOT NULL"#
            )
            .fetch_all(pool)
            .await;
        }
        RacingSeries::F2 => {
            return sqlx::query_as_unchecked!(
                JoinRes,
                r#"
    SELECT
    id,
    f2_channel as `channel!`,
    f2_threads as threads,
    f2_role as role
    FROM guilds
    WHERE
    f2_channel IS NOT NULL"#
            )
            .fetch_all(pool)
            .await;
        }
        RacingSeries::F3 => {
            return sqlx::query_as_unchecked!(
                JoinRes,
                r#"
    SELECT
    id,
    f3_channel as `channel!`,
    f3_threads as threads,
    f3_role as role
    FROM guilds
    WHERE
    f3_channel IS NOT NULL"#
            )
            .fetch_all(pool)
            .await;
        }
    }
}
