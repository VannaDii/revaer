//! Real claim regressions use the sealed initializer and normal configuration.

use super::{
    ManualSearchCompletion, ManualSearchLinkPrivacy, ManualSearchOutcome, ManualSearchWriteOutcome,
    claim_next, complete, ingest, is_active,
};
use crate::indexers::instances::{
    IndexerInstanceFieldValueInput, indexer_instance_create, indexer_instance_field_bind_secret,
    indexer_instance_field_set_value,
};
use crate::indexers::search_pages::{search_page_fetch, search_page_list};
use crate::indexers::search_requests::{SearchRequestCreateInput, search_request_create};
use crate::indexers::search_results::SearchResultIngestInput;
use crate::indexers::secrets::secret_create;
use uuid::Uuid;

#[tokio::test]
async fn disabling_source_settles_owned_manual_attempt_without_losing_partial_results()
-> anyhow::Result<()> {
    use crate::indexers::instances::{IndexerInstanceUpdateInput, indexer_instance_update};
    use crate::indexers::search_requests::search_request_is_terminal;

    let test_db = crate::indexers::setup_indexer_db("disabled partial search source").await?;
    let result = async {
        let pool = test_db.pool();
        let (source, _) = configured_source(pool, "http://127.0.0.1:8765/api", None).await?;
        let request = create_request(pool, "Disable after partial response").await?;
        let run = claim_next(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected owned work"))?;
        let input = release_input(request, source);
        anyhow::ensure!(
            ingest(pool, &run, &input, ManualSearchLinkPrivacy::default()).await?
                == ManualSearchWriteOutcome::Applied
        );
        anyhow::ensure!(!search_request_is_terminal(pool, Uuid::nil(), request).await?);
        indexer_instance_update(
            pool,
            &IndexerInstanceUpdateInput {
                actor_user_public_id: Uuid::nil(),
                indexer_instance_public_id: source,
                display_name: None,
                priority: None,
                trust_tier_key: None,
                routing_policy_public_id: None,
                is_enabled: Some(false),
                enable_rss: None,
                enable_automatic_search: None,
                enable_interactive_search: None,
            },
        )
        .await?;
        anyhow::ensure!(
            ingest(pool, &run, &input, ManualSearchLinkPrivacy::default()).await?
                == ManualSearchWriteOutcome::OwnershipLost
        );
        anyhow::ensure!(
            complete(
                pool,
                &run,
                ManualSearchCompletion {
                    diagnostic: None,
                    outcome: ManualSearchOutcome::Canceled,
                    http_status: Some(200),
                    latency_ms: 0,
                    result_count: 1,
                    retry_after_seconds: None,
                }
            )
            .await?
                == ManualSearchWriteOutcome::Applied
        );
        anyhow::ensure!(search_request_is_terminal(pool, Uuid::nil(), request).await?);
        let page = search_page_fetch(pool, Uuid::nil(), request, 1).await?;
        anyhow::ensure!(page.len() == 1 && page[0].item_position == Some(1));
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

#[tokio::test]
async fn manual_claim_retries_only_rolled_back_statements() -> anyhow::Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let test_db = crate::indexers::setup_indexer_db("claim rollback recovery").await?;
    let result = async {
        let pool = test_db.pool();
        let (source, _) = configured_source(pool, "http://127.0.0.1:8765/api", None).await?;
        for rollback in [
            include_str!("sql/claim_deadlock.sql"),
            include_str!("sql/claim_serialization.sql"),
        ] {
            let request = create_request(pool, "Recover rolled back claim").await?;
            let calls = AtomicUsize::new(0);
            let row = super::claim_query(|| {
                let query = if calls.fetch_add(1, Ordering::SeqCst) < 2 {
                    rollback
                } else {
                    super::CLAIM
                };
                sqlx::query_as::<_, super::ClaimRow>(query).fetch_optional(pool)
            })
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected a recovered claim"))?;
            let claim = row.into_run();
            anyhow::ensure!(calls.load(Ordering::SeqCst) == 3);
            anyhow::ensure!(claim.search_request_public_id == request);
            anyhow::ensure!(claim.indexer_instance_public_id == source && claim.retry_seq == 0);
            anyhow::ensure!(is_active(pool, &claim).await?);
            anyhow::ensure!(claim_next(pool).await?.is_none());
            let completion = ManualSearchCompletion {
                diagnostic: None,
                outcome: ManualSearchOutcome::Success,
                http_status: Some(200),
                latency_ms: 0,
                result_count: 0,
                retry_after_seconds: None,
            };
            anyhow::ensure!(
                complete(pool, &claim, completion).await? == ManualSearchWriteOutcome::Applied
            );
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM outbound_request_log WHERE correlation_id = $1",
            )
            .bind(claim.correlation_id)
            .fetch_one(pool)
            .await?;
            anyhow::ensure!(count == 1);
            let pages = search_page_list(pool, Uuid::nil(), request).await?;
            anyhow::ensure!(pages.len() == 1 && pages[0].sealed_at.is_some());
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

#[tokio::test]
async fn manual_claim_preserves_exhausted_and_nonretryable_errors() -> anyhow::Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let test_db = crate::indexers::setup_indexer_db("claim rollback limits").await?;
    let result = async {
        let pool = test_db.pool();
        for (query, code, expected_calls) in [
            (include_str!("sql/claim_deadlock.sql"), "40P01", 3),
            (include_str!("sql/claim_serialization.sql"), "40001", 3),
            ("SELECT 1 / 0", "22012", 1),
        ] {
            let calls = AtomicUsize::new(0);
            let outcome = super::claim_query(|| {
                calls.fetch_add(1, Ordering::SeqCst);
                sqlx::query_as::<_, super::ClaimRow>(query).fetch_optional(pool)
            })
            .await;
            let Err(error) = outcome else {
                anyhow::bail!("Expected the original claim error");
            };
            anyhow::ensure!(calls.load(Ordering::SeqCst) == expected_calls);
            anyhow::ensure!(
                error
                    .as_database_error()
                    .and_then(sqlx::error::DatabaseError::code)
                    .as_deref()
                    == Some(code)
            );
        }
        let calls = AtomicUsize::new(0);
        let outcome = super::claim_query(|| {
            calls.fetch_add(1, Ordering::SeqCst);
            std::future::ready(Err(sqlx::Error::PoolClosed))
        })
        .await;
        anyhow::ensure!(matches!(outcome, Err(sqlx::Error::PoolClosed)));
        anyhow::ensure!(calls.load(Ordering::SeqCst) == 1);
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

#[tokio::test]
async fn concurrent_manual_claims_own_one_attempt_and_resolve_bound_credentials()
-> anyhow::Result<()> {
    let test_db = crate::indexers::setup_indexer_db("manual Torznab claim").await?;
    let result = async {
        let pool = test_db.pool();
        let actor = Uuid::nil();
        let (source, credential) =
            configured_source(pool, "http://127.0.0.1:8765/api", None).await?;
        let request_id = create_request(pool, "Live manual search").await?;
        let (first, second) = tokio::join!(claim_next(pool), claim_next(pool));
        let mut claims: Vec<_> = [first?, second?].into_iter().flatten().collect();
        anyhow::ensure!(claims.len() == 1, "One attempt must have exactly one owner");
        let claim = &mut claims[0];
        anyhow::ensure!(claim.search_request_public_id == request_id);
        anyhow::ensure!(claim.indexer_instance_public_id == source);
        anyhow::ensure!(!claim.claim_token.is_nil() && !claim.correlation_id.is_nil());
        anyhow::ensure!(claim.query_text == "Live manual search" && claim.retry_seq == 0);
        let connection = claim.connection.as_ref().map_err(|error| *error)?;
        anyhow::ensure!(connection.endpoint == "http://127.0.0.1:8765/api");
        anyhow::ensure!(
            connection.api_key.as_deref() == Some(credential.as_str()),
            "Bound credential must match"
        );
        anyhow::ensure!(
            claim_next(pool).await?.is_none(),
            "Claimed work must not execute twice"
        );
        anyhow::ensure!(is_active(pool, claim).await?);
        let owned_token = claim.claim_token;
        claim.claim_token = Uuid::new_v4();
        anyhow::ensure!(!is_active(pool, claim).await?);
        let empty = ManualSearchCompletion {
            diagnostic: None,
            outcome: ManualSearchOutcome::Success,
            http_status: Some(200),
            latency_ms: 0,
            result_count: 0,
            retry_after_seconds: None,
        };
        anyhow::ensure!(
            complete(pool, claim, empty).await? == ManualSearchWriteOutcome::OwnershipLost
        );
        let hash = "abcdef0123456789abcdef0123456789abcdef01";
        let privacy = ManualSearchLinkPrivacy::default();
        let input = release_input(request_id, source);
        anyhow::ensure!(
            ingest(pool, claim, &input, privacy).await? == ManualSearchWriteOutcome::OwnershipLost
        );
        claim.claim_token = owned_token;
        anyhow::ensure!(
            ingest(pool, claim, &input, privacy).await? == ManualSearchWriteOutcome::Applied
        );
        anyhow::ensure!(
            ingest(pool, claim, &input, privacy).await? == ManualSearchWriteOutcome::Applied
        );
        anyhow::ensure!(complete(pool, claim, empty).await? == ManualSearchWriteOutcome::Applied);
        anyhow::ensure!(!is_active(pool, claim).await?);
        anyhow::ensure!(
            complete(pool, claim, empty).await? == ManualSearchWriteOutcome::OwnershipLost
        );
        let pages = search_page_list(pool, actor, request_id).await?;
        anyhow::ensure!(
            pages.len() == 1 && pages[0].item_count == 1 && pages[0].sealed_at.is_some()
        );
        let first = search_page_fetch(pool, actor, request_id, 1).await?;
        let repeated = search_page_fetch(pool, actor, request_id, 1).await?;
        anyhow::ensure!(first.len() == 1 && repeated.len() == 1);
        anyhow::ensure!(first[0].infohash_v1.as_deref() == Some(hash));
        anyhow::ensure!(first[0].indexer_instance_public_id == Some(source));
        anyhow::ensure!(first[0].item_position == repeated[0].item_position);
        anyhow::ensure!(
            first[0].canonical_torrent_public_id == repeated[0].canonical_torrent_public_id
        );
        anyhow::ensure!(
            ingest(pool, claim, &input, privacy).await? == ManualSearchWriteOutcome::OwnershipLost
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

#[tokio::test]
async fn canceled_manual_attempt_records_once_without_reviving_request() -> anyhow::Result<()> {
    let test_db = crate::indexers::setup_indexer_db("canceled manual attempt").await?;
    let result = async {
        let pool = test_db.pool();
        let actor = Uuid::nil();
        let source =
            indexer_instance_create(pool, actor, "torznab", "Cancel upstream", None, None, None)
                .await?;
        let request_id = create_request(pool, "Cancel owned work").await?;
        let claim = claim_next(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected an owned attempt"))?;
        anyhow::ensure!(claim.indexer_instance_public_id == source);
        crate::indexers::search_requests::search_request_cancel(pool, actor, request_id).await?;
        anyhow::ensure!(!is_active(pool, &claim).await?);
        let canceled = ManualSearchCompletion {
            diagnostic: None,
            outcome: ManualSearchOutcome::Canceled,
            http_status: None,
            latency_ms: 1,
            result_count: 0,
            retry_after_seconds: None,
        };
        anyhow::ensure!(
            complete(pool, &claim, canceled).await? == ManualSearchWriteOutcome::Applied
        );
        anyhow::ensure!(
            complete(pool, &claim, canceled).await? == ManualSearchWriteOutcome::OwnershipLost
        );
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM outbound_request_log WHERE correlation_id = $1",
        )
        .bind(claim.correlation_id)
        .fetch_one(pool)
        .await?;
        anyhow::ensure!(
            count == 1,
            "Cancellation must settle exactly one ledger entry"
        );
        let explain =
            crate::indexers::search_pages::search_request_explainability(pool, actor, request_id)
                .await?;
        anyhow::ensure!(explain.skipped_canceled_indexers == 1);
        anyhow::ensure!(claim_next(pool).await?.is_none());
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

async fn create_request(pool: &sqlx::PgPool, query: &str) -> anyhow::Result<Uuid> {
    let request = search_request_create(
        pool,
        &SearchRequestCreateInput {
            actor_user_public_id: Some(Uuid::nil()),
            query_text: query,
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

#[tokio::test]
async fn manual_search_rate_denial_defers_and_logs_zero_results() -> anyhow::Result<()> {
    use crate::indexers::rate_limits::{
        indexer_instance_set_rate_limit_policy, rate_limit_policy_create,
    };
    let test_db = crate::indexers::setup_indexer_db("manual rate denial").await?;
    let result = async {
        let pool = test_db.pool();
        let actor = Uuid::nil();
        let source = indexer_instance_create(pool, actor, "torznab", "Limited upstream", None, None, None).await?;
        let policy = rate_limit_policy_create(pool, actor, "One attempt per minute", 1, 0, 1).await?;
        indexer_instance_set_rate_limit_policy(pool, actor, source, Some(policy)).await?;
        let mut denied = 0;
        // Three immediate claims span at most two minute buckets.
        for _ in 0..3 {
            let request_id = create_request(pool, "Limited query").await?;
            if let Some(run) = claim_next(pool).await? {
                complete(pool, &run, ManualSearchCompletion {
                    diagnostic: None,
outcome: ManualSearchOutcome::Success, http_status: Some(200), latency_ms: 0,
                    result_count: 0, retry_after_seconds: None,
                }).await?;
            } else {
                let explain = crate::indexers::search_pages::search_request_explainability(pool, actor, request_id).await?;
                anyhow::ensure!(explain.rate_limited_indexers == 1 && explain.retrying_indexers == 1);
                anyhow::ensure!(!crate::indexers::search_requests::search_request_is_terminal(pool, actor, request_id).await?);
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM outbound_request_log log JOIN search_request request USING (search_request_id) WHERE request.search_request_public_id = $1 AND log.error_class = 'rate_limited' AND log.result_count = 0 AND log.latency_ms = 0 AND NOT log.parse_ok")
                    .bind(request_id).fetch_one(pool).await?;
                anyhow::ensure!(count == 1, "A deferred attempt must retain its valid ledger row");
                denied += 1;
            }
            crate::indexers::search_requests::search_request_cancel(pool, actor, request_id).await?;
        }
        anyhow::ensure!(denied > 0);
        Ok::<(), anyhow::Error>(())
    }.await;
    test_db.finish(result).await
}

#[tokio::test]
async fn manual_search_claim_resolves_authenticated_proxy_and_tls() -> anyhow::Result<()> {
    for mode in ["http_proxy", "socks_proxy"] {
        assert_proxy_claim(mode).await?;
    }
    Ok(())
}

async fn assert_proxy_claim(mode: &str) -> anyhow::Result<()> {
    use crate::indexers::routing::{
        routing_policy_bind_secret, routing_policy_create, routing_policy_set_param,
    };
    let test_db = crate::indexers::setup_indexer_db("manual proxy claim").await?;
    let result = async {
        let pool = test_db.pool();
        let actor = Uuid::nil();
        let route = routing_policy_create(pool, actor, "Authenticated route", mode).await?;
        let prefix = if mode == "http_proxy" {
            "proxy"
        } else {
            "socks"
        };
        for (key, value) in [
            (format!("{prefix}_host"), "::1"),
            (format!("{prefix}_username"), "proxy-operator"),
        ] {
            routing_policy_set_param(pool, actor, route, &key, Some(value), None, None).await?;
        }
        routing_policy_set_param(
            pool,
            actor,
            route,
            &format!("{prefix}_port"),
            None,
            Some(8766),
            None,
        )
        .await?;
        routing_policy_set_param(pool, actor, route, "verify_tls", None, None, Some(false)).await?;
        if mode == "http_proxy" {
            routing_policy_set_param(pool, actor, route, "proxy_use_tls", None, None, Some(true))
                .await?;
        }
        let password = Uuid::new_v4().to_string();
        let proxy_secret = secret_create(pool, actor, "password", &password).await?;
        let binding = format!("{mode}_auth");
        routing_policy_bind_secret(pool, actor, route, &binding, proxy_secret).await?;
        let (source, _) =
            configured_source(pool, "http://torznab.invalid/api", Some(route)).await?;
        let request_id = create_request(pool, "Routed query").await?;
        let run = claim_next(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected routed claim"))?;
        anyhow::ensure!(
            run.search_request_public_id == request_id
                && run.indexer_instance_public_id == source
                && run.routing_policy_public_id == Some(route)
        );
        let connection = run.connection.as_ref().map_err(|error| *error)?;
        let super::ManualSearchRouting::Proxy { verify_tls, proxy } = &connection.routing else {
            return Err(anyhow::anyhow!("Expected the configured proxy route"));
        };
        anyhow::ensure!(!verify_tls && proxy.host == "::1" && proxy.port == 8766);
        anyhow::ensure!(
            proxy.scheme
                == if mode == "http_proxy" {
                    "https"
                } else {
                    "socks5h"
                }
        );
        let authenticated = proxy
            .authentication
            .as_ref()
            .is_some_and(|(user, secret)| user == "proxy-operator" && secret == &password);
        anyhow::ensure!(
            authenticated,
            "Configured proxy authentication must resolve privately"
        );
        complete(
            pool,
            &run,
            ManualSearchCompletion {
                diagnostic: None,
                outcome: ManualSearchOutcome::Canceled,
                http_status: None,
                latency_ms: 0,
                result_count: 0,
                retry_after_seconds: None,
            },
        )
        .await?;
        crate::indexers::secrets::secret_revoke(pool, actor, proxy_secret).await?;
        create_request(pool, "Revoked proxy credential").await?;
        let revoked = claim_next(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected revoked-credential claim"))?;
        anyhow::ensure!(matches!(
            revoked.connection,
            Err(super::ManualSearchConfigurationError::CredentialUnavailable)
        ));
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

async fn configured_source(
    pool: &sqlx::PgPool,
    endpoint: &str,
    route: Option<Uuid>,
) -> anyhow::Result<(Uuid, String)> {
    let actor = Uuid::nil();
    let source = indexer_instance_create(
        pool,
        actor,
        "torznab",
        "Configured upstream",
        None,
        None,
        route,
    )
    .await?;
    indexer_instance_field_set_value(
        pool,
        &IndexerInstanceFieldValueInput {
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
    let secret = secret_create(pool, actor, "api_key", &credential).await?;
    indexer_instance_field_bind_secret(pool, actor, source, "api_key", secret).await?;
    Ok((source, credential))
}

#[tokio::test]
async fn manual_search_private_download_is_hidden_but_acquisition_keeps_original()
-> anyhow::Result<()> {
    let test_db = crate::indexers::setup_indexer_db("private manual search link").await?;
    let result = async {
        let pool = test_db.pool();
        let (source, credential) =
            configured_source(pool, "https://example.test/api", None).await?;
        let request_id = create_request(pool, "Private upstream download").await?;
        let claim = claim_next(pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Expected claim"))?;
        let download_url = format!("https://example.test/download?apikey={credential}");
        let input = SearchResultIngestInput {
            source_guid: Some("private-release"),
            details_url: Some("https://example.test/details"),
            download_url: Some(&download_url),
            title_raw: "Private download release",
            ..release_input(request_id, source)
        };
        let privacy = ManualSearchLinkPrivacy {
            download: true,
            ..ManualSearchLinkPrivacy::default()
        };
        anyhow::ensure!(
            ingest(pool, &claim, &input, privacy).await? == ManualSearchWriteOutcome::Applied
        );
        complete(
            pool,
            &claim,
            ManualSearchCompletion {
                diagnostic: None,
                outcome: ManualSearchOutcome::Success,
                http_status: Some(200),
                latency_ms: 1,
                result_count: 1,
                retry_after_seconds: None,
            },
        )
        .await?;
        let page = search_page_fetch(pool, Uuid::nil(), request_id, 1).await?;
        anyhow::ensure!(page.len() == 1 && page[0].download_url.is_none());
        anyhow::ensure!(page[0].details_url.as_deref() == input.details_url);
        let canonical_source = page[0]
            .canonical_torrent_source_public_id
            .ok_or_else(|| anyhow::anyhow!("Expected canonical source"))?;
        assert_private_acquisition_target(pool, canonical_source, &download_url).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    test_db.finish(result).await
}

async fn assert_private_acquisition_target(
    pool: &sqlx::PgPool,
    canonical_source: Uuid,
    download_url: &str,
) -> anyhow::Result<()> {
    use crate::indexers::torznab::{torznab_download_prepare, torznab_instance_create};

    let profile = crate::indexers::search_profiles::search_profile_create(
        pool,
        Uuid::nil(),
        "Private acquisition",
        Some(false),
        Some(50),
        Some("movies"),
        None,
    )
    .await?;
    let serving =
        torznab_instance_create(pool, Uuid::nil(), Some(profile), "Private acquisition").await?;
    let retained =
        torznab_download_prepare(pool, serving.torznab_instance_public_id, canonical_source)
            .await?;
    anyhow::ensure!(
        retained.as_deref() == Some(download_url),
        "Acquisition must retain the original private URL"
    );
    Ok(())
}

fn release_input(request_id: Uuid, source: Uuid) -> SearchResultIngestInput<'static> {
    SearchResultIngestInput {
        search_request_public_id: request_id,
        indexer_instance_public_id: source,
        source_guid: Some("controlled-release"),
        details_url: None,
        download_url: None,
        magnet_uri: None,
        title_raw: "Controlled release",
        size_bytes: Some(1024),
        infohash_v1: Some("abcdef0123456789abcdef0123456789abcdef01"),
        infohash_v2: None,
        magnet_hash: None,
        seeders: Some(17),
        leechers: None,
        published_at: None,
        uploader: None,
        observed_at: chrono::Utc::now(),
        attr_keys: None,
        attr_types: None,
        attr_value_text: None,
        attr_value_int: None,
        attr_value_bigint: None,
        attr_value_numeric: None,
        attr_value_bool: None,
        attr_value_uuid: None,
    }
}
