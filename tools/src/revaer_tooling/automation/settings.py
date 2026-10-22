"""Runner-provided inputs, loaded once at the CLI boundary."""

from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError

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
    ref: str = ""
    pull_request_author: str = ""
    pull_request_head_repository: str = ""
    ref_protected: bool = False
    dependabot_enabled: bool = False
    dependabot_app_login: str = ""
    dependabot_number: int | None = None


def load_workflow_settings(environment: Mapping[str, str]) -> WorkflowSettings:
    def file(name: str) -> Path | None:
        value = environment.get(name)
        return Path(value) if value else None

    number_text = environment.get("REVAER_DEPENDABOT_PR_NUMBER", "")
    if number_text and (
        not number_text.isascii() or not number_text.isdecimal() or int(number_text) < 1
    ):
        raise ToolingError("REVAER_DEPENDABOT_PR_NUMBER must be a positive integer")
    slug = environment.get("REVAER_DEPENDABOT_APP_SLUG", "")
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
        environment.get("GITHUB_REF", ""),
        environment.get("REVAER_INSTRUCTION_PR_AUTHOR", ""),
        environment.get("REVAER_INSTRUCTION_PR_HEAD_REPOSITORY", ""),
        environment.get("GITHUB_REF_PROTECTED", "") == "true",
        environment.get("REVAER_DEPENDABOT_ENABLED", "") == "true",
        slug + "[bot]" if slug else "",
        int(number_text) if number_text else None,
    )
