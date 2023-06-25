use axum::{Router, Server, routing::get};

use routes::{fallback, home};


mod bodies;
mod model;
mod routes;
mod middleware;

#[tokio::main]
async fn main() {
    let router = Router::new()
        .route("/", get(home))
        .fallback(fallback);

    Server::bind(&"127.0.0.1:1276".parse().unwrap())
        .serve(router.into_make_service())
        .await
        .unwrap();
}
