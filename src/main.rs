use axum::{routing::get, Router, Server};

use routes::{current, fallback, home, season::season, event::events, series_current, series::series};
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
