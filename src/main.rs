use std::{error::Error, time::Duration};

use middleware::magick::check_magick;
use tracing::error;

use crate::middleware::{
    magick::{clear_tmp_dir, create_tmp_dir},
    runner::runner,
};
mod bodies;
mod error;
mod middleware;
mod model;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt::init();

    if !check_magick() {
        error!("Couldn't find imagemagick! exiting...");
    }
    if let Err(why) = create_tmp_dir() {
        error!("Couldn't create tmp dir: {why}");
        std::process::exit(1);
    }
    if let Err(why) = clear_tmp_dir() {
        error!("Couldn't create tmp dir: {why}");
        std::process::exit(1);
    }

    drop(dotenvy::dotenv());

    let database = libsql::Builder::new_remote_replica(
        ":memory:",
        std::env::var("DATABASE_URL").expect("Database URL not set"),
        std::env::var("DATABASE_TOKEN").expect("Database Token not set"),
    )
    .sync_interval(Duration::from_secs(30))
    .build()
    .await?;

    let db_conn = database.connect()?;

    runner(db_conn).await?;

    database.sync().await?;

    Ok(())
}
