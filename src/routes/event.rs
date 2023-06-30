use crate::model::{series::Series, event::Event, document::{Document, Image}};
use axum::{extract::{Path, State}, Json};
use reqwest::StatusCode;
use serde::Serialize;
use sqlx::{MySql, Pool};

#[derive(Serialize)]
pub struct ReturnType {
    event: Event,
    documents: Vec<ImageDoc>
}

#[derive(Serialize)]
pub struct ImageDoc {
    document: Document,
    images: Vec<Image>
}

pub async fn events(
    Path((series, year, event)): Path<(Series, u32, String)>,
    State(pool): State<Pool<MySql>>,
) -> Result<Json<ReturnType>, (StatusCode, &'static str)> {
    if event.len() > 128 {
        return Err((StatusCode::BAD_REQUEST, "Invalid event identifier."));
    }
    let series: String = series.into();
    let event: Event = match sqlx::query_as_unchecked!(
        Event,
        r#"SELECT 
    `id` as `id?`,
    created,
    name,
    series,
    year
    FROM events where 
    series = ? AND 
    year = ? AND
    name = ?"#,
        series,
        year,
        event
    )
    .fetch_optional(&pool)
    .await {
            Ok(Some(data)) => data,
            Ok(None) => {
                return Err((StatusCode::NOT_FOUND, "Event not found."));
            },
            Err(why) => {
                eprintln!("Error: {why}");
                return Err((StatusCode::BAD_GATEWAY, "Database error."));
            }
        };
    
    let docs: Vec<Document> = match sqlx::query_as_unchecked!(Document, r#"
        SELECT `id` as `id?`,
        created,
        mirror,
        event,
        notified,
        series,
        title,
        url
        FROM documents WHERE
        event = ?
        ORDER BY created DESC
    "#, event.id.unwrap()).fetch_all(&pool).await {
            Ok(data) => data,
            Err(why) => {
                eprintln!("Error: {why}");
                return Err((StatusCode::BAD_GATEWAY, "Database error."));
            }
        };
        
    let mut image_docs = Vec::with_capacity(docs.len());
    for document in docs {
        let images: Vec<Image> = match sqlx::query_as_unchecked!(Image, r#"
            SELECT 
            `id` as `id?`,
            document,
            url,
            pagenum as page,
            created
            FROM images 
            WHERE document = ?
            ORDER BY pagenum ASC
        "#, document.id.unwrap()).fetch_all(&pool).await {
            Ok(data) => data,
            Err(why) => {
                eprintln!("Error: {why}");
                return Err((StatusCode::BAD_GATEWAY, "Database error."));
            }
        };
        image_docs.push(ImageDoc { document, images });
    }
    
    return Ok(Json(ReturnType { event, documents: image_docs }));
}
