use serenity::{
    async_trait,
    model::{
        prelude::{Activity, Guild, GuildId, Interaction, PartialGuild, UnavailableGuild},
        user::OnlineStatus,
    },
    prelude::{Context, EventHandler},
};
use sqlx::{MySql, Pool};

use crate::{model::guild::{insert_new_guild, update_guild_name}, commands};

pub struct BotEvents {
    pub pool: Pool<MySql>,
}

#[async_trait]
impl EventHandler for BotEvents {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        #[cfg(debug_assertions)]
        let _ = ctx
            .set_presence(
                Some(Activity::playing("FIA Docs Bot DEV!")),
                OnlineStatus::Online,
            )
            .await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            if let Err(why) = match command.data.name.as_str() {
                "ping" => commands::ping::run(&ctx, &command).await,
                _ => commands::unimplemented(&ctx, &command).await
            } {
                println!("Error responding to command: {why}");
                let _ = command.create_interaction_response(&ctx, |f| {
                    return f.interaction_response_data(|f| {
                        return f.ephemeral(true).content("Internal error occured.");
                    })
                }).await;
            }

        }
    }

    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            match insert_new_guild(&guild, &self.pool).await {
                Ok(_) => {
                    println!("Inserted new guild {}", guild.name);
                }
                Err(why) => {
                    println!("Error inserting new guild: {why}");
                }
            }
        }
    }

    async fn guild_update(&self, _ctx: Context, _old: Option<Guild>, new_incomplete: PartialGuild) {
        match update_guild_name(&new_incomplete, &self.pool).await {
            Ok(_) => {
                println!(
                    "Updated guild: {}; new Name: {}",
                    new_incomplete.id, new_incomplete.name
                );
            }
            Err(why) => {
                println!("Error updating guild: {why}");
            }
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
        match sqlx::query!("DELETE FROM guilds WHERE id = ?", guild.id.as_u64())
            .execute(&self.pool)
            .await
        {
            Ok(_) => {
                println!("Removed guild {}", &guild.name);
            }
            Err(why) => {
                println!("Error removing guild: {why}");
            }
        }
    }
}
