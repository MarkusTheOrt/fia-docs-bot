pub mod event;
pub mod season;
pub mod series;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use sqlx::Pool;

use crate::model::series::Series;

pub async fn home() -> (StatusCode, HeaderMap) {
    let mut headers = HeaderMap::new();
    headers.append("location", "/f1/current".parse().unwrap());
    return (StatusCode::MOVED_PERMANENTLY, headers);
}

pub async fn fallback() -> (StatusCode, Json<&'static str>) {
    return (StatusCode::NOT_FOUND, Json("not found"));
}

pub async fn series_current(
    Path(series): Path<Series>,
) -> String {
    
    return format!("series: {series}");
}
