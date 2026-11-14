use super::*;
use crate::cardigann::tests::DEFINITION;
use crate::indexers::IndexerService;
use revaer_api::app::indexers::IndexerFacade;
use revaer_telemetry::Metrics;

async fn configure_html(
    config: &Arc<ConfigService>,
    database: &TestDatabase,
    endpoint: &str,
) -> anyhow::Result<Uuid> {
    let yaml = DEFINITION.replace("http://127.0.0.1:8765", endpoint);
    IndexerService::new(Arc::clone(config), Metrics::new()?)
        .indexer_definition_import_cardigann(Uuid::nil(), &yaml, Some(false))
        .await?;
    let source = instances::indexer_instance_create(
        config.pool(),
        Uuid::nil(),
        "public-html",
        "Scheduled HTML",
        None,
        None,
        None,
    )
    .await?;
    instances::rss_subscription_set(config.pool(), Uuid::nil(), source, true, Some(300)).await?;
    database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/rss-poll-due.sql"),
            &[("revaer_test.rss_source", &source.to_string())],
        )
        .await?;
    Ok(source)
}

#[tokio::test]
async fn cardigann_rss_rules_survive_reconstruction_and_fence_revision_and_correlation()
-> anyhow::Result<()> {
    with_indexer_database(async |config, database| {
        let source = configure_html(&config, database, "http://127.0.0.1:8765").await?;
        let mut claim = executor::rss_poll_claim(config.pool(), 1)
            .await?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("Expected RSS claim"))?;
        let correlation = claim.correlation_id;
        let config = Arc::new(ConfigService::new(database.connection_string()).await?);
        let result = async {
            let plan = cardigann::load_rss(config.pool(), &claim)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Persisted RSS plan absent"))?;
            assert_eq!(plan.search_path, "search/releases");
            claim.correlation_id = Uuid::new_v4();
            assert!(cardigann::load_rss(config.pool(), &claim).await?.is_none());
            claim.correlation_id = correlation;
            instances::rss_subscription_set(config.pool(), Uuid::nil(), source, false, None)
                .await?;
            assert!(cardigann::load_rss(config.pool(), &claim).await?.is_none());
            instances::rss_subscription_set(config.pool(), Uuid::nil(), source, true, Some(300))
                .await?;
            assert!(cardigann::load_rss(config.pool(), &claim).await?.is_none());
            Ok::<(), anyhow::Error>(())
        }
        .await;
        config.pool().close().await;
        result
    })
    .await
}

#[tokio::test]
async fn cardigann_rss_metadata_only_rules_fail_before_http_with_reimport_guidance()
-> anyhow::Result<()> {
    with_indexer_database(async |config, database| {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let source = configure_html(
            &config,
            database,
            &format!("http://{}", listener.local_addr()?),
        )
        .await?;
        IndexerService::new(Arc::clone(&config), Metrics::new()?)
            .indexer_definition_import_cardigann(
                Uuid::nil(),
                "id: public-html\nname: Legacy\nsettings: []",
                Some(false),
            )
            .await?;
        let claim = executor::rss_poll_claim(config.pool(), 1)
            .await?
            .pop()
            .ok_or_else(|| anyhow::anyhow!("Expected legacy RSS claim"))?;
        let context = rss_execution::resolve(
            config.pool(),
            claim.rss_subscription_id,
            claim.correlation_id,
        )
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected owned context"))?;
        let (_stop, mut receiver) = runtime_shutdown::channel();
        let runtime = RssPollRuntime::new(Arc::clone(&config), crate::bootstrap::torznab_client);
        let error = runtime
            .fetch(&context, &claim, None, &mut receiver)
            .await
            .err()
            .ok_or_else(|| anyhow::anyhow!("Missing rules must fail"))?;
        assert_eq!(error.diagnostic, Some(crate::cardigann::REIMPORT_REQUIRED));
        runtime.execute(&claim, &mut receiver).await?;
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), listener.accept())
                .await
                .is_err()
        );
        assert_eq!(
            rss::rss_subscription_get(config.pool(), Uuid::nil(), source)
                .await?
                .last_error_class
                .as_deref(),
            Some("parse_error")
        );
        Ok(())
    })
    .await
}

#[tokio::test]
async fn cardigann_rss_timeout_and_shutdown_settle_owned_writes() -> anyhow::Result<()> {
    for shutdown in [false, true] {
        with_indexer_database(async |config, database| {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
            let source = configure_html(
                &config,
                database,
                &format!("http://{}", listener.local_addr()?),
            )
            .await?;
            let (stop, mut receiver) = runtime_shutdown::channel();
            let worker_config = Arc::clone(&config);
            let worker = tokio::spawn(async move {
                RssPollRuntime::new(worker_config, crate::bootstrap::torznab_client)
                    .run_batch(&mut receiver)
                    .await
            });
            let accepted =
                tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept()).await;
            if shutdown {
                let _ = runtime_shutdown::request(&stop);
            }
            worker.await??;
            let (_socket, _) = accepted??;
            let status = rss::rss_subscription_get(config.pool(), Uuid::nil(), source).await?;
            assert_eq!(status.last_error_class.as_deref(), Some("timeout"));
            assert_eq!(status.backoff_seconds, Some(60));
            assert!(status.next_poll_at.is_some());
            assert!(executor::rss_poll_claim(config.pool(), 1).await?.is_empty());
            Ok(())
        })
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn cardigann_rss_recovered_claim_ingests_once_without_search_or_acquisition()
-> anyhow::Result<()> {
    with_indexer_database(async |config, database| {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let source = configure_html(&config, database, &format!("http://{}", listener.local_addr()?)).await?;
        let old = executor::rss_poll_claim(config.pool(), 1).await?.pop()
            .ok_or_else(|| anyhow::anyhow!("Expected RSS claim"))?;
        database.apply_fixture_script(include_str!("../../../../../scripts/tests/rss-poll-expire.sql"),
            &[("revaer_test.rss_source", &source.to_string())]).await?;
        assert!(cardigann::load_rss(config.pool(), &old).await?.is_none());
        let current = executor::rss_poll_claim(config.pool(), 1).await?.pop()
            .ok_or_else(|| anyhow::anyhow!("Expected recovered RSS claim"))?;
        assert_ne!(old.correlation_id, current.correlation_id);
        assert!(cardigann::load_rss(config.pool(), &current).await?.is_some());
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut headers = [0; 4096];
            let length = stream.read(&mut headers).await?;
            let request = std::str::from_utf8(&headers[..length])?;
            anyhow::ensure!(request.starts_with("GET /search/releases?order=newest HTTP/1.1"));
            let row = "<tr class=release><td><a class=title href='/details/1'>Scheduled</a><a class=magnet href='magnet:?xt=urn:btih:0123456789012345678901234567890123456789'>magnet</a></td><td class=category>film</td><td class=bytes>1024</td><td class=seeders>1</td><td class=leechers>0</td><td><time datetime='2026-10-24T00:00:00Z'></time></td></tr>";
            let body = format!("<table class=releases>{row}{row}</table>");
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
            Ok::<(), anyhow::Error>(())
        });
        let (_stop, mut receiver) = runtime_shutdown::channel();
        let runtime = RssPollRuntime::new(Arc::clone(&config), crate::bootstrap::torznab_client);
        runtime.execute(&old, &mut receiver).await?;
        let executed = runtime.execute(&current, &mut receiver).await;
        server.await??;
        executed?;
        let before = rss::rss_item_seen_list(config.pool(), Uuid::nil(), source, Some(10)).await?;
        assert_eq!(before.len(), 1);
        runtime.execute(&current, &mut receiver).await?;
        let after = rss::rss_item_seen_list(config.pool(), Uuid::nil(), source, Some(10)).await?;
        assert_eq!(after.len(), 1);
        assert_eq!(before[0].first_seen_at, after[0].first_seen_at);
        database.apply_fixture_script(include_str!("../../../../../scripts/tests/rss-cardigann-assert-isolation.sql"),
            &[("revaer_test.rss_source", &source.to_string())]).await?;
        Ok(())
    }).await
}

#[tokio::test]
async fn cardigann_rss_budget_denials_prevent_http() -> anyhow::Result<()> {
    for scope in ["routing_policy", "indexer_instance"] {
        with_indexer_database(async |config, database| {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
            let source = configure_html(
                &config,
                database,
                &format!("http://{}", listener.local_addr()?),
            )
            .await?;
            let identity = source.to_string();
            let parameters = [
                ("revaer_test.rss_source", identity.as_str()),
                ("revaer_test.rss_denied_scope", scope),
            ];
            database
                .apply_fixture_script(
                    include_str!("../../../../../scripts/tests/rss-poll-denied.sql"),
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
            assert_eq!(
                rss::rss_subscription_get(config.pool(), Uuid::nil(), source)
                    .await?
                    .last_error_class
                    .as_deref(),
                Some("rate_limited")
            );
            database
                .apply_fixture_script(
                    include_str!("../../../../../scripts/tests/rss-poll-assert-rate.sql"),
                    &parameters,
                )
                .await?;
            Ok(())
        })
        .await?;
    }
    Ok(())
}

#[tokio::test]
async fn cardigann_rss_proxy_failure_never_falls_back_to_origin() -> anyhow::Result<()> {
    use revaer_data::indexers::routing;
    with_indexer_database(async |config, database| {
        let origin = TcpListener::bind(("127.0.0.1", 0)).await?;
        let proxy = TcpListener::bind(("127.0.0.1", 0)).await?;
        let port = i32::from(proxy.local_addr()?.port());
        drop(proxy);
        let source = configure_html(
            &config,
            database,
            &format!("http://{}", origin.local_addr()?),
        )
        .await?;
        let route = routing::routing_policy_create(
            config.pool(),
            Uuid::nil(),
            "Unavailable RSS proxy",
            "http_proxy",
        )
        .await?;
        routing::routing_policy_set_param(
            config.pool(),
            Uuid::nil(),
            route,
            "proxy_host",
            Some("127.0.0.1"),
            None,
            None,
        )
        .await?;
        routing::routing_policy_set_param(
            config.pool(),
            Uuid::nil(),
            route,
            "proxy_port",
            None,
            Some(port),
            None,
        )
        .await?;
        instances::indexer_instance_update(
            config.pool(),
            &instances::IndexerInstanceUpdateInput {
                actor_user_public_id: Uuid::nil(),
                indexer_instance_public_id: source,
                display_name: None,
                priority: None,
                trust_tier_key: None,
                routing_policy_public_id: Some(route),
                is_enabled: None,
                enable_rss: None,
                enable_automatic_search: None,
                enable_interactive_search: None,
            },
        )
        .await?;
        let (_stop, mut receiver) = runtime_shutdown::channel();
        RssPollRuntime::new(Arc::clone(&config), crate::bootstrap::torznab_client)
            .run_batch(&mut receiver)
            .await?;
        let status = rss::rss_subscription_get(config.pool(), Uuid::nil(), source).await?;
        assert_eq!(
            status.last_error_class.as_deref(),
            Some("connection_refused")
        );
        assert_eq!(status.backoff_seconds, Some(60));
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(50), origin.accept())
                .await
                .is_err()
        );
        Ok(())
    })
    .await
}
