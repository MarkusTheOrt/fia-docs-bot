use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Thread {
    pub id: i64,
    pub discord_id: String,
    // TODO: Rename to parent_channel or something like that.
    pub channel_id: String,
    pub event_id: i64,
    pub guild_id: i64,
    pub created_at: DateTime<Utc>,
}
