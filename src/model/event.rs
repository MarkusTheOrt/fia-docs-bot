use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::series::Series;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Event {
    pub id: Option<i64>,
    pub series: Series,
    pub year: i32,
    pub name: String,
    pub created: DateTime<Utc>,
}
