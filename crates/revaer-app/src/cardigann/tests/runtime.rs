//! Real imports, transaction rollback, process reconstruction and fenced rule reads.

use std::sync::Arc;
use std::time::Duration;

use revaer_api::app::indexers::IndexerFacade;
use revaer_config::ConfigService;
use revaer_data::indexers::{
    cardigann, definitions, instances, manual_search, search_pages, search_requests,
};
use revaer_telemetry::Metrics;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

use super::DEFINITION;
use crate::indexer_test_support::with_indexer_database;
use crate::indexers::IndexerService;
use crate::manual_search::ManualSearchRuntime;
use crate::runtime_shutdown;

async fn import(config: &Arc<ConfigService>, yaml: &str) -> anyhow::Result<()> {
    IndexerService::new(Arc::clone(config), Metrics::new()?)
        .indexer_definition_import_cardigann(Uuid::nil(), yaml, Some(false))
        .await?;
    Ok(())
}

async fn request(config: &ConfigService) -> anyhow::Result<Uuid> {
    Ok(search_requests::search_request_create(
        config.pool(),
        &search_requests::SearchRequestCreateInput {
            actor_user_public_id: Some(Uuid::nil()),
            query_text: "Cardigann café & + /",
            query_type: "free_text",
            torznab_mode: None,
            requested_media_domain_key: None,
            page_size: None,
            search_profile_public_id: None,
            request_policy_set_public_id: None,
            season_number: None,
            episode_number: None,
            identifier_types: None,
            identifier_values: None,
            torznab_cat_ids: None,
        },
    )
    .await?
    .search_request_public_id)
}

#[tokio::test]
async fn cardigann_rules_and_identity_roll_back_together_and_survive_restart() -> anyhow::Result<()>
{
    with_indexer_database(async |config, database| {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let yaml = DEFINITION.replace("127.0.0.1:8765", &listener.local_addr()?.to_string());
        import(&config, &yaml).await?;
        instances::indexer_instance_create(
            config.pool(),
            Uuid::nil(),
            "public-html",
            "HTML",
            None,
            None,
            None,
        )
        .await?;
        let before = definitions::indexer_definition_list(config.pool(), Uuid::nil())
            .await?
            .into_iter()
            .find(|row| row.upstream_slug == "public-html")
            .ok_or_else(|| anyhow::anyhow!("Definition absent"))?;
        let mut invalid = crate::cardigann::prepare(&yaml)
            .map_err(anyhow::Error::msg)?
            .ok_or_else(|| anyhow::anyhow!("Rules absent"))?;
        invalid.category_targets[0] = i32::MAX;
        let mut transaction = config.pool().begin().await?;
        definitions::indexer_definition_import_cardigann_begin(
            &mut transaction,
            Uuid::nil(),
            "public-html",
            "Replacement",
            "replacement identity",
            false,
        )
        .await?;
        anyhow::ensure!(
            cardigann::import(&mut transaction, Uuid::nil(), "public-html", &invalid)
                .await
                .is_err()
        );
        transaction.rollback().await?;
        let after = definitions::indexer_definition_list(config.pool(), Uuid::nil())
            .await?
            .into_iter()
            .find(|row| row.upstream_slug == "public-html")
            .ok_or_else(|| anyhow::anyhow!("Definition disappeared"))?;
        anyhow::ensure!(before.definition_hash == after.definition_hash);
        config.pool().close().await;
        // Reconstruct the application pool; nothing retains the parsed definition in memory.
        let restarted = Arc::new(ConfigService::new(database.connection_string()).await?);
        let observed = restarted_search(&restarted, listener).await;
        restarted.pool().close().await;
        observed
    })
    .await
}

async fn restarted_search(
    config: &Arc<ConfigService>,
    listener: TcpListener,
) -> anyhow::Result<()> {
    let request_id = request(config).await?;
    let (shutdown, receiver) = runtime_shutdown::channel();
    let worker =
        ManualSearchRuntime::new(Arc::clone(config), crate::bootstrap::manual_search_client)
            .spawn(receiver.clone());
    let mut server_shutdown = receiver;
    let upstream = tokio::spawn(async move {
        let (mut stream, _) = tokio::select! {
            accepted = listener.accept() => accepted?,
            () = runtime_shutdown::changed(&mut server_shutdown) => return Err(anyhow::anyhow!("No HTML request")),
        };
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let byte = tokio::select! {
                read = stream.read_u8() => read?,
                () = runtime_shutdown::changed(&mut server_shutdown) => return Err(anyhow::anyhow!("HTTP canceled")),
            };
            anyhow::ensure!(header.len() < 16384);
            header.push(byte);
        }
        let header = String::from_utf8(header)?;
        anyhow::ensure!(header.starts_with(
            "GET /search/releases?order=newest&term=Cardigann+caf%C3%A9+%26+%2B+%2F HTTP/1.1"
        ));
        let body = "<!doctype html><table class=releases><tr class=release><td><a class=title href=/details/1>Persisted HTML</a><a class=magnet href='magnet:?xt=urn:btih:0123456789012345678901234567890123456789'>Magnet</a><td class=category>film<td class=bytes>1024<td class=seeders>17<td class=leechers>2<td><time datetime='2026-10-24T00:00:00Z'></table>";
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            )
            .await?;
        Ok::<(), anyhow::Error>(())
    });
    let observed = tokio::time::timeout(Duration::from_secs(10), async {
        while !search_requests::search_request_is_terminal(config.pool(), Uuid::nil(), request_id)
            .await?
        {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let rows =
            search_pages::search_page_fetch(config.pool(), Uuid::nil(), request_id, 1).await?;
        anyhow::ensure!(
            rows.len() == 1 && rows[0].title_display.as_deref() == Some("Persisted HTML")
        );
        anyhow::ensure!(rows[0].tracker_category == Some(2000));
        Ok::<(), anyhow::Error>(())
    })
    .await;
    let _ = runtime_shutdown::request(&shutdown);
    let (worker, upstream) = tokio::join!(worker, upstream);
    worker??;
    upstream??;
    observed??;
    Ok(())
}

#[tokio::test]
async fn cardigann_metadata_only_import_requires_reimport_and_fences_stale_claims()
-> anyhow::Result<()> {
    with_indexer_database(async |config, _| {
        import(&config, DEFINITION).await?;
        instances::indexer_instance_create(
            config.pool(),
            Uuid::nil(),
            "public-html",
            "HTML",
            None,
            None,
            None,
        )
        .await?;
        let request_id = request(&config).await?;
        let run = manual_search::claim_next(config.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Claim absent"))?;
        anyhow::ensure!(
            run.connection
                .as_ref()
                .map_err(|error| *error)?
                .api_key
                .is_none()
        );
        anyhow::ensure!(cardigann::load(config.pool(), &run).await?.is_some());
        search_requests::search_request_cancel(config.pool(), Uuid::nil(), request_id).await?;
        anyhow::ensure!(cardigann::load(config.pool(), &run).await?.is_none());
        import(&config, "id: public-html\nname: Legacy\nsettings: []").await?;
        request(&config).await?;
        let run = manual_search::claim_next(config.pool())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Legacy claim absent"))?;
        let error = run
            .connection
            .as_ref()
            .err()
            .ok_or_else(|| anyhow::anyhow!("Legacy rules must not run"))?;
        anyhow::ensure!(error.action().contains("eimport"));
        anyhow::ensure!(cardigann::load(config.pool(), &run).await?.is_none());
        Ok(())
    })
    .await
}
