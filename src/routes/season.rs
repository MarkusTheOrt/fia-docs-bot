use axum::extract::{Path, State};
use sqlx::{MySql, Pool};

use crate::model::{event::Event, series::Series};
use sqlx::types::chrono::DateTime;

pub async fn season(
    Path((series, year)): Path<(Series, u32)>,
    State(database): State<Pool<MySql>>,
) -> String {

    let series: String = series.into();
    let data: Vec<Event> = match sqlx::query_as_unchecked!(
        Event,
        "SELECT `id` as `id?`, series, name, year, created FROM events WHERE series = ? AND year = ?",
        series,
        year
    )
    .fetch_all(&database)
    .await
    {
        Err(why) => {
            return format!("Error: {why}");
        },
        Ok(data) => data,
    };

    return serde_json::to_string_pretty(&data).unwrap();
}
