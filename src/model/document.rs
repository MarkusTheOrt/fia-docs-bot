use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use super::series::RacingSeries;

#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: u64,
    pub event: u64,
    pub series: RacingSeries,
    pub url: String,
    pub title: String,
    pub date: DateTime<Utc>,
    pub image: String,
    pub notified: u8,
}
