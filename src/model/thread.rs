use serde::{Deserialize, Serialize};
use serenity::model::prelude::{ChannelId, GuildId};

#[derive(Serialize, Deserialize)]
pub struct Thread {
    pub guild: GuildId,
    pub event: u64,
    pub id: ChannelId,
}
