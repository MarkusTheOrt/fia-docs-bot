use serde::{Deserialize, Serialize};

use super::series::RacingSeries;

#[derive(Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub name: String,
    pub year: u16,
    pub series: RacingSeries,
}
