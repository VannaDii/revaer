//! Validate the real Cardigann structure before publishing any executable state.

use std::collections::BTreeMap;

use reqwest::Url;
use revaer_data::indexers::cardigann::CardigannPlan;
use scraper::Selector;
use serde_yaml::{Mapping, Value};

const UNSUPPORTED: &str = "cardigann_requires_supported_public_get_html_rules";
const INVALID: &str = "cardigann_search_rules_invalid";

fn mapping(value: &Value) -> Result<&Mapping, &'static str> {
    value.as_mapping().ok_or(INVALID)
}

fn keys(value: &Mapping, allowed: &[&str]) -> Result<(), &'static str> {
    if value
        .keys()
        .any(|key| !key.as_str().is_some_and(|key| allowed.contains(&key)))
    {
        return Err(UNSUPPORTED);
    }
    Ok(())
}

fn string(value: Option<&Value>) -> Result<String, &'static str> {
    match value {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Number(number)) => Ok(number.to_string()),
        Some(Value::Bool(value)) => Ok(value.to_string()),
        _ => Err(INVALID),
    }
}

fn required(value: &Mapping, key: &str) -> Result<String, &'static str> {
    let text = string(value.get(key))?;
    if text.trim().is_empty() || text.len() > 2048 || text.contains('\0') {
        return Err(INVALID);
    }
    Ok(text)
}

fn selector(value: &Mapping) -> Result<String, &'static str> {
    let text = required(value, "selector")?;
    if text.len() > 512 || Selector::parse(&text).is_err() {
        return Err(INVALID);
    }
    Ok(text)
}

/// Substitute only the supported keyword variable; all other template syntax fails closed.
pub(super) fn substitute(value: &str, keywords: &str) -> Result<String, &'static str> {
    let mut remaining = value;
    let mut output = String::new();
    while let Some((prefix, suffix)) = remaining.split_once("{{") {
        if prefix.contains("}}") {
            return Err(UNSUPPORTED);
        }
        output.push_str(prefix);
        let (variable, rest) = suffix.split_once("}}").ok_or(UNSUPPORTED)?;
        if variable.trim() != ".Keywords" {
            return Err(UNSUPPORTED);
        }
        output.push_str(keywords);
        remaining = rest;
    }
    if remaining.contains("}}") {
        return Err(UNSUPPORTED);
    }
    output.push_str(remaining);
    Ok(output)
}

fn inputs(
    value: Option<&Value>,
    output: &mut BTreeMap<String, String>,
) -> Result<(), &'static str> {
    if let Some(value) = value {
        for (name, value) in mapping(value)? {
            let name = name.as_str().ok_or(INVALID)?;
            if name.is_empty()
                || name.len() > 128
                || name.starts_with('$')
                || name.contains("{{")
                || name.contains("}}")
                || matches!(
                    name.to_ascii_lowercase().as_str(),
                    "apikey"
                        | "api_key"
                        | "token"
                        | "access_token"
                        | "password"
                        | "passkey"
                        | "auth"
                )
            {
                return Err(UNSUPPORTED);
            }
            let value = string(Some(value))?;
            if value.len() > 2048 || value.contains('\0') {
                return Err(INVALID);
            }
            substitute(&value, "keywords")?;
            output.insert(name.to_owned(), value);
        }
    }
    if output.len() > 32 {
        return Err(INVALID);
    }
    Ok(())
}

/// Category names use the upstream Newznab vocabulary, mapped into our existing catalog.
fn category(value: &str) -> Result<i32, &'static str> {
    let id = match value {
        "Movies" => 2000,
        "Movies/Foreign" => 2010,
        "Movies/Other" => 2020,
        "Movies/SD" => 2030,
        "Movies/HD" => 2040,
        "Movies/UHD" => 2045,
        "Movies/BluRay" => 2050,
        "Movies/3D" => 2060,
        "Audio" => 3000,
        "Audio/MP3" => 3010,
        "Audio/Video" => 3020,
        "PC" => 4000,
        "PC/Games" => 4050,
        "TV" => 5000,
        "TV/WEB-DL" => 5010,
        "TV/Foreign" => 5020,
        "TV/SD" => 5030,
        "TV/HD" => 5040,
        "TV/UHD" => 5045,
        "TV/Other" => 5050,
        "TV/Sport" => 5060,
        "TV/Anime" => 5070,
        "TV/Documentary" => 5080,
        "XXX" => 6000,
        "XXX/DVD" => 6010,
        "XXX/WMV" => 6020,
        "XXX/XviD" => 6030,
        "XXX/x264" => 6040,
        "Books" => 7000,
        "Books/Mags" => 7010,
        "Books/EBook" => 7020,
        "Other" => 8000,
        _ => return Err("cardigann_category_not_in_supported_catalog"),
    };
    Ok(id)
}

fn categories(caps: &Mapping, plan: &mut CardigannPlan) -> Result<(), &'static str> {
    if let Some(value) = caps.get("categorymappings") {
        let mappings = value.as_sequence().ok_or(INVALID)?;
        if mappings.len() > 256 {
            return Err(INVALID);
        }
        for item in mappings {
            let item = mapping(item)?;
            keys(item, &["id", "cat", "desc", "default"])?;
            if item
                .get("default")
                .is_some_and(|value| value != &Value::Bool(false))
            {
                return Err(UNSUPPORTED);
            }
            let source = required(item, "id")?;
            if source.len() > 128 || plan.category_sources.contains(&source) {
                return Err(INVALID);
            }
            plan.category_targets
                .push(category(&required(item, "cat")?)?);
            plan.category_sources.push(source);
        }
    }
    Ok(())
}

pub(crate) fn prepare(yaml: &str) -> Result<Option<CardigannPlan>, &'static str> {
    if yaml.len() > 256 * 1024 {
        return Err(INVALID);
    }
    let value: Value = serde_yaml::from_str(yaml).map_err(|_| INVALID)?;
    let root = mapping(&value)?;
    let Some(search) = root.get("search") else {
        return Ok(None);
    };
    keys(
        root,
        &[
            "id",
            "name",
            "description",
            "language",
            "type",
            "encoding",
            "links",
            "settings",
            "caps",
            "search",
        ],
    )?;
    if root.get("type").and_then(Value::as_str) != Some("public")
        || root
            .get("encoding")
            .is_some_and(|value| value.as_str() != Some("UTF-8"))
        || root
            .get("settings")
            .is_some_and(|value| !value.as_sequence().is_some_and(Vec::is_empty))
    {
        return Err(UNSUPPORTED);
    }
    let origin = origin(root)?;
    let search = mapping(search)?;
    keys(search, &["paths", "inputs", "rows", "fields"])?;
    let (search_path, query) = path_inputs(search)?;
    let rows = mapping(search.get("rows").ok_or(INVALID)?)?;
    keys(rows, &["selector"])?;
    let mut plan = CardigannPlan {
        origin,
        search_path,
        rows_selector: selector(rows)?,
        input_names: query.keys().cloned().collect(),
        input_values: query.into_values().collect(),
        field_names: Vec::new(),
        field_selectors: Vec::new(),
        field_attributes: Vec::new(),
        category_sources: Vec::new(),
        category_targets: Vec::new(),
    };
    extract_fields(search, &mut plan)?;
    capabilities(root, &mut plan)?;
    Ok(Some(plan))
}

fn origin(root: &Mapping) -> Result<String, &'static str> {
    let links = root
        .get("links")
        .and_then(Value::as_sequence)
        .ok_or(INVALID)?;
    let origin = string(links.first())?;
    // Every declared link must be public and static, even though this subset uses the first.
    for link in links {
        let link = string(Some(link))?;
        let url = Url::parse(&link).map_err(|_| INVALID)?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || !link.ends_with('/')
            || link.contains("{{")
            || link.len() > 2048
        {
            return Err(UNSUPPORTED);
        }
    }
    Ok(origin)
}

fn path_inputs(search: &Mapping) -> Result<(String, BTreeMap<String, String>), &'static str> {
    let paths = search
        .get("paths")
        .and_then(Value::as_sequence)
        .ok_or(INVALID)?;
    if paths.len() != 1 {
        return Err(UNSUPPORTED);
    }
    let path = mapping(&paths[0])?;
    keys(path, &["path", "method", "inputs", "inheritinputs"])?;
    if path.get("method").is_some_and(|value| {
        !value
            .as_str()
            .is_some_and(|value| value.eq_ignore_ascii_case("get"))
    }) {
        return Err(UNSUPPORTED);
    }
    let search_path = required(path, "path")?;
    if search_path.contains("{{")
        || search_path.contains("}}")
        || search_path.contains('?')
        || search_path.contains('#')
        || search_path.starts_with("//")
        || Url::parse(&search_path).is_ok()
    {
        return Err(UNSUPPORTED);
    }
    let mut query = BTreeMap::new();
    match path.get("inheritinputs") {
        None | Some(Value::Bool(true)) => inputs(search.get("inputs"), &mut query)?,
        Some(Value::Bool(false)) => {}
        Some(_) => return Err(INVALID),
    }
    // Validate inherited inputs even when the path explicitly opts out of using them.
    let mut validated = BTreeMap::new();
    inputs(search.get("inputs"), &mut validated)?;
    inputs(path.get("inputs"), &mut query)?;
    Ok((search_path, query))
}

fn extract_fields(search: &Mapping, plan: &mut CardigannPlan) -> Result<(), &'static str> {
    let fields = mapping(search.get("fields").ok_or(INVALID)?)?;
    keys(
        fields,
        &[
            "title", "details", "download", "magnet", "category", "size", "seeders", "leechers",
            "date",
        ],
    )?;
    for (name, value) in fields {
        let field = mapping(value)?;
        keys(field, &["selector", "attribute"])?;
        plan.field_names
            .push(name.as_str().ok_or(INVALID)?.to_owned());
        plan.field_selectors.push(selector(field)?);
        let attribute = field
            .get("attribute")
            .map(|value| string(Some(value)))
            .transpose()?;
        if attribute
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > 128 || value.contains('\0'))
        {
            return Err(INVALID);
        }
        plan.field_attributes.push(attribute);
    }
    if !["title", "size"]
        .iter()
        .all(|name| plan.field_names.iter().any(|field| field == name))
        || !["details", "download", "magnet"]
            .iter()
            .any(|name| plan.field_names.iter().any(|field| field == name))
    {
        return Err(INVALID);
    }
    Ok(())
}

fn capabilities(root: &Mapping, plan: &mut CardigannPlan) -> Result<(), &'static str> {
    let caps = mapping(root.get("caps").ok_or(INVALID)?)?;
    keys(caps, &["modes", "categorymappings"])?;
    let modes = mapping(caps.get("modes").ok_or(INVALID)?)?;
    keys(modes, &["search"])?;
    if modes.get("search") != Some(&Value::Sequence(vec![Value::String("q".to_owned())])) {
        return Err(UNSUPPORTED);
    }
    categories(caps, plan)?;
    if plan.field_names.iter().any(|name| name == "category") && plan.category_sources.is_empty() {
        return Err(INVALID);
    }
    Ok(())
}
