use super::*;
use crate::cardigann::tests::DEFINITION;
use revaer_data::indexers::manual_search::{
    ManualSearchConnection, ManualSearchEngine, ManualSearchRouting,
};
use uuid::Uuid;

fn plan() -> anyhow::Result<CardigannPlan> {
    crate::cardigann::prepare(DEFINITION)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("Rules absent"))
}

fn run() -> ManualSearchRun {
    ManualSearchRun {
        search_request_public_id: Uuid::new_v4(),
        indexer_instance_public_id: Uuid::new_v4(),
        claim_token: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        retry_seq: 0,
        query_text: "A&B + café?".to_owned(),
        torznab_mode: Some("generic".to_owned()),
        identifier: None,
        season_number: None,
        episode_number: None,
        category_ids: vec![2000],
        connect_timeout_ms: 5000,
        read_timeout_ms: 15000,
        routing_policy_public_id: None,
        connection: Ok(ManualSearchConnection {
            endpoint: "http://127.0.0.1:8765/".to_owned(),
            engine: ManualSearchEngine::Cardigann,
            api_key: None,
            routing: ManualSearchRouting::Direct { verify_tls: true },
        }),
    }
}

#[test]
fn cardigann_url_encodes_only_keywords_and_rejects_structured_or_invalid_rules()
-> anyhow::Result<()> {
    let mut run = run();
    let mut plan = plan()?;
    let url = request_url(&run, &plan).map_err(|error| anyhow::anyhow!(error.class))?;
    assert_eq!(url.path(), "/search/releases");
    assert_eq!(
        url.query(),
        Some("order=newest&term=A%26B+%2B+caf%C3%A9%3F")
    );
    for mode in ["movie", "tv", "music", "book"] {
        run.torznab_mode = Some(mode.to_owned());
        assert_eq!(
            request_url(&run, &plan)
                .err()
                .and_then(|error| error.diagnostic),
            Some(UNSUPPORTED)
        );
    }
    run.torznab_mode = None;
    run.identifier = Some(("imdb".to_owned(), "tt1".to_owned()));
    assert!(request_url(&run, &plan).is_err());
    run.identifier = None;
    for path in [
        "https://other.example/",
        "//other.example/",
        "{{.Keywords}}",
    ] {
        plan.search_path = path.to_owned();
        assert!(request_url(&run, &plan).is_err());
    }
    plan.input_values.pop();
    assert!(request_url(&run, &plan).is_err());
    plan.field_attributes.pop();
    assert!(fields(&plan).is_err());
    Ok(())
}

fn page() -> String {
    "<!doctype html><table class=releases><tr class=release><td><a class=title href='../details/1'> A &amp; B </a><a class=magnet href='magnet:?xt=urn:btih:0123456789012345678901234567890123456789'>magnet</a><td class=category>film<td class=bytes>1048576<td class=seeders>17<td class=leechers>2<td><time datetime='2026-10-24T01:00:00+01:00'></table>".to_owned()
}

fn parse_page(text: &str, plan: &CardigannPlan) -> Result<Vec<Release>, Failure> {
    let rows = Selector::parse(&plan.rows_selector).map_err(|_| INVALID)?;
    let fields = fields(plan)?;
    let base = Url::parse("https://public.example/search/releases").map_err(|_| INVALID)?;
    parse(text, &base, plan, &rows, &fields)
}

#[test]
fn cardigann_html5_extraction_resolves_links_maps_categories_and_preserves_metadata()
-> anyhow::Result<()> {
    let plan = plan()?;
    let rows = parse_page(&page(), &plan).map_err(|error| anyhow::anyhow!(error.class))?;
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.title, "A & B");
    assert_eq!(
        row.details_url.as_deref(),
        Some("https://public.example/details/1")
    );
    assert_eq!(row.size, 1_048_576);
    assert_eq!(row.tracker_category, Some(2000));
    assert_eq!(row.seeders, Some(17));
    assert_eq!(row.leechers, Some(2));
    assert_eq!(
        row.published_at.map(|date| date.to_rfc3339()).as_deref(),
        Some("2026-10-24T00:00:00+00:00")
    );
    assert!(!row.privacy.details && !row.privacy.magnet);
    assert!(
        parse_page("<!doctype html><p>Nothing found", &plan)
            .map_err(|error| anyhow::anyhow!(error.class))?
            .is_empty()
    );
    Ok(())
}

#[test]
fn cardigann_invalid_required_and_supplied_optional_fields_fail_without_fabrication()
-> anyhow::Result<()> {
    let plan = plan()?;
    for (from, to) in [
        ("> A &amp; B <", "> <"),
        ("1048576", "1 MiB"),
        ("1048576", "-1"),
        ("class=seeders>17", "class=seeders>-1"),
        ("class=leechers>2", "class=leechers>lots"),
        ("film", "unknown"),
        ("2026-10-24T01:00:00+01:00", "yesterday"),
        ("../details/1", "javascript:alert(1)"),
        ("0123456789012345678901234567890123456789", "broken"),
        ("class=bytes", "class=missing"),
    ] {
        assert!(
            parse_page(&page().replace(from, to), &plan).is_err(),
            "Must reject {to}"
        );
    }
    for hash in [
        "urn:btih:broken",
        "urn:btmh:1220invalid",
        "urn:other:0123456789012345678901234567890123456789",
    ] {
        assert!(!valid_hash(hash));
    }
    assert!(valid_hash("urn:btih:ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"));
    assert!(valid_hash(&format!("urn:btmh:1220{}", "a".repeat(64))));
    Ok(())
}

#[test]
fn cardigann_parser_bounds_and_private_links_use_existing_presentation_policy() -> anyhow::Result<()>
{
    let mut plan = plan()?;
    let name = plan
        .field_names
        .iter()
        .position(|name| name == "magnet")
        .ok_or_else(|| anyhow::anyhow!("Magnet selector absent"))?;
    plan.field_names[name] = "download".to_owned();
    let secret = Uuid::new_v4().to_string();
    let target = format!("https://upstream.example/download?apikey={secret}");
    let text = page().replace(
        "magnet:?xt=urn:btih:0123456789012345678901234567890123456789",
        &target,
    );
    let releases = parse_page(&text, &plan).map_err(|error| anyhow::anyhow!(error.class))?;
    assert!(releases[0].privacy.download);
    assert_eq!(releases[0].download_url.as_deref(), Some(target.as_str()));
    let too_many = format!("{}{}", page(), "<p>x</p>".repeat(50000));
    assert!(parse_page(&too_many, &plan).is_err());
    let row = page()
        .replace("<!doctype html><table class=releases>", "")
        .replace("</table>", "</tr>");
    assert!(
        parse_page(
            &format!("<table class=releases>{}</table>", row.repeat(1001)),
            &plan
        )
        .is_err()
    );
    Ok(())
}

#[tokio::test]
async fn cardigann_transport_classifies_http_failures_and_uses_selected_proxy() -> anyhow::Result<()>
{
    use revaer_data::indexers::manual_search::ManualSearchProxy;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    for (status, expected) in [
        (401, "auth_error"),
        (429, "http_429"),
        (503, "http_5xx"),
        (200, "success"),
    ] {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let mut run = run();
        let mut plan = plan()?;
        let address = listener.local_addr()?;
        plan.origin = if status == 200 {
            "http://public.example/".to_owned()
        } else {
            format!("http://{address}/")
        };
        if status == 200 {
            run.connection.as_mut().map_err(|error| *error)?.routing = ManualSearchRouting::Proxy {
                verify_tls: true,
                proxy: ManualSearchProxy {
                    scheme: "http",
                    host: address.ip().to_string(),
                    port: address.port(),
                    authentication: None,
                },
            };
        }
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut headers = Vec::new();
            while !headers.ends_with(b"\r\n\r\n") {
                anyhow::ensure!(headers.len() < 16384);
                headers.push(stream.read_u8().await?);
            }
            if status == 200 {
                anyhow::ensure!(
                    String::from_utf8(headers)?
                        .starts_with("GET http://public.example/search/releases?")
                );
            }
            let body = page();
            stream.write_all(format!("HTTP/1.1 {status} Response\r\nRetry-After: 3\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await?;
            Ok::<(), anyhow::Error>(())
        });
        let client = crate::bootstrap::manual_search_client(&run)
            .map_err(|error| anyhow::anyhow!(error.class))?;
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            fetch(&client, &run, &plan),
        )
        .await?;
        server.await??;
        if expected == "success" {
            assert_eq!(
                result.map_err(|error| anyhow::anyhow!(error.class))?.len(),
                1
            );
        } else {
            let error = result
                .err()
                .ok_or_else(|| anyhow::anyhow!("HTTP failure accepted"))?;
            assert_eq!(error.class, expected);
            assert_eq!(error.http_status, Some(status));
            if status == 429 {
                assert_eq!(error.retry_after_seconds, Some(3));
            }
        }
    }
    Ok(())
}

#[tokio::test]
async fn cardigann_timeout_and_failed_proxy_never_fall_back() -> anyhow::Result<()> {
    use revaer_data::indexers::manual_search::ManualSearchProxy;
    use tokio::net::TcpListener;
    // A selected but unreachable proxy must not fall back to the reachable origin.
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let closed = TcpListener::bind(("127.0.0.1", 0)).await?;
    let proxy = closed.local_addr()?;
    drop(closed);
    let mut run = run();
    let mut plan = plan()?;
    plan.origin = format!("http://{address}/");
    run.connection.as_mut().map_err(|error| *error)?.routing = ManualSearchRouting::Proxy {
        verify_tls: true,
        proxy: ManualSearchProxy {
            scheme: "http",
            host: proxy.ip().to_string(),
            port: proxy.port(),
            authentication: None,
        },
    };
    let client = crate::bootstrap::manual_search_client(&run)
        .map_err(|error| anyhow::anyhow!(error.class))?;
    assert_eq!(
        fetch(&client, &run, &plan)
            .await
            .err()
            .map(|failure| failure.class),
        Some("connection_refused")
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(20), listener.accept())
            .await
            .is_err()
    );
    run.connection.as_mut().map_err(|error| *error)?.routing =
        ManualSearchRouting::Direct { verify_tls: true };
    run.read_timeout_ms = 20;
    let client = crate::bootstrap::manual_search_client(&run)
        .map_err(|error| anyhow::anyhow!(error.class))?;
    let (release, released) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await?;
        released.await?;
        Ok::<(), anyhow::Error>(())
    });
    let result = fetch(&client, &run, &plan).await;
    release
        .send(())
        .map_err(|()| anyhow::anyhow!("Timeout fixture closed"))?;
    server.await??;
    assert_eq!(result.err().map(|failure| failure.class), Some("timeout"));
    Ok(())
}
