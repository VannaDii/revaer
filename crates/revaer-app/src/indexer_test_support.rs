//! Owned database boundary shared by concrete indexer runtime regressions.

use revaer_config::{ConfigService, DbSessionConfig};
use revaer_test_support::postgres::{TestDatabase, start_postgres};
use std::sync::Arc;
use uuid::Uuid;

pub(crate) async fn with_indexer_database<F>(exercise: F) -> anyhow::Result<()>
where
    F: AsyncFnOnce(Arc<ConfigService>, &TestDatabase) -> anyhow::Result<()>,
{
    let mut database = start_postgres()?;
    let result = async {
        database
            .initialize_runtime(include_str!("../../revaer-data/init.sql"))
            .await?;
        let config = Arc::new(
            ConfigService::new_with_session(
                database.connection_string(),
                Some(DbSessionConfig::new(
                    "indexer-execution-test",
                    &Uuid::new_v4().to_string(),
                )),
            )
            .await?,
        );
        let result = exercise(Arc::clone(&config), &database).await;
        config.pool().close().await;
        result
    }
    .await;
    match (result, database.close()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(operation), Err(cleanup)) => Err(anyhow::anyhow!(
            "Indexer test failed: {operation}; database cleanup failed: {cleanup}"
        )),
    }
}
