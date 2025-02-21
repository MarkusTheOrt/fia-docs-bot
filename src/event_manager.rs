use std::process::exit;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc};

use chrono::Utc;
use f1_bot_types::EventStatus;
use libsql::{params, Connection};
use sentry::protocol::Request;
use sentry::types::random_uuid;
use sentry::Level;
use serenity::all::{
    ComponentInteraction, CreateActionRow, CreateButton,
    CreateInteractionResponseFollowup, EditMessage, Message, UserId,
};
use serenity::{
    all::{
        ActivityType, Guild, GuildId, Interaction, PartialGuild, ResumedEvent,
        ShardManager, UnavailableGuild,
    },
    async_trait,
    prelude::*,
};

use tracing::{error, info};

use crate::commands;
use crate::runner::{runner, AllowRequestStatus};

#[derive(Clone)]
pub struct SeriesSettings {
    pub channel: Option<u64>,
    pub use_threads: bool,
    pub role: Option<u64>,
}

impl Default for SeriesSettings {
    fn default() -> Self {
        Self {
            channel: None,
            use_threads: true,
            role: None,
        }
    }
}

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

#[derive(Clone)]
pub struct CachedGuild {
    pub id: u64,
    pub f1: SeriesSettings,
    pub f2: SeriesSettings,
    pub f3: SeriesSettings,
}

pub struct BotEvents {
    pub thread_lock: AtomicBool,
    pub conn: &'static libsql::Connection,
}

#[async_trait]
impl RawEventHandler for BotEvents {
    async fn raw_event(
        &self,
        _ctx: Context,
        _ev: serenity::all::Event,
    ) {
        sentry::capture_event(sentry::protocol::Event {
            event_id: random_uuid(),
            request: Some(Request {
                ..Default::default()
            }),
            ..Default::default()
        });
    }
}

#[async_trait]
impl EventHandler for BotEvents {
    async fn ready(
        &self,
        ctx: Context,
        _ready: serenity::all::Ready,
    ) {
        sentry::metrics::Metric::gauge("guilds", _ready.guilds.len() as f64)
            .send();
        info!("Starting up!");

        let _ = match ctx.http.get_current_application_info().await {
            Ok(res) => res,
            Err(why) => {
                error!("error receiving Application Info: {why}");
                exit(0x0100);
            },
        };

        let user = match ctx.http().get_current_user().await {
            Ok(data) => data,
            Err(why) => {
                error!("error reciving app user: {why}");
                exit(0x0100);
            },
        };

        if let Err(why) = ctx
            .http
            .create_guild_command(
                GuildId::new(883847530687913995),
                &crate::commands::sync::register(),
            )
            .await
        {
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
            error!("Error creating sync command: {why:#?}");
        }

        if !self.thread_lock.load(Ordering::Relaxed) {
            self.thread_lock.store(true, Ordering::Relaxed);
            if let Err(why) = runner(self.conn, &ctx.clone()).await {
                error!("{why:#?}");
            }
        }

        ctx.set_activity(Some(serenity::gateway::ActivityData {
            name: "FIA Documents".to_owned(),
            kind: ActivityType::Listening,
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
        match interaction {
            Interaction::Component(cmd) => {
                let Some((kind, id)) = cmd.data.custom_id.split_once("-")
                else {
                    return;
                };
                info!("kind: {kind}; Id: {id}");
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
                    Ok(_) => {},
                    Err(why) => {
                        error!("Interaction Error: {why:#?}");
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
                    error!("cmd error: {why}")
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
            error!("{why}");
        }
    }

    async fn guild_update(
        &self,
        _ctx: Context,
        _old: Option<Guild>,
        new_incomplete: PartialGuild,
    ) {
        if let Err(why) = self
            .conn
            .execute(
                r#"UPDATE guilds SET name = ? WHERE discord_id = ?"#,
                params![new_incomplete.name, new_incomplete.id.to_string()],
            )
            .await
        {
            error!("Error updating guild: {why}");
        }
    }

    async fn guild_delete(
        &self,
        _ctx: Context,
        incomplete: UnavailableGuild,
        full: Option<Guild>,
    ) {
        if incomplete.unavailable {
            return;
        }

        let guild = match full {
            Some(guild) => guild,
            None => return,
        };
        //if let Err(why) =
        //    sqlx::query!("DELETE FROM guilds WHERE id = ?", guild.id.get())
        //        .execute(self.db)
        //        .await
        //{
        //    error!("Error removing guild: {why}");
        //}
    }

    async fn resume(
        &self,
        _ctx: Context,
        _data: ResumedEvent,
    ) {
    }
}
