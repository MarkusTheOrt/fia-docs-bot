use chrono::Utc;
use serde::Serialize;

use crate::model::{document::Document, event::Event};

#[derive(Debug, Serialize)]
pub struct CurrentEventResponse {
    event: Event,
    documents: Vec<Document>,
}

impl CurrentEventResponse {
    pub fn new(title: &str) -> Self {
        Self {
            event: Event {
                id: None,
                series: crate::model::series::Series::F1,
                year: 2023,
                name: title.to_owned(),
                created: Utc::now(),
            },
            documents: vec![],
        }
    }
}
