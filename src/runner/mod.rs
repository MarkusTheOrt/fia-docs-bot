use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use f1_bot_types::{Event, EventStatus};
use libsql::{Connection, de, params};
use notifbot_macros::notifbot_enum;
use sentry::{
    TransactionContext, User,
    protocol::{SpanStatus, Value},
};
use serenity::{
    all::{
        ChannelId, Context, CreateActionRow, CreateButton, CreateEmbed,
        CreateMessage,
    },
    futures::future::join_all,
};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::database::{
    self, create_message, create_new_thread, fetch_docs_for_event,
    fetch_events_by_status, fetch_guilds, fetch_images_for_document,
    fetch_thread_for_guild_and_event, mark_doc_done, mark_event_done,
};

const REQUEST_CHANNEL_ID: u64 = 1151509515066421302;

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

#[tracing::instrument(skip(db_conn))]
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

#[tracing::instrument(skip(db_conn))]
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

#[tracing::instrument]
pub async fn create_discord_allow_request(
    ctx: &Context,
    event: &Event,
    request_id: i64,
) -> Result<(), crate::error::Error> {
    ChannelId::new(REQUEST_CHANNEL_ID)
        .send_message(
            ctx,
            CreateMessage::new()
                .content("<@142951266811641856> & <@&738665034359767060>")
                .embed(
                    CreateEmbed::new().title("New Event Found!").description(
                        format!(
                            "## {} {} {}\n\nPlease allow or deny.\n\nEvent Information: ```ron{event:#?}```",
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
        let transaction = sentry::start_transaction(TransactionContext::new(
            "runner",
            "main-task",
        ));

        let child = transaction.start_child("db", "Fetch Events");
        child.set_data("status", Value::String(EventStatus::NotAllowed.into()));
        let not_allowed_events =
            fetch_events_by_status(db_conn, EventStatus::NotAllowed).await?;

        child.set_status(SpanStatus::Ok);
        child.finish();

        for event in not_allowed_events.into_iter() {
            let span = transaction.start_child("db", "Has Allow Requests");
            if has_allow_request(db_conn, &event).await?.is_none() {
                let ca_span = span.start_child("db", "Create Allow Request");
                ca_span.set_data(
                    "allow-request",
                    serde_json::to_value(&event).unwrap(),
                );
                create_allow_request(db_conn, &event, ctx).await?;
                ca_span.set_status(SpanStatus::Ok);
                ca_span.finish();
            }
            span.set_status(SpanStatus::Ok);
            span.finish();
        }

        let span = transaction.start_child("db", "Fetch Events");
        span.set_data("status", Value::String(EventStatus::Allowed.into()));
        let allowed_events =
            fetch_events_by_status(db_conn, EventStatus::Allowed).await?;
        span.set_status(SpanStatus::Ok);
        span.finish();

        struct QueuedGuild {
            guild_id: String,
            channel_to_post: ChannelId,
            role: Option<String>,
            event_id: i64,
        }

        let mt_queued_guilds = Arc::new(Mutex::new(Vec::new()));

        for event in allowed_events.into_iter() {
            let span = transaction.start_child("main-task", "Handle Event");
            span.set_data("event", serde_json::to_value(&event).unwrap());
            if (Utc::now() - event.created_at).num_days() > 10 {
                mark_event_done(db_conn, event.id as i64).await?;
            }
            let gspan = &span;
            let (ids, guilds, series): (Vec<_>, Vec<_>, Vec<_>) =
                tokio::task::unconstrained(fetch_guilds(db_conn))
                    .await?
                    .into_iter()
                    .map(|f| (f.id, f, event.series))
                    .collect();
            for chunk in guilds.chunks(30) {
                let guild_tasks: Vec<_> = chunk
                    .iter()
                    .map(async |guild| -> crate::error::Result {
                        tokio::task::yield_now().await;
                        let (role, channel, use_threads) =
                            guild.settings_for_series(event.series);
                        let Some(channel) = channel else {
                            return Ok(());
                        };
                        let nspan = gspan.start_child("guild", "Enqueue Guild");
                        nspan.set_data(
                            "guild",
                            serde_json::to_value(guild).unwrap(),
                        );
                        let channel_to_post = if !use_threads {
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
                                    create_new_thread(
                                        db_conn, ctx, guild, &event,
                                    )
                                    .await?
                                    .discord_id
                                },
                            }
                        };
                        {
                            let mut queued_guilds =
                                mt_queued_guilds.lock().await;
                            queued_guilds.push(QueuedGuild {
                                guild_id: guild.discord_id.to_owned(),
                                event_id: event.id as i64,
                                channel_to_post: ChannelId::new(
                                    channel_to_post.parse()?,
                                ),
                                role: role.cloned(),
                            });
                        }
                        nspan.finish();
                        Ok(())
                    })
                    .collect();
                let res = join_all(guild_tasks).await;
                for ((res, _gid), series) in
                    res.chunks(30).zip(ids.chunks(30)).zip(series.chunks(30))
                {
                    for ((result, guild_id), series) in
                        res.iter().zip(_gid).zip(series)
                    {
                        if let Err(why) = result {
                            match why {
                            crate::error::Error::Serenity(e) => match e {
                                serenity::Error::Model(serenity::all::ModelError::InvalidPermissions { .. }) => {
                                        if let Err(why) = database::clear_guild_settings(
                                            db_conn,
                                            guild_id.to_owned(),
                                            series.to_owned()).await
                                        {
                                            sentry::capture_error(&why);
                                        }
                                }
                                serenity::Error::Http(serenity::all::HttpError::UnsuccessfulRequest(e)) => {
                                    match e.error.code {
                                        10003 | 50013 => {
                                        if let Err(why) = database::clear_guild_settings(
                                            db_conn,
                                            guild_id.to_owned(),
                                            series.to_owned()).await
                                        {
                                            sentry::capture_error(&why);
                                        }
                                            },
                                        _ => {}
                                    }
                                }
                                _ => {}
                            },
                            e => {
                                sentry::capture_error(&e);
                            },
                        }
                        }
                    }
                }
            }
            for document in
                fetch_docs_for_event(db_conn, event.id as i64).await?
            {
                let dspan = span.start_child("main-task", "Handle Document");
                dspan.set_data(
                    "document",
                    serde_json::to_value(&document).unwrap(),
                );
                mark_doc_done(db_conn, document.id).await?;
                let images =
                    fetch_images_for_document(db_conn, document.id).await?;
                let message_to_send = create_message(&document, images);
                let queued_guilds = mt_queued_guilds.lock().await;
                for chunk in queued_guilds.chunks(30) {
                    let queued: Vec<_> = chunk
                        .iter()
                        .filter(|f| f.event_id == document.event_id)
                        .map(async |queued| {
                            let hub = sentry::Hub::new_from_top(
                                sentry::Hub::current(),
                            );
                            let _guard = hub.push_scope();
                            hub.configure_scope(|scope| {
                                scope.set_user(Some(User {
                                    id: Some(queued.guild_id.to_string()),
                                    ..Default::default()
                                }))
                            });

                            let mut msg = message_to_send.clone();
                            if let Some(role) = &queued.role {
                                msg = msg.content(format!("<@&{role}>"));
                            }
                            let channel_id = queued.channel_to_post;
                            if let Err(why) =
                                channel_id.send_message(ctx, msg).await
                            {
                                hub.capture_error(&why);
                                error!(
                                    guild_id = queued.guild_id.clone(),
                                    document_id = document.id,
                                    document_title = document.title.clone(),
                                    "{why}"
                                );
                            }
                        })
                        .collect();
                    drop(join_all(queued).await);
                }
                dspan.set_status(SpanStatus::Ok);
                dspan.finish();
            }
            span.set_status(SpanStatus::Ok);
            span.finish();
        }
        transaction.set_status(SpanStatus::Ok);
        transaction.finish();
        mt_queued_guilds.lock().await.clear();
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
