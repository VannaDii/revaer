"""Verify case composition cannot approve shared failures or incomplete evidence."""

from copy import deepcopy
from pathlib import Path

import pytest
from fixtures.database import CONTAINER, ProofDocker
from revaer_tooling.database.ingestion.evidence import IngestionEvidence
from revaer_tooling.database.ingestion.isolated import IsolatedIngestion, Variant
from revaer_tooling.database.ingestion.matrix import IngestionMatrix
from revaer_tooling.database.postgres import Connection
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.json_data import JsonObject, decode_unique, object_value, string_value


class ScriptedIsolation(IsolatedIngestion):
    """Native isolation/parsing is tested separately; inject phase outcomes here."""

    def __init__(self, root: Path, failure: str = "") -> None:
        fixture = object_value(
            decode_unique(
                (Path(__file__).parent / "fixtures/ingestion-corrections.json").read_text()
            )
        )
        connection = Connection(
            ProofDocker(root), FileSystem(), CONTAINER, "postgres", "proof", "", None, root
        )
        super().__init__(
            connection,
            "owner",
            "runtime",
            "SELECT 1",
            IngestionEvidence(string_value(fixture["signature"]), "source", "runtime"),
        )
        self.warm = object_value(object_value(fixture["cases"])["D4"])
        self.failure = failure
        self.calls: list[tuple[str, Variant, bool]] = []

    def run(
        self, name: str, query: str, variant: Variant, *, helpers_first: bool = False
    ) -> JsonObject:
        self.calls.append((name, variant, helpers_first))
        assert ("SELECT 'helpers:known';" in query) == helpers_first
        if name == "warm-committed":
            warm = deepcopy(object_value(self.warm[variant.value]))
            if self.failure == "wrong-warm" and variant == Variant.FINAL:
                warm["states"] = ["00000"]
            return warm
        value: JsonObject = {"states": ["P0001" if name.endswith("missing") else "00000"]}
        if name == "new-v1":
            if self.failure == "shared":
                value["states"] = ["P0001"]
            elif variant == Variant.FINAL:
                if self.failure == "different":
                    value["unexpected"] = True
                elif self.failure == "transport":
                    raise ToolingError("native transport failed")
                elif self.failure == "interrupt":
                    raise KeyboardInterrupt("interrupted native case")
        return value


def report(root: Path) -> JsonObject:
    return object_value(decode_unique((root / "ingestion-matrix.json").read_text()))


def test_all_thirteen_cases_require_both_variants_and_do_not_certify_d3(tmp_path: Path) -> None:
    runner = ScriptedIsolation(tmp_path)
    checks: list[bool] = []
    result = IngestionMatrix(
        runner, "SELECT 'helpers:known';", lambda _, passed: checks.append(passed)
    ).run()
    assert len(result) == 13
    assert len(checks) == 26
    assert all(checks)
    assert len(runner.calls) == 26
    assert sum(helpers for _, _, helpers in runner.calls) == 12
    assert object_value(result[-1])["approved_delta"] == "ADR 588 D4"
    assert report(tmp_path)["matrix_passed"] is True
    assert report(tmp_path)["complete"] is False
    assert report(tmp_path)["passed"] is False
    assert (tmp_path / "ingestion-matrix.json").stat().st_mode & 0o777 == 0o600


@pytest.mark.parametrize(
    "failure,reason",
    (
        ("shared", "shared reference/final failure"),
        ("different", "reference/final behavior differs"),
        ("transport", "native transport failed"),
        ("wrong-warm", "reference/final behavior differs"),
    ),
)
def test_failed_case_stops_and_replaces_old_success_receipt(
    tmp_path: Path, failure: str, reason: str
) -> None:
    (tmp_path / "ingestion-matrix.json").write_text('{"matrix_passed":true}')
    runner = ScriptedIsolation(tmp_path, failure)
    with pytest.raises(ToolingError, match=reason):
        IngestionMatrix(runner, "SELECT 'helpers:known';", lambda *_: None).run()
    value = report(tmp_path)
    assert value["matrix_passed"] is False
    assert value["matrix_completed"] is False
    assert reason in string_value(value["stop_reason"])
    assert len(runner.calls) == (26 if failure == "wrong-warm" else 6)


def test_interruption_retains_partial_results_and_never_claims_completion(tmp_path: Path) -> None:
    with pytest.raises(KeyboardInterrupt):
        IngestionMatrix(
            ScriptedIsolation(tmp_path, "interrupt"), "SELECT 'helpers:known';", lambda *_: None
        ).run()
    assert report(tmp_path)["matrix_passed"] is False
    assert "KeyboardInterrupt" in string_value(report(tmp_path)["stop_reason"])
