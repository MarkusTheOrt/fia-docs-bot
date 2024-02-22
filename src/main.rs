use middleware::magick::check_magick;
use sqlx::mysql::MySqlPoolOptions;

use crate::middleware::{
    magick::{clear_tmp_dir, create_tmp_dir},
    runner::runner,
};
mod bodies;
mod middleware;
mod model;

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
        .connect_lazy(&database_connect)
        .expect("Database Connection failed");

    drop(database_connect);

    runner(&database).await;
}
