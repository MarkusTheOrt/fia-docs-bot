use std::sync::{RwLock, Arc};

use serenity::model::prelude::GuildId;
use sqlx::{Pool, MySql};




#[derive(Debug, Clone)]
pub struct AppState {
    pub mysql_client: Pool<MySql>,
    pub guild_cache: Arc<RwLock<Vec<GuildId>>>
}
