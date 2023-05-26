use serenity::{
    async_trait,
    model::{
        prelude::{Activity, Guild, GuildId, Interaction, UnavailableGuild},
        user::OnlineStatus,
    },
    prelude::{Context, EventHandler},
};
use sqlx::{Pool, MySql};

pub struct BotEvents {
    pub pool: Pool<MySql>,
}

#[async_trait]
impl EventHandler for BotEvents {
    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        #[cfg(debug_assertions)]
        let _ = ctx.set_presence(
            Some(Activity::playing("FIA Docs Bot DEV!")),
            OnlineStatus::Online,
        ).await;
    }

    async fn interaction_create(&self, _ctx: Context, _interaction: Interaction) {}

    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: bool) {
        println!("Guild created: {} is_new: {}", guild.name, is_new);

        if is_new {
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
        let _ = sqlx::query!("DELETE FROM guilds WHERE id = ?", guild.id.as_u64()).execute(&self.pool).await;

    }
}
