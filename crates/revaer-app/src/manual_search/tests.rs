//! The production dispatcher owns real `PostgreSQL` work and a real pending HTTP request.

use super::*;
use crate::indexer_test_support::with_indexer_database;
use revaer_data::indexers::{instances, search_pages, search_requests, secrets};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use uuid::Uuid;

#[tokio::test]
async fn manual_search_shutdown_settles_pending_http_and_owned_run() -> anyhow::Result<()> {
    pending_attempt_settles(false).await
}

#[tokio::test]
async fn manual_search_cancellation_stops_pending_http_and_fences_results() -> anyhow::Result<()> {
    pending_attempt_settles(true).await
}

async fn pending_attempt_settles(cancel_request: bool) -> anyhow::Result<()> {
    with_indexer_database(async move |config, _database| {
        exercise_pending_attempt(config, cancel_request).await
    })
    .await
}

async fn exercise_pending_attempt(
    config: Arc<ConfigService>,
    cancel_request: bool,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let endpoint = format!("http://{}/api", listener.local_addr()?);
    let request_id = configured_request(&config, &endpoint).await?;
    let (shutdown, receiver) = runtime_shutdown::channel();
    let (seen_sender, seen_receiver) = oneshot::channel();
    let (closed_sender, closed_receiver) = oneshot::channel();
    let server = tokio::spawn(pending_upstream(
        listener,
        receiver.clone(),
        seen_sender,
        closed_sender,
    ));
    let worker =
        ManualSearchRuntime::new(Arc::clone(&config), crate::bootstrap::manual_search_client)
            .spawn(receiver);
    let observed = async {
        tokio::time::timeout(Duration::from_secs(5), seen_receiver).await??;
        if cancel_request {
            search_requests::search_request_cancel(config.pool(), Uuid::nil(), request_id).await?;
            let client_closed =
                tokio::time::timeout(Duration::from_secs(5), closed_receiver).await??;
            anyhow::ensure!(
                client_closed,
                "Cancellation must stop the pending HTTP request"
            );
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    let _ = runtime_shutdown::request(&shutdown);
    // Always join both owners before closing their pool, including assertion failures.
    let (worker_result, server_result) = tokio::join!(worker, server);
    match (observed, worker_result, server_result) {
        (Ok(()), Ok(Ok(())), Ok(Ok(()))) => {}
        (observation, worker, upstream) => {
            return Err(anyhow::anyhow!(
                "Pending attempt observation: {observation:?}; worker settlement: {worker:?}; upstream settlement: {upstream:?}"
            ));
        }
    }
    let explain =
        search_pages::search_request_explainability(config.pool(), Uuid::nil(), request_id).await?;
    anyhow::ensure!(explain.skipped_canceled_indexers == 1);
    anyhow::ensure!(store::claim_next(config.pool()).await?.is_none());
    Ok(())
}

async fn configured_request(config: &ConfigService, endpoint: &str) -> anyhow::Result<Uuid> {
    let actor = Uuid::nil();
    let source = instances::indexer_instance_create(
        config.pool(),
        actor,
        "torznab",
        "Pending upstream",
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
    let credential = Uuid::new_v4().to_string();
    let secret = secrets::secret_create(config.pool(), actor, "api_key", &credential).await?;
    instances::indexer_instance_field_bind_secret(config.pool(), actor, source, "api_key", secret)
        .await?;
    let request = search_requests::search_request_create(
        config.pool(),
        &search_requests::SearchRequestCreateInput {
            actor_user_public_id: Some(actor),
            query_text: "Pending upstream search",
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
    .await?;
    Ok(request.search_request_public_id)
}

async fn pending_upstream(
    listener: TcpListener,
    mut shutdown: RuntimeShutdownReceiver,
    seen: oneshot::Sender<()>,
    closed: oneshot::Sender<bool>,
) -> anyhow::Result<()> {
    let (mut stream, _) = tokio::select! {
        result = listener.accept() => result?,
        () = runtime_shutdown::changed(&mut shutdown) => return Ok(()),
    };
    if !receive_headers(&mut stream, &mut shutdown).await? {
        return Ok(());
    }
    seen.send(())
        .map_err(|()| anyhow::anyhow!("Request observer closed"))?;
    let mut byte = [0_u8];
    let client_closed = tokio::select! {
        result = stream.read(&mut byte) => result? == 0,
        () = runtime_shutdown::changed(&mut shutdown) => false,
    };
    // A shutdown test intentionally has no disconnect observer; the outcome remains explicit.
    match closed.send(client_closed) {
        Ok(()) => Ok(()),
        Err(_) if runtime_shutdown::requested(&shutdown) => Ok(()),
        Err(_) => Err(anyhow::anyhow!("Disconnect observer closed")),
    }
}

async fn receive_headers(
    stream: &mut TcpStream,
    shutdown: &mut RuntimeShutdownReceiver,
) -> anyhow::Result<bool> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") {
        let mut byte = [0_u8];
        let read = tokio::select! {
            result = stream.read(&mut byte) => result?,
            () = runtime_shutdown::changed(shutdown) => return Ok(false),
        };
        anyhow::ensure!(
            read == 1 && bytes.len() < 16_384,
            "Expected bounded HTTP headers"
        );
        bytes.push(byte[0]);
    }
    Ok(true)
}

#[tokio::test]
async fn manual_search_http_auth_failure_is_classified_without_credentials() -> anyhow::Result<()> {
    with_indexer_database(async move |config, _database| auth_failure(&config).await).await
}

#[tokio::test]
async fn manual_search_success_ingests_http_response_and_seals_stable_page() -> anyhow::Result<()> {
    with_indexer_database(async move |config, _database| completed_attempt(&config, false).await)
        .await
}

#[tokio::test]
async fn manual_search_empty_http_response_seals_zero_result_page() -> anyhow::Result<()> {
    with_indexer_database(async move |config, _database| completed_attempt(&config, true).await)
        .await
}

async fn completed_attempt(config: &Arc<ConfigService>, empty: bool) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let request_id =
        configured_request(config, &format!("http://{}/api", listener.local_addr()?)).await?;
    let run = store::claim_next(config.pool())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected an owned HTTP attempt"))?;
    let source = run.indexer_instance_public_id;
    let credential = &run.connection.as_ref().map_err(|error| *error)?.api_key;
    let hash = format!("{}00000000", Uuid::new_v4().simple());
    let body = if empty {
        "<rss><channel/></rss>".to_owned()
    } else {
        format!(
            r#"<rss xmlns:t="http://torznab.com/schemas/2015/feed"><channel><item><title>Pending upstream search</title><size>1048576</size><guid>{hash}</guid><enclosure url="https://example.test/download?apikey={credential}"/><comments>https://example.test/details</comments><pubDate>Sat, 24 Oct 2026 00:00:00 GMT</pubDate><t:attr name="infohash" value="{hash}"/><t:attr name="category" value="2000"/><t:attr name="seeders" value="17"/><t:attr name="leechers" value="2"/></item></channel></rss>"#
        )
    };
    let (shutdown, mut receiver) = runtime_shutdown::channel();
    let server = tokio::spawn(serve_feed(listener, body, receiver.clone()));
    let runtime =
        ManualSearchRuntime::new(Arc::clone(config), crate::bootstrap::manual_search_client);
    let executed = runtime
        .execute(run, &mut receiver)
        .await
        .map_err(anyhow::Error::from);
    let _ = runtime_shutdown::request(&shutdown);
    let upstream = server
        .await
        .map_err(anyhow::Error::from)
        .and_then(|result| result);
    match (executed, upstream) {
        (Ok(()), Ok(())) => {}
        (Err(error), Ok(())) | (Ok(()), Err(error)) => return Err(error),
        (Err(operation), Err(upstream)) => {
            return Err(anyhow::anyhow!(
                "HTTP attempt failed: {operation}; upstream settlement: {upstream}"
            ));
        }
    }
    let pages = search_pages::search_page_list(config.pool(), Uuid::nil(), request_id).await?;
    anyhow::ensure!(pages.len() == 1);
    anyhow::ensure!(pages[0].sealed_at.is_some());
    anyhow::ensure!(pages[0].item_count == i32::from(!empty));
    let explain =
        search_pages::search_request_explainability(config.pool(), Uuid::nil(), request_id).await?;
    anyhow::ensure!(explain.skipped_failed_indexers == 0);
    anyhow::ensure!(explain.retrying_indexers == 0);
    anyhow::ensure!(store::claim_next(config.pool()).await?.is_none());
    if !empty {
        let rows =
            search_pages::search_page_fetch(config.pool(), Uuid::nil(), request_id, 1).await?;
        let repeated =
            search_pages::search_page_fetch(config.pool(), Uuid::nil(), request_id, 1).await?;
        anyhow::ensure!(rows.len() == 1 && repeated.len() == 1);
        anyhow::ensure!(rows[0].infohash_v1.as_deref() == Some(hash.as_str()));
        anyhow::ensure!(rows[0].indexer_instance_public_id == Some(source));
        anyhow::ensure!(rows[0].tracker_category == Some(2000));
        anyhow::ensure!(rows[0].download_url.is_none());
        anyhow::ensure!(rows[0].details_url.as_deref() == Some("https://example.test/details"));
        anyhow::ensure!(rows[0].item_position == repeated[0].item_position);
        anyhow::ensure!(
            rows[0].canonical_torrent_public_id == repeated[0].canonical_torrent_public_id
        );
    }
    Ok(())
}

async fn serve_feed(
    listener: TcpListener,
    body: String,
    mut shutdown: RuntimeShutdownReceiver,
) -> anyhow::Result<()> {
    let (mut stream, _) = tokio::select! {
        result = listener.accept() => result?,
        () = runtime_shutdown::changed(&mut shutdown) => return Ok(()),
    };
    if receive_headers(&mut stream, &mut shutdown).await? {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).await?;
    }
    Ok(())
}

async fn auth_failure(config: &ConfigService) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let request_id =
        configured_request(config, &format!("http://{}/api", listener.local_addr()?)).await?;
    let run = store::claim_next(config.pool())
        .await?
        .ok_or_else(|| anyhow::anyhow!("Expected authentication attempt"))?;
    let client = crate::bootstrap::manual_search_client(&run)
        .map_err(|failure| anyhow::anyhow!(failure.class))?;
    let (shutdown, mut receiver) = runtime_shutdown::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = tokio::select! {
            result = listener.accept() => result?,
            () = runtime_shutdown::changed(&mut receiver) => return Ok::<(), anyhow::Error>(()),
        };
        if receive_headers(&mut stream, &mut receiver).await? {
            stream
                .write_all(
                    b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await?;
        }
        Ok(())
    });
    let response = torznab::fetch(&client, &run).await;
    let _ = runtime_shutdown::request(&shutdown);
    server.await??;
    let Err(failure) = response else {
        return Err(anyhow::anyhow!(
            "Rejected authentication must be classified"
        ));
    };
    anyhow::ensure!(failure.class == "auth_error" && failure.http_status == Some(401));
    let credential = &run.connection.as_ref().map_err(|error| *error)?.api_key;
    let private = !format!("{failure:?}").contains(credential);
    anyhow::ensure!(
        private,
        "Transport diagnostics must not contain the configured credential"
    );
    store::complete(
        config.pool(),
        &run,
        ManualSearchCompletion {
            outcome: ManualSearchOutcome::Failure(failure.class),
            http_status: failure.http_status,
            latency_ms: 0,
            result_count: 0,
            retry_after_seconds: None,
        },
    )
    .await?;
    let explain =
        search_pages::search_request_explainability(config.pool(), Uuid::nil(), request_id).await?;
    anyhow::ensure!(explain.skipped_failed_indexers == 1);
    Ok(())
}
