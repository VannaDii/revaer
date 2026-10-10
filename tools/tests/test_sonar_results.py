"""Result criteria bind one task to coverage, new issues and runtime safety."""

import copy

import pytest
from fixtures.sonar import result_records
from revaer_tooling.errors import ToolingError
from revaer_tooling.json_data import JsonObject, array_value, object_value
from revaer_tooling.sonar.results import published_result, task_id
from revaer_tooling.sonar.settings import load_sonar_settings


@pytest.fixture
def records() -> dict[str, JsonObject]:
    return result_records()


def test_complete_results_and_one_authoritative_task_are_accepted(
    records: dict[str, JsonObject],
) -> None:
    assert task_id("serverUrl=https://sonarcloud.io\nceTaskId=owned-task\n") == "owned-task"
    result = published_result(records, "owned-task", "VannaDii_Revaer")
    assert (result.analysis, result.coverage, result.lines_to_cover) == (
        "owned-analysis",
        "94.1",
        "1000",
    )


@pytest.mark.parametrize(
    "source",
    (
        "",
        "ceTaskId=\n",
        "ceTaskId=one\nceTaskId=two\n",
        "ceTaskId=bad task\n",
        "ceTaskId=../../bad\n",
    ),
)
def test_missing_duplicate_or_invalid_task_identity_is_rejected(source: str) -> None:
    with pytest.raises(ToolingError):
        task_id(source)


@pytest.mark.parametrize(
    ("name", "value"),
    (
        ("id", "other-task"),
        ("componentKey", "other-project"),
        ("status", "PENDING"),
        ("status", "FAILED"),
        ("analysisId", ""),
    ),
)
def test_another_pending_or_failed_analysis_cannot_pass(
    records: dict[str, JsonObject], name: str, value: str
) -> None:
    object_value(records["ce-task"]["task"])[name] = value
    with pytest.raises(ToolingError):
        published_result(records, "owned-task", "VannaDii_Revaer")


@pytest.mark.parametrize("value", ("0", "-1", "NaN", "Infinity", "not-a-number", ""))
def test_invalid_or_nonpositive_coverage_fails(records: dict[str, JsonObject], value: str) -> None:
    measures = array_value(object_value(records["measures"]["component"])["measures"])
    object_value(measures[0])["value"] = value
    with pytest.raises(ToolingError):
        published_result(records, "owned-task", "VannaDii_Revaer")


@pytest.mark.parametrize(
    "case",
    (
        "missing-record",
        "missing-measure",
        "duplicate-measure",
        "failed-gate",
        "ignored-condition",
        "issues",
        "hotspots",
        "boolean-total",
    ),
)
def test_partial_ignored_or_nonempty_backlog_fails(
    records: dict[str, JsonObject], case: str
) -> None:
    if case == "missing-record":
        records.pop("issues")
    elif case in ("missing-measure", "duplicate-measure"):
        measures = array_value(object_value(records["measures"]["component"])["measures"])
        if case == "missing-measure":
            measures.pop()
        else:
            measures.append(copy.deepcopy(measures[0]))
    elif case == "failed-gate":
        object_value(records["quality-gate"]["projectStatus"])["status"] = "ERROR"
    elif case == "ignored-condition":
        object_value(records["quality-gate"]["projectStatus"])["ignoredConditions"] = True
    elif case == "issues":
        records["issues"]["total"] = 1
    elif case == "hotspots":
        object_value(records["hotspots"]["paging"])["total"] = 1
    elif case == "boolean-total":
        records["issues"]["total"] = False
    with pytest.raises(ToolingError):
        published_result(records, "owned-task", "VannaDii_Revaer")


def test_host_defaults_and_credentials_are_explicit() -> None:
    settings = load_sonar_settings(
        {"CI": "true", "SONAR_TOKEN": "private-fixture-token", "SONAR_PULL_REQUEST": "12"},
        linux=True,
    )
    assert settings.native_required
    assert settings.pull_request == "12"
    assert settings.auth_token == settings.scanner_token == "private-fixture-token"
    assert "private-fixture-token" not in repr(settings)
    assert not load_sonar_settings({"CI": "true"}, linux=False).native_required
    assert load_sonar_settings({"REVAER_REQUIRE_NATIVE_COVERAGE": "1"}, linux=False).native_required
    assert load_sonar_settings(
        {"SONAR_API_BASE_URL": "http://127.0.0.1:8080/api/"}, linux=False
    ).api_base.endswith("/api")


@pytest.mark.parametrize(
    ("path", "production"),
    (
        ("tools/src/revaer_tooling/tasks/sonar.py", False),
        ("scripts/check.sh", False),
        ("crates/revaer-test-support/src/postgres.rs", False),
        ("crates/revaer-ui/tools/asset_sync/src/main.rs", False),
        ("crates/revaer-ui/static/nexus/js/demo.js", False),
        ("vendor/yewdux/examples/basic/src/main.rs", False),
        ("crates/revaer-app/src/bootstrap.rs", True),
        ("crates/revaer-data/init.sql", True),
        ("crates/revaer-torrent-libt/src/ffi/session.cpp", True),
        ("crates/revaer-ui/static/nexus/assets/app.css", True),
        ("vendor/gloo/src/lib.rs", True),
        ("vendor/yewdux/crates/yewdux/src/lib.rs", True),
    ),
)
def test_historical_issues_only_block_active_runtime(
    records: dict[str, JsonObject], path: str, production: bool
) -> None:
    records["all-issues"] = {
        "total": 1,
        "issues": [{"component": "VannaDii_Revaer:" + path}],
    }
    if production:
        with pytest.raises(ToolingError, match="active production"):
            published_result(records, "owned-task", "VannaDii_Revaer")
    else:
        published_result(records, "owned-task", "VannaDii_Revaer")


def test_incomplete_historical_issue_search_cannot_pass(records: dict[str, JsonObject]) -> None:
    records["all-issues"] = {"total": 1, "issues": []}
    with pytest.raises(ToolingError, match="incomplete"):
        published_result(records, "owned-task", "VannaDii_Revaer")


@pytest.mark.parametrize(
    ("name", "value"),
    [
        ("SONAR_API_BASE_URL", "http://external.invalid/api"),
        ("SONAR_API_BASE_URL", "https://user:password@sonarcloud.io/api"),
        ("SONAR_API_BASE_URL", "https://sonarcloud.io/api?override=true"),
        ("SONAR_API_BASE_URL", "invalid"),
        ("SONAR_API_RETRY_ATTEMPTS", "0"),
        ("SONAR_API_RETRY_DELAY_SECONDS", "-1"),
        ("SONAR_RESULT_RETRY_ATTEMPTS", "1.5"),
        ("SONAR_RESULT_RETRY_DELAY_SECONDS", "-1"),
        ("REVAER_REQUIRE_NATIVE_COVERAGE", "false"),
    ],
)
def test_invalid_sonar_configuration_fails_before_external_calls(name: str, value: str) -> None:
    with pytest.raises(ToolingError):
        load_sonar_settings({name: value}, linux=False)
