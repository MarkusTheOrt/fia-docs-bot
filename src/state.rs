use std::sync::{Arc, RwLock};

use serenity::model::prelude::GuildId;
use sqlx::{MySql, Pool};

#[derive(Debug, Clone)]
pub struct AppState {
    pub mysql_client: Pool<MySql>,
    pub guild_cache: Arc<RwLock<Vec<GuildId>>>,
}
