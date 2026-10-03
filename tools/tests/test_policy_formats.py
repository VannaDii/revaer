"""Exercise ambiguous configuration using the same parsers as the policy gate."""

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.policy.formats import properties, yaml_document


def test_yaml_preserves_actions_keys_scalars_and_nested_containers() -> None:
    assert yaml_document(
        "on: {pull_request: null}\nsteps: [{env: {ENABLED: false, COUNT: 3}}]\n", "fixture.yml"
    ) == {"on": {"pull_request": "null"}, "steps": [{"env": {"ENABLED": "false", "COUNT": "3"}}]}


@pytest.mark.parametrize(
    ("source", "message"),
    [
        ("", "one YAML document"),
        ("---\n", "must be a mapping"),
        ("[]", "must be a mapping"),
        ("one: 1\n---\ntwo: 2\n", "one YAML document"),
        ("key: one\nkey: two", "duplicate key"),
        ("key: {inner: one, inner: two}", "duplicate key"),
        ("? [a, b]\n: value", "non-scalar mapping key"),
        ("first: &same {key: value}\nsecond: *same", "aliases are forbidden"),
        ("cycle: &cycle [*cycle]", "aliases are forbidden"),
        ("merged: {<<: {key: value}}", "merge keys are forbidden"),
        ("unclosed: [", "invalid YAML"),
    ],
)
def test_yaml_rejects_ambiguous_or_invalid_input(source: str, message: str) -> None:
    with pytest.raises(ToolingError, match=message):
        yaml_document(source, "fixture.yml")


def test_properties_obey_java_separators_comments_and_value_escapes() -> None:
    entries = properties(
        "# comment\n  ! comment\n\nalpha = value\nbeta: other\ngamma \t= text\n"
        "empty\nescaped=tr\\u0075e\\tvalue\\\\end\ncolon=:value\n",
        "fixture.properties",
    )
    assert [(entry.key, entry.value, entry.line) for entry in entries] == [
        ("alpha", "value", 4),
        ("beta", "other", 5),
        ("gamma", "text", 6),
        ("empty", "", 7),
        ("escaped", "true\tvalue\\end", 8),
        ("colon", ":value", 9),
    ]


@pytest.mark.parametrize(
    ("source", "message"),
    [
        (" first=value", "leading property whitespace"),
        ("first=one\\\nother=two", "continuations"),
        ("first=value\\", "continuations"),
        ("sonar\\.sources=value", "escaped property keys"),
        ("sonar\\u002Esources=value", "escaped property keys"),
        ("sonar\\:sources=value", "escaped property keys"),
        ("first=one\nfirst: two", "duplicate property"),
        ("first=\\uG123", "invalid Unicode"),
        ("first=\\u12", "invalid Unicode"),
    ],
)
def test_properties_fail_before_ambiguous_settings_can_override(source: str, message: str) -> None:
    with pytest.raises(ToolingError, match=message):
        properties(source, "fixture.properties")
