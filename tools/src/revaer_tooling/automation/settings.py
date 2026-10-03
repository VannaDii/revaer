"""Runner-provided inputs, loaded once at the CLI boundary."""

from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

SUPPLY_CHAIN_RESULTS = ("AUDIT_RESULT", "DENY_RESULT", "UDEPS_RESULT")


@dataclass(frozen=True)
class WorkflowSettings:
    output: Path | None
    environment: Path | None
    summary: Path | None
    ref_type: str
    ref_name: str
    event_name: str
    run_id: str
    run_number: str
    supply_chain_results: tuple[tuple[str, str], ...]


def load_workflow_settings(environment: Mapping[str, str]) -> WorkflowSettings:
    def file(name: str) -> Path | None:
        value = environment.get(name)
        return Path(value) if value else None

    return WorkflowSettings(
        file("GITHUB_OUTPUT"),
        file("GITHUB_ENV"),
        file("GITHUB_STEP_SUMMARY"),
        environment.get("GITHUB_REF_TYPE", ""),
        environment.get("GITHUB_REF_NAME", ""),
        environment.get("GITHUB_EVENT_NAME", ""),
        environment.get("GITHUB_RUN_ID", ""),
        environment.get("GITHUB_RUN_NUMBER", ""),
        tuple((name, environment.get(name, "")) for name in SUPPLY_CHAIN_RESULTS),
    )
