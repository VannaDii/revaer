//! Recent-feed identifiers; RSS does not require search-release metadata.

use reqwest::Url;

use crate::manual_search::torznab::{Failure, document, private_url};

const ATOM: &str = "http://www.w3.org/2005/Atom";
const TORZNAB: &str = "http://torznab.com/schemas/2015/feed";

pub(crate) struct Item {
    pub guid: String,
    pub infohash_v1: String,
    pub infohash_v2: String,
    pub magnet: Option<String>,
}

pub(crate) fn request_url(endpoint: &str, api_key: &str) -> Result<Url, Failure> {
    let mut url = Url::parse(endpoint).map_err(|_| Failure::classified("unknown"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(Failure::classified("unknown"));
    }
    let retained: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| !matches!(key.as_ref(), "t" | "q" | "apikey" | "offset"))
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    url.set_query(None);
    url.query_pairs_mut()
        .extend_pairs(retained)
        .append_pair("t", "search")
        .append_pair("q", "")
        .append_pair("apikey", api_key);
    Ok(url)
}

pub(crate) fn parse(text: &str, api_key: &str) -> Result<Vec<Item>, Failure> {
    let invalid = Failure::classified("parse_error");
    let document = document(text)?;
    let root = document.root_element();
    if root.has_tag_name("error") {
        return Err(Failure::classified(match root.attribute("code") {
            Some("100" | "101" | "102") => "auth_error",
            _ => "parse_error",
        }));
    }
    let atom = root.has_tag_name((ATOM, "feed"));
    let parent = if atom {
        root
    } else if root.has_tag_name("rss") {
        root.children()
            .find(|node| node.has_tag_name("channel"))
            .ok_or(invalid)?
    } else {
        return Err(invalid);
    };
    let mut items = Vec::new();
    for node in parent.children().filter(|node| {
        if atom {
            node.has_tag_name((ATOM, "entry"))
        } else {
            node.has_tag_name("item")
        }
    }) {
        if items.len() == 1000 {
            return Err(invalid);
        }
        items.push(parse_item(node, atom, api_key));
    }
    Ok(items)
}

fn parse_item(node: roxmltree::Node<'_, '_>, atom: bool, api_key: &str) -> Item {
    let text = |name| {
        node.children()
            .find(|child| child.tag_name().name() == name)
            .and_then(|child| child.text())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let attr = |name| {
        node.children()
            .find(|child| {
                child.has_tag_name((TORZNAB, "attr")) && child.attribute("name") == Some(name)
            })
            .and_then(|child| child.attribute("value"))
    };
    let link = if atom {
        node.children()
            .find(|child| {
                child.has_tag_name((ATOM, "link"))
                    && matches!(child.attribute("rel"), None | Some("alternate"))
            })
            .and_then(|child| child.attribute("href"))
    } else {
        text("link")
    };
    let magnet = attr("magneturl")
        .or_else(|| link.filter(|link| link.starts_with("magnet:?")))
        .or_else(|| {
            node.children()
                .find(|child| child.has_tag_name("enclosure"))
                .and_then(|child| child.attribute("url"))
                .filter(|link| link.starts_with("magnet:?"))
        });
    let guid = text(if atom { "id" } else { "guid" })
        .or(link)
        .unwrap_or("");
    let guid = if guid.chars().count() > 256
        || (!api_key.is_empty() && guid.contains(api_key))
        || (guid.contains("://") && private_url(Some(guid), api_key))
    {
        ""
    } else {
        guid
    };
    let hash = |value: Option<&str>, width| {
        value
            .filter(|value| {
                value.len() == width && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    Item {
        guid: guid.to_string(),
        infohash_v1: hash(attr("infohash"), 40),
        infohash_v2: hash(attr("infohashv2"), 64),
        magnet: magnet.map(ToString::to_string),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rss_identifiers_do_not_require_search_metadata() -> Result<(), Failure> {
        let items = parse(
            "<rss><channel><item><guid>opaque</guid></item><item/></channel></rss>",
            "",
        )?;
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].guid, "opaque");
        assert_eq!(items[1].guid, "");
        assert!(parse("<rss><channel/></rss>", "")?.is_empty());
        let long = format!(
            "<rss><channel><item><guid>{}</guid></item></channel></rss>",
            "x".repeat(257)
        );
        assert!(parse(&long, "")?[0].guid.is_empty());
        Ok(())
    }

    #[test]
    fn atom_recent_items_and_endpoint_parameters() -> Result<(), Failure> {
        let items = parse(
            r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry><id>atom-id</id><link href="https://example.test/item"/></entry></feed>"#,
            "",
        )?;
        assert_eq!(items[0].guid, "atom-id");
        let url = request_url(
            "https://example.test/api?cat=2000&t=tvsearch&q=old&offset=100",
            &uuid::Uuid::new_v4().to_string(),
        )?;
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(pairs.get("t").map(AsRef::as_ref), Some("search"));
        assert_eq!(pairs.get("q").map(AsRef::as_ref), Some(""));
        assert_eq!(pairs.get("cat").map(AsRef::as_ref), Some("2000"));
        assert!(!pairs.contains_key("offset"));
        Ok(())
    }

    #[test]
    fn rss_private_guids_and_invalid_hashes_never_escape_to_status() -> Result<(), Failure> {
        let key = uuid::Uuid::new_v4().to_string();
        let feed = format!(
            r#"<rss xmlns:torznab="http://torznab.com/schemas/2015/feed"><channel><item><guid>https://example.test/item?apikey={key}</guid><torznab:attr name="infohash" value="invalid"/></item></channel></rss>"#
        );
        let items = parse(&feed, &key)?;
        assert!(items[0].guid.is_empty() && items[0].infohash_v1.is_empty());
        assert!(request_url(&format!("https://user:{key}@example.test/api"), &key).is_err());
        Ok(())
    }

    #[test]
    fn malformed_and_unsafe_xml_fail_closed() {
        for text in [
            "<rss>",
            "<html/>",
            "<!DOCTYPE rss [<!ENTITY x 'value'>]><rss><channel/></rss>",
        ] {
            assert_eq!(
                parse(text, "").err().map(|failure| failure.class),
                Some("parse_error")
            );
        }
    }
}
