use crate::model::{
    document::{Document, Image},
    event::Event,
    series::Series,
};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ReturnType {
    event: Event,
    documents: Vec<ImageDoc>,
}

#[derive(Serialize, Clone)]
pub struct ImageDoc {
    pub document: Document,
    pub images: Vec<Image>,
}

#[derive(Debug, Clone)]
struct JoinQuery {
    id: u64,
    name: String,
    series: Series,
    year: u32,
    created: DateTime<Utc>,
    document_id: u64,
    document_created: DateTime<Utc>,
    document_title: String,
    document_url: String,
    document_mirror: String,
    image_id: u64,
    image_created: DateTime<Utc>,
    image_url: String,
    page: u8,
}

impl From<JoinQuery> for (Document, Image) {
    fn from(value: JoinQuery) -> Self {
        let document = Document {
            id: Some(value.document_id),
            event: value.id,
            title: value.document_title,
            series: value.series,
            created: value.document_created,
            url: value.document_url,
            mirror: value.document_mirror,
            notified: true,
        };
        let image = Image {
            id: Some(value.image_id),
            url: value.image_url,
            page: value.page,
            document: value.document_id,
            created: value.image_created,
        };
        return (document, image);
    }
}

pub async fn events(
    Path((series, year, event_name)): Path<(Series, u32, String)>,
    State(state): State<crate::State>,
) -> Result<(StatusCode, Json<ReturnType>), (StatusCode, &'static str)> {
    let pool = &state.pool;
    let cache = state.cache;
    if event_name.len() > 128 {
        return Err((StatusCode::BAD_REQUEST, "Invalid event identifier."));
    }

    let event_name = event_name.replace('-', " ").to_lowercase();

    {
        let cv = cache.read().unwrap();
        if (Utc::now() - cv.last_populated).num_seconds() < 180 {
            if let Some(cache_data) =
                cv.cache.get(&(series, year, event_name.clone()))
            {
                return Ok((StatusCode::OK, Json(cache_data.clone())));
            }
        }
    }

    let series_str: String = series.into();
    let query: Vec<JoinQuery> = match sqlx::query_as_unchecked!(
        JoinQuery,
        r#"SELECT
            events.id as id,
            events.name as name,
            events.series as series,
            events.year as year,
            events.created as created,
            documents.id as document_id,
            documents.title as document_title,
            documents.created as document_created,
            documents.mirror as document_mirror,
            documents.url as document_url,
            images.id as image_id,
            images.created as image_created,
            images.url as image_url,
            images.pagenum as page
            FROM events
            JOIN documents on documents.event = events.id
            JOIN images on images.document = documents.id
            WHERE events.year = ? AND events.series = ? AND events.name = ?
            ORDER BY events.created DESC, document_created DESC, page"#,
        year,
        series_str,
        event_name
    )
    .fetch_all(pool)
    .await
    {
        Ok(data) => data,
        Err(why) => {
            eprintln!("Error: {why}");
            return Err((StatusCode::INTERNAL_SERVER_ERROR, "Database Error."));
        },
    };

    let first = query.first();
    if first.is_none() {
        return Err((StatusCode::NOT_FOUND, "Event not found."));
    }

    let first = first.unwrap();

    let event = Event {
        id: Some(first.id),
        series: first.series,
        year: first.year,
        name: first.name.clone(),
        created: first.created,
    };

    let mut return_data: Vec<ImageDoc> = Vec::with_capacity(query.len() / 2);
    for data in query.into_iter() {
        if let Some(prev_entry) = return_data.last_mut() {
            if prev_entry.document.id.is_some_and(|f| f == data.document_id) {
                prev_entry.images.push(Image {
                    id: Some(data.image_id),
                    url: data.image_url,
                    page: data.page,
                    document: data.document_id,
                    created: data.image_created,
                });
                continue;
            }
        }
        let (document, image): (Document, Image) = data.into();
        return_data.push(ImageDoc {
            document,
            images: vec![image],
        });
    }

    let ret = ReturnType {
        event,
        documents: return_data,
    };
    {
        let mut cw = cache.write().unwrap();
        cw.cache.insert((series, year, event_name), ret.clone());
        cw.last_populated = Utc::now();
    }

    return Ok((StatusCode::OK, Json(ret)));
}
