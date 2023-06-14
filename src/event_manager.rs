use std::{process::exit, time::Duration, sync::Arc};

use serenity::{
    async_trait,
    model::prelude::{
        Activity, Guild, GuildId, Interaction, PartialGuild, ResumedEvent, UnavailableGuild,
    },
    prelude::{Context, EventHandler},
};
use sqlx::{MySql, Pool};
use tokio::{sync::Mutex};

use crate::{
    commands,
    model::guild::{insert_new_guild, update_guild_name},
};

pub struct BotEvents {
    pub pool: Pool<MySql>,
    pub f1_crawler_enabled: Arc<Mutex<bool>>,
    pub f2_crawler_enabled: Arc<Mutex<bool>>,
    pub f3_crawler_enabled: Arc<Mutex<bool>>,
    pub wrc_crawler_enabled: Arc<Mutex<bool>>,
    pub wrx_crawler_enabled: Arc<Mutex<bool>>,
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
        println!("got app info");

        if let Err(why) = commands::ping::register(&ctx).await {
            println!("Error registering command `ping`: {why}");
            exit(0x0100);
        }
        println!("registered ping");

        if let Err(why) = commands::set_channel::register(&ctx).await {
            println!("Error registering command `set_channel`: {why}");
            exit(0x0100);
        }
        println!("registered set_channel");

        if let Err(why) = commands::unset_channel::register(&ctx).await {
            println!("Error registering command `unset_channel`: {why}");
            exit(0x0100);
        }

        println!("registered unset_channel");

        if let Err(why) = commands::set_role::register(&ctx).await {
            println!("Error registering command `set_role`: {why}");
            exit(0x0100);
        }

        println!("registered set_role");

        if let Err(why) = commands::unset_role::register(&ctx).await {
            println!("Error registering command `unset_role`: {why}");
            exit(0x0100);
        }
        println!("registered unset_role");

        if let Err(why) = commands::stop_crawler::register(&ctx).await {
            println!("Error registering command `stop_crawler`: {why}");
            exit(0x0100);
        }
        println!("registered stop_crawler");

        println!("All commands registered!");
        println!("Started up {}#{}", res.name, res.id);
        let thread_arc = self.f1_crawler_enabled.clone();
        tokio::spawn(async move {
            loop {
                let is_enabled = {
                    let is_enabled = thread_arc.lock().await;
                    *is_enabled
                };
                if !is_enabled {
                    println!("stopped crawling!");
                    break;
                }

                println!("Crawler run");
                std::thread::sleep(Duration::from_millis(500));
            }
        });
        ctx.set_activity(Activity::listening("FIA Documents")).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            if let Err(why) = match command.data.name.as_str() {
                "ping" => commands::ping::run(&ctx, &command).await,
                "set-channel" => commands::set_channel::run(&self.pool, &ctx, &command).await,
                "unset-channel" => commands::unset_channel::run(&self.pool, &ctx, &command).await,
                "set-role" => commands::set_role::run(&self.pool, &ctx, &command).await,
                "stop-crawler" => commands::stop_crawler::run(&self, &ctx, &command).await,
                _ => commands::unimplemented(&ctx, &command).await,
            } {
                println!("Error responding to command: {why}");
                let _ = command
                    .create_interaction_response(&ctx, |f| {
                        return f.interaction_response_data(|f| {
                            return f.ephemeral(true).content("Internal error occured.");
                        });
                    })
                    .await;
            }
        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, _is_new: bool) {
        if let Err(why) = insert_new_guild(&guild, &self.pool).await {
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
        if let Err(why) = sqlx::query!("DELETE FROM guilds WHERE id = ?", guild.id.as_u64())
            .execute(&self.pool)
            .await
        {
            println!("Error removing guild: {why}");
        }
    }

    async fn resume(&self, _ctx: Context, _data: ResumedEvent) {
    }
}
