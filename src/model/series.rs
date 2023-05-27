use serde::{Serialize, Deserialize};


/// This struct represents a racing series (F1, F2, F3, WRC, etc...)
/// NOTE: THIS IS CURRENTLY NOT IN USE!
#[derive(Serialize, Deserialize)]
pub struct Series {
    pub id: u64,
    pub name: String,
    pub short_handle: String,
}
