use axum::{extract::Path, Json};

use crate::{bodies::series::Series as SeriesResponse, model::series::Series};

pub async fn series(
    Path(series): Path<Series>
) -> Json<SeriesResponse> {
    return match series {
        Series::F1 => Json(SeriesResponse::f1()),
        Series::F2 => Json(SeriesResponse::f2()),
        Series::F3 => Json(SeriesResponse::f3())
    };
}
