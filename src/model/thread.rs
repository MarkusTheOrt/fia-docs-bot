use serde::{Serialize, Deserialize};
use serenity::model::prelude::{GuildId, ChannelId};


#[derive(Serialize, Deserialize)]
pub struct Thread {
    guild: GuildId,
    event: u64,
    id: ChannelId
}
