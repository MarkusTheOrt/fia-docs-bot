use std::error::Error;

use serenity::async_trait;

use crate::model::document::Document;

/// # Crawler
/// Trait which defines a stable API for crawling different types of documents.
#[async_trait]
pub trait Crawler {
    type DocType;
    type Parser;
    const DATA_URL: &'static str;

    async fn fetch_data() -> Result<String, Box<dyn Error>>;

    async fn parse_documents() -> Result<Vec<Self::DocType>, Box<dyn Error>>;
}

pub struct F1Crawler;

pub struct F1Parser;
pub struct F2Parser;
pub struct F3Parser;

#[async_trait]
impl Crawler for F1Crawler {
    type DocType = Document;
    type Parser = F1Parser;
    const DATA_URL: &'static str = "https://www.fia.com/documents/championships/fia-formula-one-world-championship-14/season/season-2023-2042";
    async fn fetch_data() -> Result<String, Box<dyn Error>> {
        return Err("Hello".into());
    }
    
    async fn parse_documents() -> Result<Vec<Self::DocType>, Box<dyn Error>> {
        return Err("Hello".into());


    }

}
