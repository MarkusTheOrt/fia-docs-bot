use chrono::{DateTime, Utc};
use f1_bot_types::Series;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Document {
    pub id: Option<u64>,
    pub event: Option<u64>,
    pub series: Series,
    pub url: String,
    pub title: String,
    pub date: DateTime<Utc>,
    pub image: Option<String>,
    pub notified: i8,
}
