//pub mod event;
//pub mod season;

use axum::{http::StatusCode, Json};

use crate::bodies::current_event::CurrentEventResponse;

pub async fn home() -> (StatusCode, Json<CurrentEventResponse>) {
    return (StatusCode::OK, Json(CurrentEventResponse::new("Test Grand Prix")));
}

pub async fn fallback() -> (StatusCode, Json<&'static str>) {
    return (StatusCode::NOT_FOUND, Json("not found"));
}
