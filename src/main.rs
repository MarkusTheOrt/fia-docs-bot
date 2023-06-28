use std::num::NonZeroI16;

use axum::{routing::get, Router, Server};

use chrono::Utc;
use html5ever::{
    tendril::{ByteTendril, ReadExt},
    tokenizer::{BufferQueue, Tokenizer, TokenizerOpts},
};
use middleware::parser::HTMLParser;
use routes::{
    current, event::events, fallback, home, season::season, series::series,
    series_current,
};
use sqlx::mysql::MySqlPoolOptions;

mod bodies;
mod middleware;
mod model;
mod routes;

#[tokio::main]
async fn main() {
    drop(dotenvy::dotenv());
    let database_connect =
        std::env::var("DATABASE_URL").expect("Database URL not set.");

    let database = MySqlPoolOptions::new()
        .max_connections(1000)
        .connect_lazy(&database_connect)
        .expect("Database Connection failed");

    drop(database_connect);

    let _thread = tokio::spawn(async move {
        let test = reqwest::get("https://www.fia.com/documents/championships/fia-formula-one-world-championship-14/season/season-2023-2042?nocache").await.expect("fia.com");

        let bytes = test.text().await.expect("stringy");

        let mut tendril = ByteTendril::new();
        let _ = bytes.as_bytes().read_to_tendril(&mut tendril);
        let mut input = BufferQueue::new();
        input.push_back(tendril.try_reinterpret().unwrap());
        let mut parser_season = middleware::parser::Season {
            year: NonZeroI16::new(2023).unwrap(),
            events: vec![],
        };
        let sink = HTMLParser::new(&mut parser_season);
        let opts = TokenizerOpts::default();
        let mut tok = Tokenizer::new(sink, opts);
        let _ = tok.feed(&mut input);
        tok.end();
    });

    let router = Router::new()
        .route("/", get(home))
        .route("/current", get(current))
        .route("/current/", get(current))
        .route("/:series/current", get(series_current))
        .route("/:series/current/", get(series_current))
        .route("/:series", get(series))
        .route("/:series/", get(series))
        .route("/:series/events/", get(series))
        .route("/:series/events", get(series))
        .route("/:series/:year/", get(season))
        .route("/:series/:year", get(season))
        .route("/:series/:year/:event", get(events))
        .route("/:series/:year/:event/", get(events))
        .with_state(database)
        .fallback(fallback);

    Server::bind(&"127.0.0.1:1276".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}
