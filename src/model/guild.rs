use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use serenity::model::prelude::{GuildId, ChannelId, RoleId};


#[derive(Serialize, Deserialize)]
pub struct Guild {
    pub id: GuildId,
    pub name: String,
    pub channel: Option<ChannelId>,
    pub role: Option<RoleId>,
    pub joined: DateTime<Utc>
}



