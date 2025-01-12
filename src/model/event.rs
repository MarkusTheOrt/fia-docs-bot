use f1_bot_types::Series;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub name: String,
    pub year: u16,
    pub series: Series,
}
