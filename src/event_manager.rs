use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::{process::exit, sync::Arc, time::UNIX_EPOCH};

use chrono::{DateTime, Utc};
use serenity::futures::future::join_all;
use serenity::{
    all::{
        ActivityType, Guild, GuildId, Interaction, PartialGuild, ResumedEvent,
        UnavailableGuild,
    },
    async_trait,
    prelude::*,
};

use sqlx::MySqlPool;
use tracing::{error, info};

use crate::runner::AllGuild;
use crate::{
    commands::{
        set::{self, run},
        unimplemented,
    },
    model::guild::{insert_new_guild, update_guild_name},
    runner::runner,
};

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

impl From<AllGuild> for CachedGuild {
    fn from(value: AllGuild) -> Self {
        Self {
            id: value.id,
            f1: SeriesSettings {
                channel: value.f1_channel,
                use_threads: value.f1_threads,
                role: value.f1_role,
            },
            f2: SeriesSettings {
                channel: value.f2_channel,
                use_threads: value.f2_threads,
                role: value.f2_role,
            },
            f3: SeriesSettings {
                channel: value.f3_channel,
                use_threads: value.f3_threads,
                role: value.f3_role,
            },
        }
    }
}

pub struct GuildCache {
    pub last_populated: DateTime<Utc>,
    pub cache: Vec<CachedGuild>,
}

impl Default for GuildCache {
    fn default() -> Self {
        Self {
            last_populated: DateTime::from(UNIX_EPOCH),
            cache: vec![],
        }
    }
}

pub struct BotEvents {
    pub db: &'static MySqlPool,
    pub guild_cache: Arc<Mutex<GuildCache>>,
    pub thread_lock: AtomicBool,
}

#[async_trait]
impl EventHandler for BotEvents {

    async fn ready(&self, ctx: Context, _ready: serenity::all::Ready) {
        info!("Ready called");
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

        if !self.thread_lock.load(Ordering::Relaxed) {
            self.thread_lock.swap(true, Ordering::Relaxed);
            let thread_ctx = ctx.clone();
            let thread_db_pool = self.db;
            let thread_cache = self.guild_cache.clone();
            std::thread::spawn(move || {
                runner(thread_ctx, thread_db_pool, thread_cache);
            });
        }

        {
            let mut fut = vec![];

            if let Ok(commands) = ctx.http().get_global_commands().await {
                for command in commands {
                    fut.push(ctx.http.delete_global_command(command.id));
                }
            }

            for fut in join_all(fut).await {
                if let Err(why) = fut {
                    error!("removing command: {why}");
                }
            }
        }

        {
            if let Err(why) =
                ctx.http.create_global_command(&set::register()).await
            {
                error!("Error registering command: {why}");
                exit(0x0100);
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
        ctx: Context,
        _guilds: Vec<GuildId>,
    ) {

    }

    async fn interaction_create(
        &self,
        ctx: Context,
        interaction: Interaction,
    ) {
        if let Interaction::Command(cmd) = interaction {
            if let Err(why) = match cmd.data.name.as_str() {
                "settings" => run(self.db, &ctx, cmd, &self.guild_cache).await,
                _ => unimplemented(&ctx, cmd).await,
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
        if let Err(why) =
            insert_new_guild(&guild, self.db, &self.guild_cache).await
        {
            error!("Error inserting new guild: {why}");
        }
    }

    async fn guild_update(
        &self,
        _ctx: Context,
        _old: Option<Guild>,
        new_incomplete: PartialGuild,
    ) {
        if let Err(why) = update_guild_name(&new_incomplete, self.db).await {
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
        if let Err(why) =
            sqlx::query!("DELETE FROM guilds WHERE id = ?", guild.id.get())
                .execute(self.db)
                .await
        {
            error!("Error removing guild: {why}");
        }
    }

    async fn resume(
        &self,
        _ctx: Context,
        _data: ResumedEvent,
    ) {
    }
}
