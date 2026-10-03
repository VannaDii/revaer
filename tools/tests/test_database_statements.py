"""Exercise PostgreSQL's scanner and the frozen UTF-8 prefix contract."""

from importlib import import_module

import pytest
from revaer_tooling.database.statements import Boundary, Statements
from revaer_tooling.errors import ToolingError


def test_native_scanner_preserves_unicode_comments_and_nested_routine_bodies() -> None:
    chunks = (
        b"-- comment ;\nSELECT '\xe9\x9b\xaa;';\t\r \n",
        b"/* outer; /* inner; */ end */ DO $body$ BEGIN PERFORM 1; END $body$;\n",
        b"SELECT \"semi;colon\", 'doubled'';quote';",
        b"SELECT E'escaped\\';quote';\n",
    )
    source = b"".join(chunks)
    parsed = Statements.parse(source)
    expected = []
    total = 0
    for ordinal, chunk in enumerate(chunks, 1):
        total += len(chunk)
        semicolon = source.rfind(b";", 0, total)
        expected.append(Boundary(ordinal, total, source[:semicolon].count(b"\n") + 1))
    assert parsed.boundaries == tuple(expected)
    assert parsed.count == 4
    assert all(parsed.complete_prefix(boundary.byte_count) for boundary in expected)
    assert not parsed.complete_prefix(0) and not parsed.complete_prefix(total - 1)
    assert parsed.mapping().splitlines()[0] == "ordinal\tbyte_count\tline_number"
    assert parsed.mapping().splitlines()[-1] == f"4\t{total}\t{expected[-1].line_number}"


def test_same_line_statement_comment_and_trailing_unfinished_statement() -> None:
    source = b"SELECT 1; SELECT 2; -- trailing comment;\n SELECT 3"
    parsed = Statements.parse(source)
    assert parsed.boundaries == (Boundary(1, 9, 1), Boundary(2, 19, 1))
    # This is a lexical boundary map, not syntax validation or implicit repair.
    assert not parsed.complete_prefix(len(source))


def test_native_postgresql_escape_rules_and_escaped_newline_line_numbers() -> None:
    assert Statements.parse(b"SELECT 'a\\'; SELECT 2;").count == 2
    source = b"SELECT E'escaped\\\nline';\nSELECT 2;\n"
    assert tuple(row.line_number for row in Statements.parse(source).boundaries) == (2, 3)


@pytest.mark.parametrize(
    "source",
    (
        b"",
        b"-- no statements",
        b"SELECT 1",
        b"SELECT 'open",
        b'SELECT "open',
        b"SELECT $tag$open",
        b"SELECT 1; /* open",
        b"SELECT '\xff';",
        b"SELECT 1;\x00 SELECT 2;",
    ),
)
def test_invalid_or_incomplete_inputs_fail_without_echoing_sql(source: bytes) -> None:
    with pytest.raises(ToolingError):
        Statements.parse(source)


@pytest.mark.parametrize(
    "tokens",
    (
        None,
        [None],
        [(0, 100, "ASCII_59", "NO_KEYWORD")],
        [(0, 0, "ASCII_59", "NO_KEYWORD")],
        [(True, 2, "SELECT", "RESERVED")],
    ),
)
def test_untrusted_native_token_shape_cannot_manufacture_boundaries(
    monkeypatch: pytest.MonkeyPatch, tokens: object
) -> None:
    parser = import_module("pglast.parser")
    monkeypatch.setattr(parser, "scan", lambda source: tokens)
    with pytest.raises(ToolingError, match="invalid"):
        Statements.parse(b"SELECT 1;")


def test_wrong_parser_major_and_missing_optional_dependency(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    parser = import_module("pglast.parser")
    monkeypatch.setattr(parser, "get_postgresql_version", lambda: (18, 1))
    with pytest.raises(ToolingError, match="PostgreSQL 16"):
        Statements.parse(b"SELECT 1;")

    def missing(name: str) -> None:
        raise ImportError(name)

    monkeypatch.setattr("revaer_tooling.database.statements.import_module", missing)
    with pytest.raises(ToolingError, match="uv sync --locked --group dev"):
        Statements.parse(b"SELECT 1;")
