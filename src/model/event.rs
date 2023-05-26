use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize)]
pub struct Event {
    id: u64,
    name: String,
    year: u16,
}
