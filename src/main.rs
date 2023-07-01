use axum::{routing::get, Router, Server};

use middleware::magick::check_magick;
use routes::{
    event::events, fallback, home, season::season, series::series,
    series_current,
};
use sqlx::{mysql::MySqlPoolOptions, MySql, Pool};

use crate::middleware::{
    magick::{clear_tmp_dir, create_tmp_dir},
    runner::runner,
};
mod bodies;
mod middleware;
mod model;
mod routes;

#[tokio::main]
async fn run_main(database: Pool<MySql>) {
    runner(&database).await;
}

#[tokio::main]
async fn main() {
    if !check_magick() {
        eprintln!("Couldn't find imagemagick! exiting...");
        std::process::exit(1);
    }
    if let Err(why) = create_tmp_dir() {
        eprintln!("Couldn't create tmp dir: {why}");
        std::process::exit(1);
    }
    if let Err(why) = clear_tmp_dir() {
        eprintln!("Couldn't create tmp dir: {why}");
        std::process::exit(1);
    }

    drop(dotenvy::dotenv());
    let database_connect =
        std::env::var("DATABASE_URL").expect("Database URL not set.");

    let database = MySqlPoolOptions::new()
        .max_connections(1000)
        .connect_lazy(&database_connect)
        .expect("Database Connection failed");

    drop(database_connect);

    let database_1 = database.clone();

    std::thread::spawn(|| {
        run_main(database_1);
    });

    println!("starting application...");
    let router = Router::new()
        .route("/", get(home))
        .route("/:series/current", get(series_current))
        .route("/:series/current/", get(series_current))
        .route("/:series", get(series))
        .route("/:series/", get(series))
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
