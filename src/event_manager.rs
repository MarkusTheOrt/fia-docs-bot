use std::process::exit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use chrono::Utc;
use f1_bot_types::EventStatus;
use libsql::{params, Connection};
use sentry::Hub;
use sentry::TransactionContext;
use serenity::all::{
    ComponentInteraction, CreateActionRow, CreateButton,
    CreateInteractionResponseFollowup, EditMessage, Message, UserId,
};
use serenity::{
    all::{
        ActivityType, Guild, GuildId, Interaction, PartialGuild, ResumedEvent,
        UnavailableGuild,
    },
    async_trait,
    prelude::*,
};

use tracing::{error, info};

use crate::commands;
use crate::runner::{runner, AllowRequestStatus};

pub async fn allow_request(
    db_conn: &Connection,
    id: i64,
    cmd: ComponentInteraction,
    ctx: &impl CacheHttp,
) -> Result<(), crate::error::Error> {
    cmd.defer(ctx).await?;
    disable_buttons(&cmd.message, ctx, id).await?;
    cmd.create_followup(
        ctx,
        CreateInteractionResponseFollowup::new().content(format!(
            "Allowed by {} ({})",
            cmd.user.name,
            cmd.user.global_name.as_ref().unwrap_or(&"INVALID".to_string())
        )),
    )
    .await?;
    update_allow_request(db_conn, id, cmd.user.id, AllowRequestStatus::Allowed)
        .await?;
    Ok(())
}

pub async fn update_allow_request(
    db_conn: &Connection,
    id: i64,
    user_id: UserId,
    new_status: AllowRequestStatus,
) -> Result<(), crate::error::Error> {
    let tx = db_conn.transaction().await?;

    tx.execute(
        r#"UPDATE events SET status = ?
    WHERE id = (
        SELECT event_id
        FROM allow_requests
        WHERE id = ?
    )
    "#,
        params![
            match new_status {
                AllowRequestStatus::Allowed => EventStatus::Allowed,
                AllowRequestStatus::Denied => EventStatus::Denied,
                _ => EventStatus::NotAllowed,
            },
            id
        ],
    )
    .await?;

    tx.execute(
        r#"UPDATE allow_requests SET response = ?, approved_by = ?, approved_at = ? WHERE id = ?"#,
        params![new_status, user_id.to_string(), Utc::now().to_rfc3339(), id],
    )
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn disable_buttons(
    message: &Message,
    ctx: &impl CacheHttp,
    id: i64,
) -> Result<(), crate::error::Error> {
    message
        .clone()
        .edit(
            ctx,
            EditMessage::new().components(vec![CreateActionRow::Buttons(
                vec![
                    CreateButton::new(format!("allow-{id}"))
                        .label("Allow")
                        .style(serenity::all::ButtonStyle::Success)
                        .disabled(true),
                    CreateButton::new(format!("deny-{id}"))
                        .label("Deny")
                        .style(serenity::all::ButtonStyle::Danger)
                        .disabled(true),
                ],
            )]),
        )
        .await?;

    Ok(())
}

pub async fn deny_request(
    db_conn: &Connection,
    id: i64,
    cmd: ComponentInteraction,
    ctx: &impl CacheHttp,
) -> Result<(), crate::error::Error> {
    cmd.defer(ctx).await?;
    disable_buttons(&cmd.message, ctx, id).await?;
    cmd.create_followup(
        ctx,
        CreateInteractionResponseFollowup::new().content(format!(
            "Denied by {} ({})",
            cmd.user.name,
            cmd.user.global_name.as_ref().unwrap_or(&"INVALID".to_string())
        )),
    )
    .await?;
    update_allow_request(db_conn, id, cmd.user.id, AllowRequestStatus::Denied)
        .await?;

    Ok(())
}

pub struct BotEvents {
    pub thread_lock: AtomicBool,
    pub conn: &'static libsql::Connection,
}

#[async_trait]
impl EventHandler for BotEvents {
    async fn ready(
        &self,
        ctx: Context,
        _ready: serenity::all::Ready,
    ) {
        let tx = sentry::start_transaction(TransactionContext::new(
            "ready", "discord",
        ));

        sentry::metrics::Metric::gauge("guilds", _ready.guilds.len() as f64)
            .send();
        info!("Starting up!");

        let span = tx.start_child("discord", "Get Application Info");
        let _ = match ctx.http.get_current_application_info().await {
            Ok(res) => res,
            Err(why) => {
                sentry::capture_error(&why);
                error!("error receiving Application Info: {why}");
                span.finish();
                exit(0x0100);
            },
        };
        span.finish();

        let span = tx.start_child("discord", "Get Bot Userinfo");
        let user = match ctx.http().get_current_user().await {
            Ok(data) => data,
            Err(why) => {
                sentry::capture_error(&why);
                error!("error reciving app user: {why}");
                span.finish();
                exit(0x0100);
            },
        };
        span.finish();

        let span = tx.start_child("discord", "Create Control-guild Commands");
        if let Err(why) = ctx
            .http
            .create_guild_command(
                GuildId::new(883847530687913995),
                &crate::commands::sync::register(),
            )
            .await
        {
            sentry::capture_error(&why);
            error!("Error creating sync command: {why:#?}");
        }
        if let Err(why) = ctx
            .http
            .create_guild_command(
                GuildId::new(883847530687913995),
                &crate::commands::shutdown::register(),
            )
            .await
        {
            sentry::capture_error(&why);
            error!("Error creating sync command: {why:#?}");
        }
        span.finish();
        tx.set_status(sentry::protocol::SpanStatus::Ok);
        tx.finish();

        if !self.thread_lock.load(Ordering::Relaxed) {
            self.thread_lock.store(true, Ordering::Relaxed);
            if let Err(why) = runner(self.conn, &ctx.clone()).await {
                sentry::capture_error(&why);
                error!("{why:#?}");
            }
        }

        ctx.set_activity(Some(serenity::gateway::ActivityData {
            name: "Watching out for FIA Docs".to_owned(),
            kind: ActivityType::Custom,
            url: None,
            state: None,
        }));

        info!(
            "Started up {}#{}",
            user.name,
            user.discriminator.expect("Discriminator galore!")
        );
    }

    async fn cache_ready(
        &self,
        _ctx: Context,
        _guilds: Vec<GuildId>,
    ) {
        info!("Cache Ready!");
    }

    async fn interaction_create(
        &self,
        ctx: Context,
        interaction: Interaction,
    ) {
        let tx = sentry::start_transaction(TransactionContext::new(
            "interaction-create",
            "discord",
        ));

        let hub = Hub::new_from_top(Hub::current());
        tx.set_data("interaction", serde_json::to_value(&interaction).unwrap());
        match interaction {
            Interaction::Component(cmd) => {
                hub.configure_scope(|scope| {
                    scope.set_user(Some(sentry::User {
                        id: cmd.guild_id.map(|f| f.to_string()),
                        username: Some(format!(
                            "{} ({})",
                            cmd.user.name,
                            cmd.user.global_name.clone().unwrap_or_default()
                        )),
                        ..Default::default()
                    }))
                });
                let Some((kind, id)) = cmd.data.custom_id.split_once("-")
                else {
                    return;
                };
                match match kind {
                    "allow" => {
                        allow_request(self.conn, id.parse().unwrap(), cmd, &ctx)
                            .await
                    },
                    "deny" => {
                        deny_request(self.conn, id.parse().unwrap(), cmd, &ctx)
                            .await
                    },
                    _ => Ok(()),
                } {
                    Ok(_) => {
                        tx.set_status(sentry::protocol::SpanStatus::Ok);
                        tx.finish();
                    },
                    Err(why) => {
                        tx.set_status(sentry::protocol::SpanStatus::Cancelled);
                        hub.capture_error(&why);
                        error!("Interaction Error: {why:#?}");
                        tx.finish();
                    },
                }
            },
            Interaction::Command(cmd) => {
                if let Err(why) = match cmd.data.name.as_str() {
                    "settings" => {
                        commands::set::run(self.conn, &ctx, cmd).await
                    },
                    "sync" => commands::sync::run(&ctx, cmd).await,
                    "shutdown" => commands::shutdown::run(&ctx, cmd).await,
                    "check-repost" => {
                        commands::repost::run(self.conn, &ctx, cmd).await
                    },
                    _ => Ok(()),
                } {
                    tx.set_status(sentry::protocol::SpanStatus::Cancelled);
                    hub.capture_error(&why);
                    error!("cmd error: {why}");
                    tx.finish();
                } else {
                    tx.set_status(sentry::protocol::SpanStatus::Ok);
                    tx.finish();
                }
            },
            _ => {},
        }
    }

    async fn guild_create(
        &self,
        _ctx: Context,
        guild: Guild,
        _is_new: Option<bool>,
    ) {
        match _is_new {
            None | Some(false) => return,
            Some(true) => {},
        }
        let tx = sentry::start_transaction(TransactionContext::new(
            "guild-create",
            "discord",
        ));
        let span = tx.start_child("db", "Insert new Guild");
        if let Err(why) = self
            .conn
            .execute(
                r#"INSERT INTO guilds (discord_id, name, joined_at) 
        VALUES (?, ?, ?) 
        ON CONFLICT(discord_id) 
        DO UPDATE SET 
        name = excluded.name"#,
                params![
                    guild.id.to_string(),
                    guild.name.clone(),
                    guild.joined_at.to_utc().to_rfc3339(),
                ],
            )
            .await
        {
            sentry::capture_error(&why);
            error!("{why}");
        }
        span.finish();
        tx.set_status(sentry::protocol::SpanStatus::Ok);
        tx.finish();
    }

    async fn guild_update(
        &self,
        _ctx: Context,
        old: Option<Guild>,
        new_incomplete: PartialGuild,
    ) {
        let tx = sentry::start_transaction(TransactionContext::new(
            "guild-update",
            "discord",
        ));
        if let Some(guild) = old {
            tx.set_data("old_guild", serde_json::to_value(&guild).unwrap());
        }
        tx.set_data("new_guild_data", serde_json::to_value(&new_incomplete).unwrap());

        let span = tx.start_child("db", "Update guild");
        if let Err(why) = self
            .conn
            .execute(
                r#"UPDATE guilds SET name = ? WHERE discord_id = ?"#,
                params![new_incomplete.name, new_incomplete.id.to_string()],
            )
            .await
        {
            sentry::capture_error(&why);
            error!("Error updating guild: {why}");
        } else {
            tx.set_status(sentry::protocol::SpanStatus::Ok)
        }
        span.finish();
        tx.finish();
    }

    async fn guild_delete(
        &self,
        _ctx: Context,
        incomplete: UnavailableGuild,
        _full: Option<Guild>,
    ) {
        if incomplete.unavailable {
            return;
        }
    }

    async fn resume(
        &self,
        _ctx: Context,
        _data: ResumedEvent,
    ) {
    }
}
