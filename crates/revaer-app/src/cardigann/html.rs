//! Execute one public GET and extract bounded releases with persisted CSS rules.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use revaer_data::indexers::cardigann::CardigannPlan;
use revaer_data::indexers::manual_search::{ManualSearchLinkPrivacy, ManualSearchRun};
use scraper::{ElementRef, Html, Selector};

use crate::manual_search::torznab::{Failure, Release, fetch_text, private_url};

use super::definition::substitute;

const INVALID: Failure = Failure::classified("parse_error");
const UNSUPPORTED: &str = "This Cardigann definition supports ordinary keyword search only; structured modes require a different definition";

struct Field<'a> {
    name: &'a str,
    selector: Selector,
    attribute: Option<&'a str>,
}

fn fields(plan: &CardigannPlan) -> Result<Vec<Field<'_>>, Failure> {
    if plan.field_names.len() != plan.field_selectors.len()
        || plan.field_names.len() != plan.field_attributes.len()
        || plan.field_names.len() > 9
        || plan.category_sources.len() != plan.category_targets.len()
    {
        return Err(Failure::configuration(super::REIMPORT_REQUIRED));
    }
    let mut fields = Vec::new();
    for ((name, selector), attribute) in plan
        .field_names
        .iter()
        .zip(&plan.field_selectors)
        .zip(&plan.field_attributes)
    {
        if ![
            "title", "details", "download", "magnet", "category", "size", "seeders", "leechers",
            "date",
        ]
        .contains(&name.as_str())
        {
            return Err(Failure::configuration(super::REIMPORT_REQUIRED));
        }
        fields.push(Field {
            name,
            selector: Selector::parse(selector)
                .map_err(|_| Failure::configuration(super::REIMPORT_REQUIRED))?,
            attribute: attribute.as_deref(),
        });
    }
    Ok(fields)
}

fn request_url(run: &ManualSearchRun, plan: &CardigannPlan) -> Result<Url, Failure> {
    if !matches!(run.torznab_mode.as_deref(), None | Some("generic"))
        || run.identifier.is_some()
        || run.season_number.is_some()
        || run.episode_number.is_some()
    {
        return Err(Failure::configuration(UNSUPPORTED));
    }
    // Path and origin come from the same atomic persisted rule snapshot.
    let origin = Url::parse(&plan.origin).map_err(|_| INVALID)?;
    if !matches!(origin.scheme(), "http" | "https")
        || origin.host_str().is_none()
        || private_url(Some(origin.as_str()), "")
        || origin.fragment().is_some()
        || plan.search_path.contains("{{")
    {
        return Err(Failure::configuration(super::REIMPORT_REQUIRED));
    }
    let mut url = origin.join(&plan.search_path).map_err(|_| INVALID)?;
    if url.origin() != origin.origin()
        || url.fragment().is_some()
        || plan.input_names.len() != plan.input_values.len()
        || plan.input_names.len() > 32
    {
        return Err(Failure::configuration(super::REIMPORT_REQUIRED));
    }
    for (name, value) in plan.input_names.iter().zip(&plan.input_values) {
        let value = substitute(value, &run.query_text)
            .map_err(|_| Failure::configuration(super::REIMPORT_REQUIRED))?;
        // Cardigann omits empty inputs unless its unsupported allowEmptyInputs feature is enabled.
        if !value.is_empty() {
            url.query_pairs_mut().append_pair(name, &value);
        }
    }
    Ok(url)
}

pub(crate) async fn fetch(
    client: &Client,
    run: &ManualSearchRun,
    plan: &CardigannPlan,
) -> Result<Vec<Release>, Failure> {
    let fields = fields(plan)?;
    let rows = Selector::parse(&plan.rows_selector)
        .map_err(|_| Failure::configuration(super::REIMPORT_REQUIRED))?;
    let url = request_url(run, plan)?;
    let text = fetch_text(client, url.clone(), run.read_timeout_ms).await?;
    parse(&text, &url, plan, &rows, &fields)
}

fn values(row: ElementRef<'_>, fields: &[Field<'_>]) -> Result<BTreeMap<String, String>, Failure> {
    fields
        .iter()
        .map(|field| {
            let selected = row.select(&field.selector).next().ok_or(INVALID)?;
            let text = match field.attribute {
                Some(attribute) => selected.value().attr(attribute).ok_or(INVALID)?.to_owned(),
                None => selected.text().collect::<String>(),
            };
            let text = text.trim();
            if text.is_empty() {
                return Err(INVALID);
            }
            Ok((field.name.to_owned(), text.to_owned()))
        })
        .collect()
}

fn valid_hash(value: &str) -> bool {
    value.strip_prefix("urn:btih:").is_some_and(|hash| {
        (hash.len() == 40 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
            || (hash.len() == 32
                && hash
                    .bytes()
                    .all(|byte| matches!(byte.to_ascii_uppercase(), b'A'..=b'Z' | b'2'..=b'7')))
    }) || value
        .strip_prefix("urn:btmh:1220")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn link(value: Option<&String>, base: &Url, magnet: bool) -> Result<Option<String>, Failure> {
    value
        .map(|value| {
            let url = base.join(value).map_err(|_| INVALID)?;
            if if magnet {
                url.scheme() != "magnet"
            } else {
                !matches!(url.scheme(), "http" | "https")
            } {
                return Err(INVALID);
            }
            if magnet
                && !url
                    .query_pairs()
                    .any(|(key, value)| key == "xt" && valid_hash(&value))
            {
                return Err(INVALID);
            }
            // Keep original upstream data for acquisition; the existing privacy gate controls presentation.
            Ok(url.into())
        })
        .transpose()
}

fn count(value: Option<&String>) -> Result<Option<i32>, Failure> {
    value
        .map(|value| {
            value
                .parse::<i32>()
                .ok()
                .filter(|value| *value >= 0)
                .ok_or(INVALID)
        })
        .transpose()
}

fn parse(
    text: &str,
    base: &Url,
    plan: &CardigannPlan,
    rows: &Selector,
    fields: &[Field<'_>],
) -> Result<Vec<Release>, Failure> {
    let document = Html::parse_document(text);
    if document.tree.nodes().count() > 50_000 {
        return Err(INVALID);
    }
    let mut releases = Vec::new();
    for row in document.select(rows) {
        if releases.len() == 1000 {
            return Err(INVALID);
        }
        let values = values(row, fields)?;
        let title = values.get("title").ok_or(INVALID)?.to_owned();
        let size = values
            .get("size")
            .ok_or(INVALID)?
            .parse::<i64>()
            .ok()
            .filter(|value| *value >= 0)
            .ok_or(INVALID)?;
        let details_url = link(values.get("details"), base, false)?;
        let download_url = link(values.get("download"), base, false)?;
        let magnet = link(values.get("magnet"), base, true)?;
        if details_url.is_none() && download_url.is_none() && magnet.is_none() {
            return Err(INVALID);
        }
        let tracker_category = values
            .get("category")
            .map(|source| {
                plan.category_sources
                    .iter()
                    .zip(&plan.category_targets)
                    .find(|(key, _)| *key == source)
                    .map(|(_, value)| *value)
                    .ok_or(INVALID)
            })
            .transpose()?;
        let published_at = values
            .get("date")
            .map(|value| {
                DateTime::parse_from_rfc3339(value)
                    .map(|value| value.with_timezone(&Utc))
                    .map_err(|_| INVALID)
            })
            .transpose()?;
        releases.push(Release {
            privacy: ManualSearchLinkPrivacy {
                details: private_url(details_url.as_deref(), ""),
                download: private_url(download_url.as_deref(), ""),
                magnet: private_url(magnet.as_deref(), ""),
            },
            title,
            guid: None,
            details_url,
            download_url,
            magnet,
            infohash: None,
            size,
            seeders: count(values.get("seeders"))?,
            leechers: count(values.get("leechers"))?,
            published_at,
            tracker_category,
        });
    }
    Ok(releases)
}

#[cfg(test)]
mod tests;
