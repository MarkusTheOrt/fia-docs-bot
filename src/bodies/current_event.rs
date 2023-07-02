use serde::Serialize;

use crate::model::{document::Document, event::Event};

#[derive(Debug, Serialize)]
pub struct CurrentEventResponse {
    event: Event,
    documents: Vec<Document>,
}
