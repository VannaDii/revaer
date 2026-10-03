"""Load existing scanner overrides once, without exposing tokens in diagnostics."""

from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import urlsplit

from ..errors import ToolingError


@dataclass(frozen=True)
class SonarSettings:
    project_key: str
    auth_token: str = field(repr=False)
    scanner_token: str = field(repr=False)
    api_base: str
    api_attempts: int
    api_delay: int
    result_attempts: int
    result_delay: int
    coverage: Path
    scanner_log: Path
    report_task: Path
    scm_evidence: Path
    result_evidence: Path
    base_sha: str
    base_ref: str
    head_sha: str
    pull_request: str
    native_required: bool
    install_root: Path | None
    binaries_url: str
    version_override: str | None
    flavor_override: str | None


def _integer(environment: Mapping[str, str], key: str, default: int, minimum: int) -> int:
    value = environment.get(key) or str(default)
    if not value.isdecimal() or int(value) < minimum:
        raise ToolingError(f"{key} must be an integer no smaller than {minimum}")
    return int(value)


def load_sonar_settings(environment: Mapping[str, str], *, linux: bool) -> SonarSettings:
    api_base = environment.get("SONAR_API_BASE_URL") or "https://sonarcloud.io/api"
    url = urlsplit(api_base)
    if (
        not url.hostname
        or url.username is not None
        or url.password is not None
        or url.query
        or url.fragment
        or not (
            url.scheme == "https"
            or (url.scheme == "http" and url.hostname in ("127.0.0.1", "::1", "localhost"))
        )
    ):
        raise ToolingError("SONAR_API_BASE_URL must use HTTPS or an explicit local test endpoint")
    required = environment.get("REVAER_REQUIRE_NATIVE_COVERAGE")
    if required is None or required == "":
        required = "1" if environment.get("CI") == "true" and linux else "0"
    if required not in ("0", "1"):
        raise ToolingError("REVAER_REQUIRE_NATIVE_COVERAGE must be 0 or 1")
    return SonarSettings(
        project_key=environment.get("SONAR_PROJECT_KEY") or "VannaDii_Revaer",
        auth_token=environment.get("SONAR_AUTH_TOKEN") or environment.get("SONAR_TOKEN") or "",
        scanner_token=environment.get("SONAR_TOKEN") or "",
        api_base=api_base.rstrip("/"),
        api_attempts=_integer(environment, "SONAR_API_RETRY_ATTEMPTS", 5, 1),
        api_delay=_integer(environment, "SONAR_API_RETRY_DELAY_SECONDS", 3, 0),
        result_attempts=_integer(environment, "SONAR_RESULT_RETRY_ATTEMPTS", 10, 1),
        result_delay=_integer(environment, "SONAR_RESULT_RETRY_DELAY_SECONDS", 3, 0),
        coverage=Path(environment.get("SONAR_COVERAGE_ROOT") or "coverage"),
        scanner_log=Path(environment.get("SONAR_SCANNER_LOG") or "artifacts/sonar/scanner.log"),
        report_task=Path(
            environment.get("SONAR_REPORT_TASK_PATH") or ".scannerwork/report-task.txt"
        ),
        scm_evidence=Path(
            environment.get("SONAR_SCM_EVIDENCE_PATH") or "artifacts/sonar/scm-evidence.txt"
        ),
        result_evidence=Path(environment.get("SONAR_RESULT_EVIDENCE_DIR") or "artifacts/sonar/api"),
        base_sha=environment.get("SONAR_BASE_SHA") or "",
        base_ref=environment.get("SONAR_BASE_REF") or "",
        head_sha=environment.get("SONAR_HEAD_SHA") or "",
        pull_request=environment.get("SONAR_PULL_REQUEST") or "",
        native_required=required == "1",
        install_root=Path(environment["SONAR_SCANNER_INSTALL_ROOT"])
        if environment.get("SONAR_SCANNER_INSTALL_ROOT")
        else None,
        binaries_url=environment.get("SONAR_SCANNER_BINARIES_URL")
        or "https://binaries.sonarsource.com/Distribution/sonar-scanner-cli",
        version_override=environment.get("SONAR_SCANNER_VERSION") or None,
        flavor_override=environment.get("SONAR_SCANNER_FLAVOR") or None,
    )
