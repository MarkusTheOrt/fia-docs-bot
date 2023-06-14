use std::{error::Error as StdError, fmt::{Display, self}};

use chrono::Utc;
use html5ever::{tokenizer::{TokenSink, Tokenizer, TokenizerOpts, BufferQueue}, tendril::*};
use reqwest::StatusCode;
use serenity::async_trait;

use crate::model::document::Document;

/// # Crawler
/// Trait which defines a stable API for crawling different types of documents.
#[async_trait]
pub trait Crawler {
    type DocType;
    const DATA_URL: &'static str;

    async fn fetch_data(&self) -> Result<String, CrawlerErr>;

    async fn parse_documents(&self) -> Result<Vec<Self::DocType>, CrawlerErr>;
}

pub struct F1Crawler;

#[derive(Default, Debug)]
struct TempDoc {
    url: Option<String>,
    title: Option<String>,
    date: Option<Utc>,
}

/// Parses documents from https://fia.com/documents
pub struct FIAParser<'a> {
    pub documents: &'a mut Vec<Document>,
    current_doc: TempDoc,
}

#[derive(Debug)]
pub struct Error {
    pub inner: Box<Inner>
}

#[derive(Debug, Clone, Copy)]
pub enum Kind {
    Http,
    Parser
}

#[derive(Debug)]
pub struct Inner {
    pub kind: Kind,
    pub source: Option<Box<dyn StdError + Send + Sync>>
}

impl Error {
    pub(crate) fn new<E>(kind: Kind, source: Option<E>) -> Self 
    where E: Into<Box<dyn StdError + Send + Sync>> {
        return Self {
            inner: Box::new(Inner {
                kind,
                source: source.map(Into::into)
            })
        }
    }
}

impl Error {
    pub fn kind(&self) -> Kind {
        return self.inner.kind;
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            Kind::Http => f.write_str("HTTP Error"),
            Kind::Parser => f.write_str("Parser Error")
        }
    }
}

#[derive(Debug)]
pub enum CrawlerErr {
    Reqwest(reqwest::Error),
    Internal(Error)
}


impl Display for CrawlerErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reqwest(inner) => {
                return fmt::Display::fmt(&inner, f)
            },
            Self::Internal(inner) => {
                return fmt::Display::fmt(&inner, f)
            }
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        return self.inner.source.as_ref().map(|f| &**f as _);
    }
}

impl StdError for CrawlerErr {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Reqwest(inner) => Some(inner),
            Self::Internal(inner) => Some(inner)
        }
    }
}

impl From<reqwest::Error> for CrawlerErr {
    fn from(e: reqwest::Error) -> CrawlerErr {
        return CrawlerErr::Reqwest(e);
    }
}

impl From<Error> for CrawlerErr {
    fn from(value: Error) -> Self {
        return CrawlerErr::Internal(value);
    }
}

#[async_trait]
impl Crawler for F1Crawler {
    type DocType = Document;
    const DATA_URL: &'static str = "https://www.fia.com/documents/championships/fia-formula-one-world-championship-14/season/season-2023-2042";
    async fn fetch_data(&self) -> Result<String, CrawlerErr> {
        let request = reqwest::get(Self::DATA_URL).await?;
            
        if request.status() != StatusCode::OK {
            return Err(Error::new(Kind::Http, "StatusCode Not 200".into()).into())
        }

        return match request.text().await {
            Ok(data) => Ok(data),
            Err(why) => Err(CrawlerErr::Reqwest(why))
        }
    }

    async fn parse_documents(&self) -> Result<Vec<Self::DocType>, CrawlerErr> {
        let data = self.fetch_data().await?;
        // Most Events do not create more than 60 documents, so this should be
        // speedy and fine!
        let mut documents = Vec::with_capacity(60);

        let parser = FIAParser {
            documents: &mut documents,
            current_doc: TempDoc::default(),
        };
        
        let mut tendril = ByteTendril::new();
        data.as_bytes().read_to_tendril(&mut tendril).unwrap();
        let mut queue = BufferQueue::new();
        queue.push_back(tendril.try_reinterpret().unwrap());
        let mut tok = Tokenizer::new(parser, TokenizerOpts::default());
        let _ = tok.feed(&mut queue);
        tok.end();
        return Ok(documents);
    }
}

impl<'a> TokenSink for FIAParser<'a> {
    type Handle = ();

    fn process_token(
        &mut self,
        token: html5ever::tokenizer::Token,
        _line_number: u64,
    ) -> html5ever::tokenizer::TokenSinkResult<Self::Handle> {
        todo!()
    }
}
