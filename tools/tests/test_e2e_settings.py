"""Browser selection, phase dependencies, recording policy, and validated overrides."""

import pytest
from revaer_tooling.e2e.settings import Recording, load_e2e_settings
from revaer_tooling.errors import ToolingError


def test_defaults_and_project_dependencies_preserve_order() -> None:
    default = load_e2e_settings({})
    assert default.phases() == ("api-none", "api-api-key", "ui-chromium")
    assert default.workers == 1 and default.retries == 0
    assert default.trace == Recording.RETRY
    selected = load_e2e_settings(
        {
            "E2E_BROWSERS": "webkit,chromium,webkit",
            "E2E_PLAYWRIGHT_PROJECTS": "ui-webkit",
            "E2E_UI_WORKERS": "4",
            "E2E_WORKERS": "2",
            "CI": "true",
        }
    )
    assert selected.phases() == ("api-none", "api-api-key", "ui-webkit")
    assert selected.workers == 2 and selected.retries == 2
    diagnostic = load_e2e_settings(
        {
            "E2E_PLAYWRIGHT_PROJECTS": "ui-chromium",
            "E2E_PLAYWRIGHT_NO_DEPS": "true",
        }
    )
    assert diagnostic.phases() == ("ui-chromium",)


@pytest.mark.parametrize(
    "environment",
    [
        {"E2E_BROWSERS": "chormium"},
        {"E2E_BROWSERS": ","},
        {"E2E_RETRIES": "-1"},
        {"E2E_UI_WORKERS": "0"},
        {"E2E_WORKERS": "many"},
        {"E2E_HEADLESS": "maybe"},
        {"E2E_BASE_URL": "https://external.invalid"},
        {"E2E_BASE_URL": "http://127.0.0.1:0"},
        {"E2E_BASE_URL": "http://127.0.0.1/?key=value"},
        {"E2E_API_RUNNER": "guess"},
        {"E2E_TRACE": "unknown"},
        {"E2E_VIDEO": "unknown"},
        {"E2E_SCREENSHOT": "unknown"},
        {"E2E_DB_PREFIX": "unsafe-name"},
        {"PLAYWRIGHT_SHARD_TOTAL": "2", "PLAYWRIGHT_SHARD_INDEX": "3"},
    ],
)
def test_invalid_settings_fail_before_any_service_is_started(environment: dict[str, str]) -> None:
    with pytest.raises(ToolingError):
        load_e2e_settings(environment)


def test_recording_modes_preserve_first_retry_and_failure_semantics() -> None:
    assert [Recording.RETRY.enabled(attempt) for attempt in (1, 2, 3)] == [False, True, False]
    assert Recording.ON.enabled(1) and Recording.ON.retain(False)
    assert Recording.FAILURE.enabled(1) and not Recording.FAILURE.retain(False)
    assert Recording.FAILURE.retain(True)
    assert not Recording.OFF.enabled(1) and not Recording.OFF.retain(True)


def test_shards_and_timeouts_are_explicit() -> None:
    settings = load_e2e_settings(
        {
            "PLAYWRIGHT_SHARD_TOTAL": "4",
            "PLAYWRIGHT_SHARD_INDEX": "2",
            "E2E_HTTP_WAIT_SECONDS": "3",
            "E2E_HTTP_WAIT_INTERVAL_MS": "400",
            "E2E_TEST_TIMEOUT_MS": "0",
            "E2E_HEADLESS": "false",
            "E2E_API_RUNNER": "lib-test",
            "E2E_FS_ROOT": "relative/root",
            "E2E_COVERAGE_DIR": "artifacts/coverage",
        }
    )
    assert settings.shard_suffix == "-shard-2" and settings.startup_attempts == 8
    assert settings.test_timeout_ms == 0 and not settings.headless
    assert settings.api_runner == "lib-test"
    with pytest.raises(ToolingError, match="Unknown E2E projects"):
        load_e2e_settings({"E2E_PLAYWRIGHT_PROJECTS": "unknown"}).phases()


def test_browser_line_coverage_requires_chromium_and_keeps_other_browsers() -> None:
    settings = load_e2e_settings(
        {"E2E_BROWSER_COVERAGE": "1", "E2E_BROWSERS": "chromium,firefox,webkit"}
    )
    assert settings.phases() == (
        "api-none",
        "api-api-key",
        "ui-chromium",
        "ui-firefox",
        "ui-webkit",
    )
    with pytest.raises(ToolingError, match="requires the Chromium phase"):
        load_e2e_settings({"E2E_BROWSER_COVERAGE": "1", "E2E_BROWSERS": "firefox"}).phases()
