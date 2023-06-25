pub mod event;
pub mod season;
pub mod series;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use sqlx::Pool;

use crate::model::{event::Event, series::Series};

pub async fn home() -> (StatusCode, HeaderMap) {
    let mut headers = HeaderMap::new();
    headers.append("location", "/current".parse().unwrap());
    return (StatusCode::MOVED_PERMANENTLY, headers);
}

pub async fn fallback() -> (StatusCode, Json<&'static str>) {
    return (StatusCode::NOT_FOUND, Json("not found"));
}

pub async fn series_current(
    Path(series): Path<Series>,
    State(database): State<Pool<sqlx::MySql>>,
) -> String {
    return format!("series: {series}");
}

pub async fn current(
    State(database): State<Pool<sqlx::MySql>>
) -> Result<(StatusCode, Json<Event>), (StatusCode, Json<&'static str>)> {
    let data = match sqlx::query_as_unchecked!(
        Event,
        r#"SELECT `id` as `id?` ,`name`,`year`,`created`,`series`
    FROM events WHERE `current` = 1"#
    )
    .fetch_optional(&database)
    .await
    {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err((StatusCode::NOT_FOUND, Json("No Active event.")));
        },
        Err(why) => {
            eprintln!("{why}");
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json("Database Error"),
            ));
        },
    };

    return Ok((StatusCode::OK, Json(data)));
}
