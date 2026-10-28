use super::DEFINITION;
use crate::cardigann::definition::{prepare, substitute};

#[test]
fn cardigann_preserves_real_paths_fields_categories_and_literal_inputs() -> anyhow::Result<()> {
    let plan = prepare(DEFINITION)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("Expected executable rules"))?;
    assert_eq!(plan.search_path, "search/releases");
    assert_eq!(plan.category_sources, ["film"]);
    assert_eq!(plan.category_targets, [2000]);
    assert_eq!(plan.input_names, ["order", "term"]);
    assert_eq!(plan.input_values, ["newest", "{{ .Keywords }}"]);
    assert!(plan.field_names.iter().any(|name| name == "date"));
    assert_eq!(
        substitute("prefix {{.Keywords}} / {{ .Keywords }}", "A&B").map_err(anyhow::Error::msg)?,
        "prefix A&B / A&B"
    );
    assert!(
        prepare("id: legacy\nname: Legacy\nsettings: []")
            .map_err(anyhow::Error::msg)?
            .is_none()
    );
    Ok(())
}

#[test]
fn cardigann_rejects_unsupported_execution_without_silently_ignoring_it() {
    for (from, to) in [
        ("type: public", "type: private"),
        ("encoding: UTF-8", "encoding: ISO-8859-1"),
        ("settings: []", "settings: [{name: username, type: text}]"),
        (
            "search:\n  paths:",
            "login: {method: cookie}\nsearch:\n  paths:",
        ),
        (
            "search:\n  paths:",
            "search:\n  keywordsfilters: []\n  paths:",
        ),
        (
            "search:\n  paths:",
            "search:\n  response: {type: json}\n  paths:",
        ),
        ("method: get", "method: post"),
        ("method: get", "method: get\n      followredirect: true"),
        ("path: search/releases", "path: '{{ .Keywords }}'"),
        (
            "path: search/releases",
            "path: https://other.example/search",
        ),
        ("path: search/releases", "path: //other.example/search"),
        (
            "term: \"{{ .Keywords }}\"",
            "term: '{{ if .Query.IMDBID }}x{{end}}'",
        ),
        ("term:", "$raw:"),
        ("term:", "apikey:"),
        (
            "rows:\n    selector:",
            "rows:\n    filters: []\n    selector:",
        ),
        ("selector: a.title", "selector: a.title\n      filters: []"),
        ("search: [q]", "search: [q]\n    movie-search: [imdbid]"),
        ("cat: Movies", "cat: Unrecognized"),
        ("cat: Movies}", "cat: Movies, default: true}"),
    ] {
        assert!(
            prepare(&DEFINITION.replace(from, to)).is_err(),
            "Rejected construct: {to}"
        );
    }
    for expression in [
        "{{.Config.cookie}}",
        "{{.Keywords | urlencode}}",
        "{{",
        "bad }}",
    ] {
        assert!(substitute(expression, "query").is_err());
    }
}

#[test]
fn cardigann_validates_bounds_selectors_and_required_shapes() {
    for (from, to) in [
        ("http://127.0.0.1:8765/", "file:///private/"),
        (
            "http://127.0.0.1:8765/",
            "https://example.test/?token=private",
        ),
        ("table.releases > tbody > tr.release", "table["),
        ("selector: a.title", "selector: ''"),
        ("attribute: href", "attribute: ''"),
        ("title:\n", "unknown:\n"),
        ("size:\n", "poster:\n"),
        ("search: [q]", "search: true"),
        (
            "rows:\n    selector: table.releases > tbody > tr.release",
            "rows: []",
        ),
        ("inputs:\n    term:", "inputs: []\n    term:"),
    ] {
        assert!(prepare(&DEFINITION.replace(from, to)).is_err());
    }
    assert!(prepare(&"x".repeat(256 * 1024 + 1)).is_err());
}

#[test]
fn cardigann_path_inputs_override_or_explicitly_replace_inherited_inputs() -> anyhow::Result<()> {
    let yaml = DEFINITION.replace(
        "method: get",
        "method: get\n      inputs: {order: oldest, page: 1}",
    );
    let plan = prepare(&yaml)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("Expected rules"))?;
    assert_eq!(plan.input_names, ["order", "page", "term"]);
    assert_eq!(plan.input_values, ["oldest", "1", "{{ .Keywords }}"]);
    let yaml = yaml.replace("method: get", "method: get\n      inheritinputs: false");
    let plan = prepare(&yaml)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("Expected rules"))?;
    assert_eq!(plan.input_names, ["order", "page"]);
    Ok(())
}
