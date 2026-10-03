"""Compare with original Ruby fixtures; reject widening the D4/D5 exceptions."""

from copy import deepcopy
from pathlib import Path

import pytest
from revaer_tooling.database.ingestion.corrections import ApprovedCorrections, exact
from revaer_tooling.errors import ToolingError
from revaer_tooling.json_data import (
    JsonObject,
    array_value,
    decode_unique,
    object_value,
    string_value,
)


@pytest.fixture
def fixture() -> JsonObject:
    return object_value(
        decode_unique((Path(__file__).parent / "fixtures/ingestion-corrections.json").read_text())
    )


def pair(
    fixture: JsonObject, choice: str
) -> tuple[ApprovedCorrections, JsonObject, JsonObject, str]:
    value = object_value(object_value(fixture["cases"])[choice])
    return (
        ApprovedCorrections(string_value(fixture["signature"])),
        object_value(value["reference"]),
        object_value(value["final"]),
        "warm-committed" if choice == "D4" else "existing-external-id",
    )


def stage(value: JsonObject, choice: str) -> JsonObject:
    return value if choice == "D4" else object_value(value["tested"])


@pytest.mark.parametrize("choice", ("D4", "D5"))
def test_matches_original_ruby_fixture_without_mutating_evidence(
    fixture: JsonObject, choice: str
) -> None:
    gate, reference, final, name = pair(fixture, choice)
    before = deepcopy((reference, final))
    assert gate.match(name, reference, final) == "ADR 588 " + choice
    assert (reference, final) == before
    assert gate.match("unapproved-case", reference, final) is None


@pytest.mark.parametrize("choice", ("D4", "D5"))
@pytest.mark.parametrize("field", ("line", "location", "statement", "routine", "operation", "hint"))
def test_changed_diagnostic_is_not_an_approved_correction(
    fixture: JsonObject, choice: str, field: str
) -> None:
    gate, reference, final, name = pair(fixture, choice)
    diagnostic = object_value(array_value(stage(reference, choice)["diagnostics"])[0])
    diagnostic[field] = 1 if field == "line" else "changed"
    assert gate.match(name, reference, final) is None


@pytest.mark.parametrize("choice", ("D4", "D5"))
@pytest.mark.parametrize("mutation", ("result", "extra-row", "clock", "setting", "bool-as-number"))
def test_other_final_differences_cannot_hide_inside_correction(
    fixture: JsonObject, choice: str, mutation: str
) -> None:
    gate, reference, final, name = pair(fixture, choice)
    tested = stage(final, choice)
    if mutation in ("result", "bool-as-number"):
        row = object_value(array_value(tested["results"])[-1])
        row["canonical_changed"] = True if mutation == "result" else 0
    elif mutation == "extra-row":
        array_value(object_value(tested["after"])["canonical_size_sample"]).append(
            {"unexpected": 1}
        )
    elif mutation == "clock":
        object_value(array_value(object_value(tested["after"])["canonical_torrent"])[0])[
            "updated_at"
        ] = "wrong-clock"
    else:
        tested["caller_settings"] = {"before": ["use_column"], "after": ["use_column"]}
    assert gate.match(name, reference, final) is None


@pytest.mark.parametrize("choice", ("D4", "D5"))
def test_missing_table_inventory_is_an_error(fixture: JsonObject, choice: str) -> None:
    gate, reference, final, name = pair(fixture, choice)
    # D5 shares the reference fixture's after and tested before semantically;
    # update both so the explicit inventory guard, not a prior mismatch, fires.
    for value in (stage(reference, choice)["before"], stage(reference, choice)["after"]):
        object_value(value).pop("canonical_size_sample", None)
    if choice == "D5":
        object_value(object_value(reference["fixture"])["after"]).pop("canonical_size_sample", None)
        final["fixture"] = deepcopy(reference["fixture"])
    with pytest.raises(ToolingError, match="inventory"):
        gate.match(name, reference, final)


def test_json_comparison_preserves_numeric_equality_without_boolean_coercion() -> None:
    assert exact({"value": [1, 2.0]}, {"value": [1.0, 2]})
    assert not exact({"value": [False]}, {"value": [0]})
    assert not exact({"value": [True]}, {"value": [1]})
