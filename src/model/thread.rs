use serde::{Serialize, Deserialize};
use serenity::model::prelude::{GuildId, ChannelId};


#[derive(Serialize, Deserialize)]
pub struct Thread {
    pub guild: GuildId,
    pub event: u64,
    pub id: ChannelId
}
