use axum::{extract::{Path, State}, Json};
use reqwest::StatusCode;
use sqlx::{MySql, Pool};

use crate::model::{event::Event, series::Series};

pub async fn season(
    Path((series, year)): Path<(Series, u32)>,
    State(state): State<crate::State>,
) -> Result<Json<Vec<Event>>, (StatusCode, &'static str)> {
    let database = &state.pool;
    let series: String = series.into();
    let data: Vec<Event> = match sqlx::query_as_unchecked!(
        Event,
        "SELECT `id` as `id?`, series, name, year, created FROM events WHERE series = ? AND year = ?",
        series,
        year
    )
    .fetch_all(database)
    .await
    {
        Err(why) => {
            eprintln!("Database Error: {why}");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Database Error."));
        },
        Ok(data) => data,
    };
    
    if data.is_empty() {
        return Err((StatusCode::NOT_FOUND, "not found."));
    }
    
    return Ok(Json(data));
}
