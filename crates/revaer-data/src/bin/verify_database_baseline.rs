#![forbid(unsafe_code)]
#![deny(warnings, clippy::all, clippy::pedantic)]

//! Read-only command for the canonical gate's explicitly initialized database.

use std::{process::ExitCode, str::FromStr, time::Duration};

use revaer_data::baseline::verify_runtime_pool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

#[tokio::main]
async fn main() -> ExitCode {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("Database baseline verification requires DATABASE_URL.");
        return ExitCode::FAILURE;
    };
    let Ok(options) = PgConnectOptions::from_str(&url) else {
        eprintln!("Database baseline connection configuration is invalid.");
        return ExitCode::FAILURE;
    };
    let options = options.options([("statement_timeout", "10000")]);
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(10))
        .connect_lazy_with(options);
    let result = verify_runtime_pool(&pool).await;
    pool.close().await;
    match result {
        Ok(_) => {
            println!("Packaged database baseline and runtime identity verified.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Database baseline verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}
