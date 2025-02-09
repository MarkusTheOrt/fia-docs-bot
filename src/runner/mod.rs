use f1_bot_types::Event;
use libsql::Connection;
use tracing::{error, info};

pub async fn fetch_unallowed_events(
    db_conn: &Connection
) -> Result<Vec<Event>, Box<dyn std::error::Error>> {
    let mut stmt = db_conn.prepare("SELECT * FROM events WHERE status = \"NotAllowed\" AND year = strftime('%Y', current_timestamp)").await?;
    let mut data = stmt.query(()).await?;
    let mut return_value = Vec::new();
    while let Ok(Some(data)) = data.next().await {
        info!("{data:#?}");
        match libsql::de::from_row::<Event>(&data) {
            Ok(event) => return_value.push(event),
            Err(why) => error!("{why:#?}")
        }
    }
    Ok(return_value)
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
