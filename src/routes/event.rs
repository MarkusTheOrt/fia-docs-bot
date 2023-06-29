use axum::extract::Path;

pub async fn events(Path((year, event)): Path<(u32, String)>) -> String {
    return format!("year: {year}, event: {event}");
}
