use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::series::Series;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub id: Option<u64>,
    pub url: String,
    pub page: u8,
    pub document: u64,
    pub created: DateTime<Utc>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    pub id: Option<u64>,
    pub event: u64,
    pub title: String,
    pub series: Series,
    pub created: DateTime<Utc>,
    pub url: String,
    pub mirror: String,
    pub notified: bool,
}
