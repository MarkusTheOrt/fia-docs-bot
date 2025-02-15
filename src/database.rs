#![allow(unused)]
use crate::error::Result;
use f1_bot_types::{Event, EventStatus, Series};
use libsql::{de, params, Connection};
use serde::Serialize;

pub async fn fetch_events_by_status(
    db_conn: &Connection,
    status: EventStatus,
) -> Result<Vec<Event>> {
    let mut res = db_conn
        .query(
            r#"SELECT * FROM events 
    WHERE status = ? 
    AND year = strftime('%Y', current_timestamp)"#,
            [status],
        )
        .await?;
    let mut return_value = Vec::new();
    while let Ok(Some(data)) = res.next().await {
        return_value.push(de::from_row::<Event>(&data)?);
    }
    Ok(return_value)
}

pub async fn get_event_by_id(
    db_conn: &Connection,
    id: u64,
) -> Result<Option<Event>> {
    let mut res =
        db_conn.query("SELECT * FROM events WHERE id = ?", [id]).await?;
    res.next()
        .await?
        .map(|f| libsql::de::from_row::<Event>(&f))
        .transpose()
        .map_err(|e| e.into())
}

pub async fn update_event_status(
    db_conn: &Connection,
    event: &Event,
    new_status: EventStatus,
) -> Result {
    db_conn
        .execute(
            r#"UPDATE events SET status = ? WHERE id = ?"#,
            params![new_status, event.id],
        )
        .await?;
    Ok(())
}
