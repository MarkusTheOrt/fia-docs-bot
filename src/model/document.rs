use chrono::{Utc, DateTime};
use serde::{Serialize, Deserialize};
use sqlx::{Encode, MySql};

use super::series::Series;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub id: Option<u64>,
    pub url: String,
    pub page: u8,
    pub document: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Document {
    pub id: Option<u64>,
    pub title: String,
    pub series: Series,
    pub created: DateTime<Utc>,
    pub url: String,
    pub mirror: String,
    pub notified: bool,
    pub previews: Vec<Image>
}


