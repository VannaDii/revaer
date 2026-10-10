"""Validated browser-run inputs, preserving the existing E2E environment surface."""

import math
import re
from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path
from urllib.parse import urlsplit

from ..errors import ToolingError


class Recording(StrEnum):
    OFF = "off"
    ON = "on"
    FAILURE = "retain-on-failure"
    RETRY = "on-first-retry"

    def enabled(self, attempt: int) -> bool:
        return self in (Recording.ON, Recording.FAILURE) or (
            self == Recording.RETRY and attempt == 2
        )

    def retain(self, failed: bool) -> bool:
        return self != Recording.OFF and (self != Recording.FAILURE or failed)


@dataclass(frozen=True)
class E2eSettings:
    api_url: str
    ui_url: str
    admin_url: str | None = field(repr=False)
    database_prefix: str
    filesystem_root: Path | None
    managed_media_roots: bool
    coverage_directory: Path | None
    skip_database_start: bool
    api_runner: str | None
    headless: bool
    browsers: tuple[str, ...]
    browser_channel: str | None
    projects: tuple[str, ...]
    no_dependencies: bool
    workers: int
    retries: int
    width: int
    height: int
    test_timeout_ms: int
    expect_timeout_ms: int
    action_timeout_ms: int
    navigation_timeout_ms: int
    trace: Recording
    video: Recording
    screenshot: str
    startup_attempts: int
    startup_interval_ms: int
    shard_index: int
    shard_total: int
    browser_coverage: bool

    @property
    def shard_suffix(self) -> str:
        return f"-shard-{self.shard_index}" if self.shard_total > 1 else ""

    def phases(self, *, media: bool = False) -> tuple[str, ...]:
        api = (
            ("api-none-missing-catalog", "api-none", "api-api-key-missing-catalog", "api-api-key")
            if media
            else ("api-none", "api-api-key")
        )
        available = (*api, *(f"ui-{browser}" for browser in self.browsers))
        selected = set(self.projects or available)
        unknown = selected - set(available)
        if unknown:
            raise ToolingError("Unknown E2E projects: " + ", ".join(sorted(unknown)))
        if self.browser_coverage and "ui-chromium" not in selected:
            raise ToolingError("Browser line coverage requires the Chromium phase")
        if not self.no_dependencies:
            if any(project.startswith("ui-") for project in selected):
                selected.add("api-api-key")
            if "api-api-key" in selected:
                selected.add("api-none")
            if media:
                for phase in ("api-none", "api-api-key"):
                    if phase in selected:
                        selected.add(phase + "-missing-catalog")
        return tuple(phase for phase in available if phase in selected)


def integer(environment: Mapping[str, str], name: str, default: int, minimum: int = 1) -> int:
    try:
        value = int(environment.get(name) or default)
    except ValueError as error:
        raise ToolingError(f"{name} must be an integer of at least {minimum}") from error
    if value < minimum:
        raise ToolingError(f"{name} must be an integer of at least {minimum}")
    return value


def boolean(environment: Mapping[str, str], name: str, default: bool) -> bool:
    raw = environment.get(name)
    if raw is None or raw == "":
        return default
    value = raw.lower()
    if value not in ("1", "true", "yes", "on", "0", "false", "no", "off"):
        raise ToolingError(f"{name} must be a boolean")
    return value in ("1", "true", "yes", "on")


def local_http_url(value: str) -> str:
    parsed = urlsplit(value)
    if (
        parsed.scheme != "http"
        or parsed.hostname not in ("127.0.0.1", "localhost", "::1")
        or parsed.username is not None
        or parsed.password is not None
        or parsed.path not in ("", "/")
        or parsed.query
        or parsed.fragment
        or not 1 <= (parsed.port if parsed.port is not None else 80) <= 65535
    ):
        raise ToolingError("The owned E2E services require local HTTP origins without credentials")
    return value.rstrip("/")


def load_e2e_settings(environment: Mapping[str, str]) -> E2eSettings:
    api_url = local_http_url(environment.get("E2E_API_BASE_URL") or "http://localhost:7070")
    ui_url = local_http_url(environment.get("E2E_BASE_URL") or "http://localhost:8080")
    prefix = environment.get("E2E_DB_PREFIX") or "revaer_e2e"
    if re.fullmatch(r"[a-z][a-z0-9_]{0,29}", prefix) is None:
        raise ToolingError(
            "E2E_DB_PREFIX must be a lowercase SQL identifier of at most 30 characters"
        )
    browsers = tuple(
        dict.fromkeys(
            item.strip() for item in (environment.get("E2E_BROWSERS") or "chromium").split(",")
        )
    )
    if not browsers or any(
        browser not in ("chromium", "firefox", "webkit") for browser in browsers
    ):
        raise ToolingError("E2E_BROWSERS must select chromium, firefox, or webkit")
    runner = environment.get("E2E_API_RUNNER") or None
    if runner not in (None, "binary", "lib-test"):
        raise ToolingError("E2E_API_RUNNER must be binary or lib-test")
    try:
        trace = Recording(environment.get("E2E_TRACE") or "on-first-retry")
        video = Recording(environment.get("E2E_VIDEO") or "retain-on-failure")
    except ValueError as error:
        raise ToolingError(
            "E2E_TRACE and E2E_VIDEO must use supported Playwright recording modes"
        ) from error
    screenshot = environment.get("E2E_SCREENSHOT") or "only-on-failure"
    if screenshot not in ("on", "off", "only-on-failure"):
        raise ToolingError("E2E_SCREENSHOT must be on, off, or only-on-failure")
    interval = integer(environment, "E2E_HTTP_WAIT_INTERVAL_MS", 500)
    attempts = integer(
        environment,
        "E2E_HTTP_WAIT_ATTEMPTS",
        math.ceil(integer(environment, "E2E_HTTP_WAIT_SECONDS", 120) * 1000 / interval),
    )
    shard_total = integer(environment, "PLAYWRIGHT_SHARD_TOTAL", 1)
    shard_index = integer(environment, "PLAYWRIGHT_SHARD_INDEX", 1)
    if shard_index > shard_total:
        raise ToolingError("PLAYWRIGHT_SHARD_INDEX must not exceed PLAYWRIGHT_SHARD_TOTAL")
    fs_root = environment.get("E2E_FS_ROOT")
    coverage = environment.get("E2E_COVERAGE_DIR")
    workers = integer(environment, "E2E_UI_WORKERS", 1)
    return E2eSettings(
        api_url=api_url,
        ui_url=ui_url,
        admin_url=environment.get("E2E_DB_ADMIN_URL")
        or environment.get("REVAER_TEST_DATABASE_URL")
        or environment.get("DATABASE_URL"),
        database_prefix=prefix,
        filesystem_root=Path(fs_root) if fs_root else None,
        managed_media_roots=boolean(environment, "E2E_MANAGED_MEDIA_ROOTS", False),
        coverage_directory=Path(coverage) if coverage else None,
        skip_database_start=boolean(environment, "E2E_SKIP_DB_START", False),
        api_runner=runner,
        headless=boolean(environment, "E2E_HEADLESS", True),
        browsers=browsers,
        browser_channel=environment.get("E2E_BROWSER_CHANNEL") or None,
        projects=tuple(
            item
            for item in re.split(r"[,\s]+", environment.get("E2E_PLAYWRIGHT_PROJECTS", ""))
            if item
        ),
        no_dependencies=boolean(environment, "E2E_PLAYWRIGHT_NO_DEPS", False),
        workers=min(workers, integer(environment, "E2E_WORKERS", workers)),
        retries=integer(
            environment, "E2E_RETRIES", 2 if boolean(environment, "CI", False) else 0, 0
        ),
        width=integer(environment, "E2E_VIEWPORT_WIDTH", 1440),
        height=integer(environment, "E2E_VIEWPORT_HEIGHT", 900),
        test_timeout_ms=integer(environment, "E2E_TEST_TIMEOUT_MS", 30000, 0),
        expect_timeout_ms=integer(environment, "E2E_EXPECT_TIMEOUT_MS", 5000, 0),
        action_timeout_ms=integer(environment, "E2E_ACTION_TIMEOUT_MS", 10000, 0),
        navigation_timeout_ms=integer(environment, "E2E_NAVIGATION_TIMEOUT_MS", 15000, 0),
        trace=trace,
        video=video,
        screenshot=screenshot,
        startup_attempts=attempts,
        startup_interval_ms=interval,
        shard_index=shard_index,
        shard_total=shard_total,
        browser_coverage=boolean(environment, "E2E_BROWSER_COVERAGE", False),
    )
