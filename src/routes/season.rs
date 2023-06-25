use axum::extract::Path;

use crate::model::series::Series;


pub async fn season(Path((series, year)): Path<(Series, u32)>) -> String {
    return format!("series: {series}, Year: {year}");
}
