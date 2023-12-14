use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::series::RacingSeries;

#[derive(Serialize, Deserialize, Debug)]
pub struct Document {
    pub id: Option<u64>,
    pub event: Option<u64>,
    pub series: RacingSeries,
    pub url: String,
    pub title: String,
    pub date: DateTime<Utc>,
    pub image: Option<String>,
    pub notified: i8,
}

impl Document {
    pub fn new(
        series: RacingSeries,
        url: &str,
        title: &str,
        date: DateTime<Utc>,
    ) -> Self {
        return Self {
            id: None,
            event: None,
            series,
            url: url.to_owned(),
            title: title.to_owned(),
            date,
            image: None,
            notified: 0,
        };
    }
}
