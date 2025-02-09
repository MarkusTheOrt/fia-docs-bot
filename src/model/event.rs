use chrono::{DateTime, Utc};
use f1_bot_types::Series;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: u64,
    pub title: String,
    pub year: u16,
    pub series: Series,
    pub created: DateTime<Utc>,
    // Whether or not the Event is posted.
    pub posted: u8,
    // Whether or not the Event is allowed by the botowner.
    pub allowed: u8,
}

use libsql::de;

#[allow(unused)]
pub async fn fetch_events(
    db_conn: &libsql::Connection
) -> libsql::Result<Vec<Event>> {
    let mut stmt = db_conn.prepare("SELECT * FROM events").await?;
    
    let mut rows = stmt.query(()).await?;
    let mut events = Vec::new();

    while let Ok(Some(row)) = rows.next().await {

        if let Ok(event) = de::from_row::<Event>(&row) {
            events.push(event);
        }
    }

    Ok(events)
}
