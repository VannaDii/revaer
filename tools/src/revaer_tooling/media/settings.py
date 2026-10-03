"""Resolve media overrides once at the CLI boundary."""

from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError


@dataclass(frozen=True)
class FixtureSettings:
    lock: Path
    manifest: Path
    probes: Path
    report: Path
    force_download: bool
    force_generate: bool
    connect_timeout: int
    deadline: int


def load_fixture_settings(environment: Mapping[str, str]) -> FixtureSettings:
    def flag(name: str) -> bool:
        value = environment.get(name) or "0"
        if value not in ("0", "1"):
            raise ToolingError(f"{name} must be 0 or 1")
        return value == "1"

    def seconds(name: str, default: int) -> int:
        try:
            value = int(environment.get(name) or default)
        except ValueError as error:
            raise ToolingError(f"{name} must be a positive integer") from error
        if value < 1:
            raise ToolingError(f"{name} must be a positive integer")
        return value

    return FixtureSettings(
        lock=Path(environment.get("REVAER_FIXTURE_LOCK_PATH") or "test-fixtures/lock.json"),
        manifest=Path(
            environment.get("REVAER_FIXTURE_MANIFEST_PATH") or "test-fixtures/manifest.json"
        ),
        probes=Path(environment.get("REVAER_FIXTURE_PROBE_DIR") or "test-fixtures/probe"),
        report=Path(
            environment.get("REVAER_MEDIA_CONVERSION_REPORT") or "target/media-conversion-report.md"
        ),
        force_download=flag("REVAER_FIXTURE_FORCE_DOWNLOAD"),
        force_generate=flag("REVAER_FIXTURE_FORCE_GENERATE"),
        connect_timeout=seconds("REVAER_FIXTURE_CONNECT_TIMEOUT_SECONDS", 15),
        deadline=seconds("REVAER_FIXTURE_DEADLINE_SECONDS", 120),
    )
