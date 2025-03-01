use std::time::Duration;

use chrono::{DateTime, Utc};
use f1_bot_types::{Event, EventStatus};
use libsql::{de, params, Connection};
use notifbot_macros::notifbot_enum;
use serenity::all::{
    ChannelId, Context, CreateActionRow, CreateButton, CreateEmbed,
    CreateMessage,
};
use tracing::{info, Level};

use crate::database::{
    create_message, create_new_thread, fetch_docs_for_event,
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
        let runner_span = tracing::span!(Level::INFO, "Runner");
        let sguard = runner_span.enter();
        let not_allowed_events =
            fetch_events_by_status(db_conn, EventStatus::NotAllowed).await?;

        for event in not_allowed_events.into_iter() {
            if has_allow_request(db_conn, &event).await?.is_none() {
                create_allow_request(db_conn, &event, ctx).await?;
            }
        }

        let allowed_events =
            fetch_events_by_status(db_conn, EventStatus::Allowed).await?;

        struct QueuedGuild {
            channel_to_post: ChannelId,
            role: Option<String>,
            event_id: i64,
        }

        let mut queued_guilds = Vec::new();
        for event in allowed_events.into_iter() {
            if (Utc::now() - event.created_at).num_days() > 7 {
                mark_event_done(db_conn, event.id as i64).await?;
            }
            for guild in fetch_guilds(db_conn).await? {
                let (role, channel, use_threads) =
                    guild.settings_for_series(event.series);

                let Some(channel) = channel else {
                    continue;
                };

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
                            if let Ok(thread) =
                                create_new_thread(db_conn, &ctx, &guild, &event)
                                    .await
                            {
                                thread.discord_id
                            } else {
                                continue;
                            }
                        },
                    }
                };

                queued_guilds.push(QueuedGuild {
                    event_id: event.id as i64,
                    channel_to_post: ChannelId::new(channel_to_post.parse()?),
                    role: role.cloned(),
                });
            }
            let posting_span = tracing::info_span!("Posting Documents");
            for document in
                fetch_docs_for_event(db_conn, event.id as i64).await?
            {
                mark_doc_done(db_conn, document.id).await?;
                let images =
                    fetch_images_for_document(db_conn, document.id).await?;
                let message_to_send = create_message(&document, images);
                for queued in queued_guilds
                    .iter()
                    .filter(|f| f.event_id == document.event_id)
                {
                    let _guard = posting_span.enter();
                    let mut msg = message_to_send.clone();
                    if let Some(role) = &queued.role {
                        msg = msg.content(format!("<@&{role}>"));
                    }
                    let channel_id = queued.channel_to_post;
                    let _ = channel_id.send_message(ctx, msg).await;
                }
            }
        }
        queued_guilds.clear();
        drop(sguard);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
