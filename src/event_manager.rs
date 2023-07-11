use std::{process::exit, sync::Arc, time::UNIX_EPOCH};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use serenity::{
    all::{
        ActivityType, Guild, GuildId, Interaction, PartialGuild, ResumedEvent, UnavailableGuild,
    },
    async_trait,
    prelude::*,
};

use sqlx::{MySql, Pool};

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

#[derive(Clone)]
pub struct CachedGuild {
    pub id: u64,
    pub f1: SeriesSettings,
    pub f2: SeriesSettings,
    pub f3: SeriesSettings,
}

impl From<AllGuild> for CachedGuild {
    fn from(value: AllGuild) -> Self {
        return Self {
            id: value.id,
            f1: SeriesSettings { channel: value.f1_channel, use_threads: value.f1_threads, role: value.f1_role },
            f2: SeriesSettings { channel: value.f2_channel, use_threads: value.f2_threads, role: value.f2_role },
            f3: SeriesSettings { channel: value.f3_channel, use_threads: value.f3_threads, role: value.f3_role }
        };
    }
}

pub struct GuildCache {
    pub last_populated: DateTime<Utc>,
    pub cache: Vec<CachedGuild>,
}

impl CachedGuild {
    pub fn new(id: u64) -> Self {
        return Self {
            id,
            f1: SeriesSettings { channel: None, use_threads: true, role: None },
            f2: SeriesSettings { channel: None, use_threads: true, role: None },
            f3: SeriesSettings { channel: None, use_threads: true, role: None },
        }
    }
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
    pub pool: Pool<MySql>,
    pub guild_cache: Arc<Mutex<GuildCache>>,
}

#[async_trait]
impl EventHandler for BotEvents {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let res = match ctx.http.get_current_application_info().await {
            Ok(res) => res,
            Err(why) => {
                println!("error receiving Application Info: {why}");
                exit(0x0100);
            }
        };

        let user = match ctx.http().get_current_user().await {
            Ok(data) => data,
            Err(why) => {
                println!("error reciving app user: {why}");
                exit(0x0100);
            }
        };

        let thread_ctx = ctx.clone();
        let thread_db_pool = self.pool.clone();
        let thread_cache = self.guild_cache.clone();
        std::thread::spawn(move || {
            runner(thread_ctx, thread_db_pool, thread_cache);
        });

        //{
        //    let mut fut = vec![];
        //
        //    if let Ok(commands) = ctx.http().get_global_commands().await {
        //        for command in commands {
        //            fut.push(ctx.http.delete_global_command(command.id));
        //        }
        //    }
        //
        //    for fut in join_all(fut).await {
        //        if let Err(why) = fut {
        //            println!("error removing command: {why}");
        //        }
        //    }
        //}

        {
            if let Err(why) = ctx.http.create_global_command(&set::register()).await {
                println!("Error registering command: {why}");
                exit(0x0100);
            }
        }

        ctx.set_activity(Some(serenity::gateway::ActivityData {
            name: "fia.com/documents".to_owned(),
            kind: ActivityType::Watching,
            url: None,
        }));

        println!(
            "Started up {}#{}",
            user.name,
            user.discriminator.expect("Discriminator galore!")
        );
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(cmd) => {
                if let Err(why) = match cmd.data.name.as_str() {
                    "settings" => run(&self.pool, &ctx, cmd, &self.guild_cache).await,
                    _ => unimplemented(&ctx, cmd).await,
                } {
                    println!("cmd error: {why}")
                }
            }
            _ => {}
        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: Option<bool>) {
        println!("guild: {}", guild.name);
        if let Err(why) = insert_new_guild(&guild, &self.pool, &self.guild_cache).await {
            println!("Error inserting new guild: {why}");
        }
    }

    async fn guild_update(&self, _ctx: Context, _old: Option<Guild>, new_incomplete: PartialGuild) {
        if let Err(why) = update_guild_name(&new_incomplete, &self.pool).await {
            println!("Error updating guild: {why}");
        }
    }

    async fn guild_delete(&self, _ctx: Context, incomplete: UnavailableGuild, full: Option<Guild>) {
        if incomplete.unavailable {
            return;
        }

        let guild = match full {
            Some(guild) => guild,
            None => return,
        };
        if let Err(why) = sqlx::query!("DELETE FROM guilds WHERE id = ?", guild.id.get())
            .execute(&self.pool)
            .await
        {
            println!("Error removing guild: {why}");
        }
    }

    async fn resume(&self, _ctx: Context, _data: ResumedEvent) {}
}
