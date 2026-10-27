//! Bounded Torznab transport and RSS normalization. Upstream text is never diagnostic.

use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use revaer_data::indexers::manual_search::{
    ManualSearchConnection, ManualSearchLinkPrivacy, ManualSearchRun,
};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ITEMS: usize = 1000;
const TORZNAB_NAMESPACE: &str = "http://torznab.com/schemas/2015/feed";

pub(super) struct Release {
    pub privacy: ManualSearchLinkPrivacy,
    pub title: String,
    pub guid: Option<String>,
    pub details_url: Option<String>,
    pub download_url: Option<String>,
    pub magnet: Option<String>,
    pub infohash: Option<String>,
    pub size: i64,
    pub seeders: Option<i32>,
    pub leechers: Option<i32>,
    pub published_at: Option<DateTime<Utc>>,
    pub tracker_category: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Failure {
    pub class: &'static str,
    pub http_status: Option<i32>,
    pub retry_after_seconds: Option<i32>,
}

impl Failure {
    pub(crate) const fn classified(class: &'static str) -> Self {
        Self {
            class,
            http_status: None,
            retry_after_seconds: None,
        }
    }
}

fn network_failure(error: &reqwest::Error) -> Failure {
    Failure::classified(if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection_refused"
    } else {
        "unknown"
    })
}

pub(super) async fn fetch(client: &Client, run: &ManualSearchRun) -> Result<Vec<Release>, Failure> {
    let connection = run
        .connection
        .as_ref()
        .map_err(|_| Failure::classified("auth_error"))?;
    let url = request_url(run, connection)?;
    let text = fetch_text(client, url, run.read_timeout_ms).await?;
    parse(&text, &connection.api_key)
}

/// Shared bounded transport for search releases and recent RSS identifiers.
pub(crate) async fn fetch_text(
    client: &Client,
    url: Url,
    read_timeout_ms: i32,
) -> Result<String, Failure> {
    let timeout_ms = u64::try_from(read_timeout_ms).map_err(|_| Failure::classified("unknown"))?;
    let mut response = client
        .get(url)
        .timeout(Duration::from_millis(timeout_ms))
        .send()
        .await
        .map_err(|error| network_failure(&error))?;
    let status = response.status();
    if !status.is_success() {
        let class = match status.as_u16() {
            401 => "auth_error",
            403 => "http_403",
            429 => "http_429",
            500..=599 => "http_5xx",
            _ => "unknown",
        };
        return Err(Failure {
            class,
            http_status: Some(i32::from(status.as_u16())),
            retry_after_seconds: response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<i32>().ok())
                .filter(|value| *value >= 0),
        });
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES as u64)
    {
        return Err(Failure::classified("parse_error"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| network_failure(&error))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(Failure::classified("parse_error"));
        }
        bytes.extend_from_slice(&chunk);
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| Failure::classified("parse_error"))?;
    Ok(text.to_string())
}

fn request_url(run: &ManualSearchRun, connection: &ManualSearchConnection) -> Result<Url, Failure> {
    let mut url = Url::parse(&connection.endpoint).map_err(|_| Failure::classified("unknown"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(Failure::classified("unknown"));
    }
    // Preserve caller-owned non-authentication endpoint parameters, replacing protocol inputs.
    let parameters: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            !matches!(
                key.as_ref(),
                "t" | "q" | "apikey" | "cat" | "imdbid" | "tmdbid" | "tvdbid" | "season" | "ep"
            )
        })
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    let mode = match run.torznab_mode.as_deref() {
        None | Some("generic") => "search",
        Some("tv") => "tvsearch",
        Some("movie") => "movie",
        Some(_) => return Err(Failure::classified("unknown")),
    };
    {
        let mut query = url.query_pairs_mut();
        query
            .extend_pairs(parameters)
            .append_pair("t", mode)
            .append_pair("q", &run.query_text)
            .append_pair("apikey", &connection.api_key);
        if let Some((kind, value)) = &run.identifier {
            let key = match kind.as_str() {
                "imdb" => "imdbid",
                "tmdb" => "tmdbid",
                "tvdb" => "tvdbid",
                _ => return Err(Failure::classified("unknown")),
            };
            // Torznab's inherited movie API uses the numeric IMDb identifier.
            query.append_pair(key, value.strip_prefix("tt").unwrap_or(value));
        }
        for (key, value) in [("season", run.season_number), ("ep", run.episode_number)] {
            if let Some(value) = value {
                query.append_pair(key, &value.to_string());
            }
        }
        if !run.category_ids.is_empty() {
            let categories = run
                .category_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",");
            query.append_pair("cat", &categories);
        }
    }
    Ok(url)
}

fn child_text<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
        .and_then(|child| child.text())
}

fn attribute<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<&'a str> {
    node.children()
        .find(|child| {
            child.has_tag_name((TORZNAB_NAMESPACE, "attr")) && child.attribute("name") == Some(name)
        })
        .and_then(|child| child.attribute("value"))
}

fn optional_count(value: Option<&str>) -> Result<Option<i32>, Failure> {
    value
        .map(|text| {
            text.parse::<i32>()
                .ok()
                .filter(|count| *count >= 0)
                .ok_or(Failure::classified("parse_error"))
        })
        .transpose()
}

pub(crate) fn document(text: &str) -> Result<roxmltree::Document<'_>, Failure> {
    let invalid = Failure::classified("parse_error");
    if text.len() > MAX_RESPONSE_BYTES {
        return Err(invalid);
    }
    roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: 50_000,
            ..roxmltree::ParsingOptions::default()
        },
    )
    .map_err(|_| invalid)
}

fn parse(text: &str, api_key: &str) -> Result<Vec<Release>, Failure> {
    let invalid = Failure::classified("parse_error");
    let document = document(text)?;
    let root = document.root_element();
    if root.has_tag_name("error") {
        return Err(Failure::classified(match root.attribute("code") {
            Some("100" | "101" | "102") => "auth_error",
            _ => "parse_error",
        }));
    }
    if !root.has_tag_name("rss") {
        return Err(invalid);
    }
    let channel = root
        .children()
        .find(|node| node.has_tag_name("channel"))
        .ok_or(invalid)?;
    let mut releases = Vec::new();
    for item in channel.children().filter(|node| node.has_tag_name("item")) {
        if releases.len() == MAX_ITEMS {
            return Err(invalid);
        }
        releases.push(parse_release(item, api_key)?);
    }
    Ok(releases)
}

fn parse_release(item: roxmltree::Node<'_, '_>, api_key: &str) -> Result<Release, Failure> {
    let invalid = Failure::classified("parse_error");
    let title = child_text(item, "title")
        .filter(|value| !value.trim().is_empty())
        .ok_or(invalid)?;
    let size = attribute(item, "size")
        .or_else(|| child_text(item, "size"))
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|size| *size >= 0)
        .ok_or(invalid)?;
    let infohash = attribute(item, "infohash").map(str::to_ascii_lowercase);
    if infohash
        .as_ref()
        .is_some_and(|hash| hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Err(invalid);
    }
    let enclosure = item
        .children()
        .find(|node| node.has_tag_name("enclosure"))
        .and_then(|node| node.attribute("url"));
    let link = child_text(item, "link");
    let magnet = attribute(item, "magneturl")
        .or_else(|| enclosure.filter(|url| url.starts_with("magnet:")))
        .or_else(|| link.filter(|url| url.starts_with("magnet:")));
    let download_url = enclosure.or(link).filter(|url| !url.starts_with("magnet:"));
    let seeders = optional_count(attribute(item, "seeders"))?;
    let leechers = match attribute(item, "leechers") {
        Some(value) => optional_count(Some(value))?,
        None => optional_count(attribute(item, "peers"))?
            .zip(seeders)
            .map(|(peers, seeds)| peers.saturating_sub(seeds).max(0)),
    };
    let published_at = child_text(item, "pubDate")
        .map(|value| {
            DateTime::parse_from_rfc2822(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| invalid)
        })
        .transpose()?;
    let details_url = child_text(item, "comments");
    let guid = child_text(item, "guid");
    if !api_key.is_empty() && title.contains(api_key) {
        return Err(invalid);
    }
    Ok(Release {
        privacy: ManualSearchLinkPrivacy {
            details: private_url(details_url, api_key),
            download: private_url(download_url, api_key),
            magnet: private_url(magnet, api_key),
        },
        title: title.to_owned(),
        guid: guid.map(str::to_owned),
        details_url: details_url.map(str::to_owned),
        download_url: download_url.map(str::to_owned),
        magnet: magnet.map(str::to_owned),
        infohash,
        size,
        seeders,
        leechers,
        published_at,
        tracker_category: optional_count(attribute(item, "category"))?,
    })
}

pub(crate) fn private_url(value: Option<&str>, api_key: &str) -> bool {
    let Some(value) = value else { return false };
    if !api_key.is_empty() && value.contains(api_key) {
        return true;
    }
    let Ok(url) = Url::parse(value) else {
        return true;
    };
    !url.username().is_empty()
        || url.password().is_some()
        || url.query_pairs().any(|(key, value)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "apikey" | "api_key" | "token" | "access_token" | "passkey" | "password" | "auth"
            ) || (!api_key.is_empty() && value.contains(api_key))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torznab_request_preserves_structured_inputs_and_encodes_reserved_characters()
    -> anyhow::Result<()> {
        use revaer_data::indexers::manual_search::ManualSearchRouting;
        use uuid::Uuid;
        for (mode, identifier, expected_mode, expected_id) in [
            ("tv", Some(("tvdb", "7")), "tvsearch", Some(("tvdbid", "7"))),
            (
                "movie",
                Some(("imdb", "tt0019798")),
                "movie",
                Some(("imdbid", "0019798")),
            ),
            (
                "movie",
                Some(("tmdb", "42")),
                "movie",
                Some(("tmdbid", "42")),
            ),
            ("generic", None, "search", None),
        ] {
            let secret = Uuid::new_v4().to_string();
            let mut endpoint = Url::parse("https://upstream.example.test/api?retained=1")?;
            endpoint
                .query_pairs_mut()
                .append_pair("apikey", &Uuid::new_v4().to_string())
                .append_pair("cat", "9999")
                .append_pair("imdbid", "9999999");
            let connection = ManualSearchConnection {
                endpoint: endpoint.into(),
                api_key: secret,
                routing: ManualSearchRouting::Direct { verify_tls: true },
            };
            let run = ManualSearchRun {
                search_request_public_id: Uuid::new_v4(),
                indexer_instance_public_id: Uuid::new_v4(),
                claim_token: Uuid::new_v4(),
                correlation_id: Uuid::new_v4(),
                retry_seq: 0,
                query_text: "A&B + café?".to_owned(),
                torznab_mode: Some(mode.to_owned()),
                identifier: identifier.map(|(kind, value)| (kind.to_owned(), value.to_owned())),
                season_number: (mode == "tv").then_some(0),
                episode_number: (mode == "tv").then_some(2),
                category_ids: vec![2000, 5000],
                connect_timeout_ms: 5000,
                read_timeout_ms: 15000,
                routing_policy_public_id: None,
                connection: Ok(connection),
            };
            let connection = run.connection.as_ref().map_err(|error| *error)?;
            let url =
                request_url(&run, connection).map_err(|error| anyhow::anyhow!(error.class))?;
            let observed = url
                .query_pairs()
                .into_owned()
                .collect::<std::collections::BTreeMap<_, _>>();
            anyhow::ensure!(observed.get("t").map(String::as_str) == Some(expected_mode));
            anyhow::ensure!(observed.get("q").map(String::as_str) == Some("A&B + café?"));
            anyhow::ensure!(observed.get("retained").map(String::as_str) == Some("1"));
            anyhow::ensure!(observed.get("cat").map(String::as_str) == Some("2000,5000"));
            let credential_matches = observed
                .get("apikey")
                .is_some_and(|value| value == &connection.api_key);
            anyhow::ensure!(
                credential_matches,
                "The configured credential must replace endpoint credentials"
            );
            for key in ["imdbid", "tmdbid", "tvdbid"] {
                let expected = expected_id
                    .filter(|(name, _)| *name == key)
                    .map(|(_, value)| value);
                anyhow::ensure!(observed.get(key).map(String::as_str) == expected);
            }
            anyhow::ensure!(
                observed.get("season").map(String::as_str) == (mode == "tv").then_some("0")
            );
            anyhow::ensure!(
                observed.get("ep").map(String::as_str) == (mode == "tv").then_some("2")
            );
        }
        Ok(())
    }

    #[test]
    fn torznab_private_links_are_retained_and_marked_without_rejecting_release()
    -> anyhow::Result<()> {
        let secret = uuid::Uuid::new_v4().to_string();
        let link = format!("https://example.test/download?apikey={secret}");
        let feed = format!(
            "<rss><channel><item><title>Private release</title><size>1024</size><guid>{link}</guid><link>{link}</link><comments>https://example.test/details</comments></item></channel></rss>"
        );
        let releases = parse(&feed, &secret).map_err(|error| anyhow::anyhow!(error.class))?;
        anyhow::ensure!(releases.len() == 1 && releases[0].privacy.download);
        anyhow::ensure!(!releases[0].privacy.details && !releases[0].privacy.magnet);
        anyhow::ensure!(
            releases[0].download_url.as_deref() == Some(link.as_str()),
            "Original private target must be retained"
        );
        for value in [
            "https://example.test/release?passkey=opaque",
            "https://example.test/release?API_KEY=opaque",
            "https://user:opaque@example.test/release",
            "magnet:?xt=urn:btih:abcdef&token=opaque",
        ] {
            anyhow::ensure!(private_url(Some(value), &secret));
        }
        anyhow::ensure!(!private_url(
            Some("https://example.test/details?id=42"),
            &secret
        ));
        anyhow::ensure!(!private_url(None, &secret));
        Ok(())
    }

    #[test]
    fn torznab_parser_accepts_matching_and_empty_feeds_and_rejects_malformed() -> anyhow::Result<()>
    {
        let feed = r#"<rss xmlns:t="http://torznab.com/schemas/2015/feed"><channel><item><title>Distinct release</title><size>1048576</size><guid>release-id</guid><comments>https://example.test/release</comments><t:attr name="infohash" value="ABCDEF0123456789ABCDEF0123456789ABCDEF01"/><t:attr name="magneturl" value="magnet:?xt=urn:btih:abcdef0123456789abcdef0123456789abcdef01"/><t:attr name="seeders" value="17"/><t:attr name="peers" value="19"/></item></channel></rss>"#;
        let releases = parse(feed, "").map_err(|failure| anyhow::anyhow!(failure.class))?;
        anyhow::ensure!(releases.len() == 1);
        anyhow::ensure!(releases[0].title == "Distinct release" && releases[0].size == 1_048_576);
        anyhow::ensure!(
            releases[0].infohash.as_deref() == Some("abcdef0123456789abcdef0123456789abcdef01")
        );
        anyhow::ensure!(releases[0].leechers == Some(2));
        anyhow::ensure!(
            parse("<rss><channel/></rss>", "")
                .map_err(|failure| anyhow::anyhow!(failure.class))?
                .is_empty()
        );
        for malformed in [
            "<rss>",
            "<html/>",
            "<!DOCTYPE rss [<!ENTITY secret 'value'>]><rss><channel/></rss>",
            "<rss><channel><item><title>Missing size</title></item></channel></rss>",
        ] {
            anyhow::ensure!(matches!(
                parse(malformed, ""),
                Err(Failure {
                    class: "parse_error",
                    ..
                })
            ));
        }
        anyhow::ensure!(matches!(
            parse(
                r#"<error code="100" description="untrusted upstream text"/>"#,
                ""
            ),
            Err(Failure {
                class: "auth_error",
                ..
            })
        ));
        Ok(())
    }
}
