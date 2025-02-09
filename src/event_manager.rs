use std::process::exit;
use std::sync::{atomic::AtomicBool, Arc};

use libsql::params;
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
    pub shards: Option<Arc<ShardManager>>,
}

#[async_trait]
impl EventHandler for BotEvents {
    async fn ready(
        &self,
        ctx: Context,
        _ready: serenity::all::Ready,
    ) {
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
        if let Interaction::Command(cmd) = interaction {
            if let Err(why) = match cmd.data.name.as_str() {
                "settings" => commands::set::run(self.conn, &ctx, cmd).await,
                "sync" => commands::sync::run(&ctx, cmd).await,
                "shutdown" => commands::shutdown::run(&ctx, cmd).await,
                _ => Ok(()),
            } {
                error!("cmd error: {why}")
            }
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
                r#"INSERT INTO guilds (discord_id, name, date_joined) 
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
