"""Reject missing, stale, inconsistent and empty shard evidence before aggregation."""

import json
from pathlib import Path

import pytest
from revaer_tooling.e2e.shards import verify_shards
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem

PHASES = ("api-none", "api-api-key", "ui-chromium")


@pytest.fixture
def evidence(tmp_path: Path) -> Path:
    # Two scenarios on three shards deliberately leave the third shard empty.
    # The full nonempty collection is what makes that absence legitimate.
    for index in (1, 2, 3):
        (tmp_path / f"python-e2e-summary-shard-{index}.json").write_text(
            json.dumps(
                {
                    "status": "passed",
                    "shard": index,
                    "total_shards": 3,
                    "phases": dict.fromkeys(PHASES, "passed"),
                }
            )
        )
        for phase in PHASES:
            for worker in ("gw0", "gw1"):
                (tmp_path / f"selection-{phase}-shard-{index}-{worker}.json").write_text(
                    json.dumps(
                        {
                            "phase": phase,
                            "shard": index,
                            "total_shards": 3,
                            "all": ["test_a", "test_b"],
                            "selected": ["test_a", "test_b"][index - 1 :: 3],
                        }
                    )
                )
            if index < 3:
                kind = "ui" if phase.startswith("ui-") else "api"
                (tmp_path / f"{kind}-coverage-{phase}-shard-{index}-gw0.json").write_text(
                    '["/observed"]'
                )
    return tmp_path


def test_complete_selections_allow_only_expected_empty_shards(evidence: Path) -> None:
    verify_shards(FileSystem(), evidence, PHASES, 3)
    with pytest.raises(ToolingError, match="positive"):
        verify_shards(FileSystem(), evidence, PHASES, 0)


@pytest.mark.parametrize(
    ("name", "content", "diagnostic"),
    [
        ("python-e2e-summary-shard-2.json", None, "completion summary"),
        ("python-e2e-summary-shard-2.json", '{"status":"running"}', "complete every"),
        ("selection-ui-chromium-shard-2-gw0.json", "{", "valid JSON"),
        ("selection-ui-chromium-shard-2-gw0.json", "{}", "array"),
        ("ui-coverage-ui-chromium-shard-2-gw0.json", None, "nonempty"),
        ("ui-coverage-ui-chromium-shard-2-gw0.json", "[]", "nonempty"),
        ("ui-coverage-ui-chromium-shard-2-gw0.json", "{}", "array"),
    ],
)
def test_missing_and_malformed_evidence_fails(
    evidence: Path, name: str, content: str | None, diagnostic: str
) -> None:
    path = evidence / name
    if content is None:
        path.unlink()
    else:
        path.write_text(content)
    with pytest.raises(ToolingError, match=diagnostic):
        verify_shards(FileSystem(), evidence, PHASES, 3)


@pytest.mark.parametrize(
    "changed",
    [
        {"all": []},
        {"all": ["test_b", "test_a"]},
        {"all": ["test_a", "test_a"]},
        {"all": ["test_a", "test_c"]},
        {"selected": ["test_a"]},
        {"phase": "api-none"},
        {"shard": 1},
        {"total_shards": 2},
    ],
)
def test_workers_must_agree_on_the_exact_partition(
    evidence: Path, changed: dict[str, object]
) -> None:
    path = evidence / "selection-ui-chromium-shard-2-gw1.json"
    selection = json.loads(path.read_text())
    selection.update(changed)
    path.write_text(json.dumps(selection))
    with pytest.raises(ToolingError, match=r"Shard|scenarios"):
        verify_shards(FileSystem(), evidence, PHASES, 3)


def test_missing_phase_selection_and_setup_only_coverage_are_rejected(evidence: Path) -> None:
    for path in evidence.glob("selection-ui-chromium-shard-2-*.json"):
        path.unlink()
    with pytest.raises(ToolingError, match="selection evidence"):
        verify_shards(FileSystem(), evidence, PHASES, 3)


def test_setup_requests_cannot_replace_api_test_evidence(evidence: Path) -> None:
    path = evidence / "api-coverage-api-none-shard-1-gw0.json"
    path.rename(path.with_name("api-coverage-api-none-shard-1-setup.json"))
    with pytest.raises(ToolingError, match="nonempty api-none"):
        verify_shards(FileSystem(), evidence, PHASES, 3)
