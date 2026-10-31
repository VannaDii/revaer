//! Exercise the actual RSS executor with restricted `PostgreSQL` and loopback HTTP.

use super::*;
use crate::indexer_test_support::with_indexer_database;
use revaer_data::indexers::{instances, rss, secrets};
use revaer_test_support::postgres::TestDatabase;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

mod html_tests;

async fn configure(
    config: &ConfigService,
    database: &TestDatabase,
    endpoint: &str,
) -> anyhow::Result<Uuid> {
    let actor = Uuid::nil();
    let source = instances::indexer_instance_create(
        config.pool(),
        actor,
        "torznab",
        &format!("RSS execution {}", Uuid::new_v4()),
        None,
        None,
        None,
    )
    .await?;
    instances::indexer_instance_field_set_value(
        config.pool(),
        &instances::IndexerInstanceFieldValueInput {
            actor_user_public_id: actor,
            indexer_instance_public_id: source,
            field_name: "base_url",
            value_plain: Some(endpoint),
            value_int: None,
            value_decimal: None,
            value_bool: None,
        },
    )
    .await?;
    let key = Uuid::new_v4().to_string();
    let secret = secrets::secret_create(config.pool(), actor, "api_key", &key).await?;
    instances::indexer_instance_field_bind_secret(config.pool(), actor, source, "api_key", secret)
        .await?;
    instances::rss_subscription_set(config.pool(), actor, source, true, Some(300)).await?;
    database
        .apply_fixture_script(
            include_str!("../../../../scripts/tests/rss-poll-due.sql"),
            &[("revaer_test.rss_source", &source.to_string())],
        )
        .await?;
    Ok(source)
}

#[tokio::test]
async fn rss_transport_timeout_and_connection_failure_schedule_backoff() -> anyhow::Result<()> {
    with_indexer_database(async |config, database| {
        for pending in [true, false] {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
            let address = listener.local_addr()?;
            let source = configure(&config, database, &format!("http://{address}/api")).await?;
            let (stop, mut receiver) = runtime_shutdown::channel();
            let server = if pending {
                let mut shutdown = receiver.clone();
                Some(tokio::spawn(async move {
                    let (_stream, _) = tokio::select! {
                        accepted = listener.accept() => accepted?,
                        () = runtime_shutdown::changed(&mut shutdown) => return Ok(()),
                    };
                    runtime_shutdown::changed(&mut shutdown).await;
                    Ok::<(), std::io::Error>(())
                }))
            } else {
                drop(listener);
                None
            };
            let result = RssPollRuntime::new(Arc::clone(&config), crate::bootstrap::torznab_client)
                .run_batch(&mut receiver)
                .await;
            let _ = runtime_shutdown::request(&stop);
            if let Some(server) = server {
                server.await??;
            }
            result?;
            let status = rss::rss_subscription_get(config.pool(), Uuid::nil(), source).await?;
            assert_eq!(
                status.last_error_class.as_deref(),
                Some(if pending {
                    "timeout"
                } else {
                    "connection_refused"
                })
            );
            assert_eq!(status.backoff_seconds, Some(60));
            assert!(status.subscription_is_enabled && status.next_poll_at.is_some());
        }
        Ok(())
    })
    .await
}

#[tokio::test]
async fn rss_shutdown_settles_pending_request_before_pool_close() -> anyhow::Result<()> {
    with_indexer_database(async |config, database| {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let source = configure(
            &config,
            database,
            &format!("http://{}/api", listener.local_addr()?),
        )
        .await?;
        let (stop, receiver) = runtime_shutdown::channel();
        let worker_config = Arc::clone(&config);
        let mut worker_shutdown = receiver.clone();
        let worker = tokio::spawn(async move {
            RssPollRuntime::new(worker_config, crate::bootstrap::torznab_client)
                .run_batch(&mut worker_shutdown)
                .await
        });
        let observed =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept()).await;
        let _ = runtime_shutdown::request(&stop);
        worker.await??;
        let (_stream, _) = observed??;
        let status = rss::rss_subscription_get(config.pool(), Uuid::nil(), source).await?;
        assert_eq!(status.last_error_class.as_deref(), Some("timeout"));
        assert!(status.subscription_is_enabled && status.next_poll_at.is_some());
        assert!(executor::rss_poll_claim(config.pool(), 1).await?.is_empty());
        Ok(())
    })
    .await
}

#[tokio::test]
async fn rss_runtime_parsed_eligible_and_inserted_counts_are_distinct() -> anyhow::Result<()> {
    with_indexer_database(async |config, database| {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let source = configure(&config, database, &format!("http://{}/api", listener.local_addr()?)).await?;
        let body = "<rss><channel><item><guid>first</guid></item><item><guid>first</guid></item><item/></channel></rss>";
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut headers = [0; 4096];
            let received = stream.read(&mut headers).await?;
            anyhow::ensure!(received > 0, "Expected an HTTP request");
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
            Ok::<(), anyhow::Error>(())
        });
        let (_stop, mut receiver) = runtime_shutdown::channel();
        let result = RssPollRuntime::new(Arc::clone(&config), crate::bootstrap::torznab_client).run_batch(&mut receiver).await;
        server.await??;
        result?;
        let status = rss::rss_subscription_get(config.pool(), Uuid::nil(), source).await?;
        assert!(status.last_polled_at.is_some());
        let seen = rss::rss_item_seen_list(config.pool(), Uuid::nil(), source, Some(10)).await?;
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].item_guid.as_deref(), Some("first"));
        Ok(())
    }).await
}

#[tokio::test]
async fn rss_budget_denials_do_not_send_http_and_retain_scope() -> anyhow::Result<()> {
    for scope in ["routing_policy", "indexer_instance"] {
        with_indexer_database(async |config, database| {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
            let source = configure(
                &config,
                database,
                &format!("http://{}/api", listener.local_addr()?),
            )
            .await?;
            let identity = source.to_string();
            let parameters = [
                ("revaer_test.rss_source", identity.as_str()),
                ("revaer_test.rss_denied_scope", scope),
            ];
            database
                .apply_fixture_script(
                    include_str!("../../../../scripts/tests/rss-poll-denied.sql"),
                    &parameters,
                )
                .await?;
            let (_stop, mut receiver) = runtime_shutdown::channel();
            RssPollRuntime::new(Arc::clone(&config), crate::bootstrap::torznab_client)
                .run_batch(&mut receiver)
                .await?;
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept())
                    .await
                    .is_err()
            );
            let status = rss::rss_subscription_get(config.pool(), Uuid::nil(), source).await?;
            assert_eq!(status.last_error_class.as_deref(), Some("rate_limited"));
            assert_eq!(status.backoff_seconds, Some(60));
            database
                .apply_fixture_script(
                    include_str!("../../../../scripts/tests/rss-poll-assert-rate.sql"),
                    &parameters,
                )
                .await?;
            Ok(())
        })
        .await?;
    }
    Ok(())
}
