//! Thin bootstrap command for the unreleased v0 database initializer.

use std::error::Error;
use std::process::ExitCode;

use revaer_data::config::initialize_schema;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("database initialization failed: {error}");
            let mut source = error.source();
            while let Some(cause) = source {
                eprintln!("caused by: {cause}");
                source = cause.source();
            }
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    initialize_schema(&pool).await?;
    pool.close().await;
    Ok(())
}
