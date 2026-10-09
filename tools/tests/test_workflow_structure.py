"""Policy mutation cases preserve Actions semantics while migrating its executor."""

import copy

import pytest
import yaml
from revaer_tooling.policy.formats import Document, Value
from revaer_tooling.policy.workflows import workflow_findings

COMMANDS = frozenset(("ci", "setup", "helm-package", "release preview"))
PATH = ".github/workflows/fixture.yml"


@pytest.fixture
def workflow() -> Document:
    return {
        "on": {"pull_request": {}},
        "permissions": {"contents": "read"},
        "jobs": {
            "check": {
                "timeout-minutes": "30",
                "steps": [
                    {"uses": "actions/checkout@" + "a" * 40},
                    {"uses": "./.github/actions/setup-revaer", "timeout-minutes": "20"},
                    {"name": "Check", "run": "uv run --locked -- rv ci"},
                ],
            }
        },
    }


def findings(document: Document, path: str = PATH) -> list[str]:
    return workflow_findings(path, yaml.safe_dump(document), COMMANDS)


def test_valid_workflow_and_reusable_dependencies(workflow: Document) -> None:
    assert not findings(workflow)
    workflow["jobs"] = {
        "check": {"uses": "./.github/workflows/shared.yml"},
        "release": {"needs": "check", "uses": "owner/shared/workflows/ci.yml@" + "b" * 40},
    }
    assert not findings(workflow)


@pytest.mark.parametrize(
    ("profile", "timeout", "accepted"),
    (
        ("coverage", "40", True),
        ("coverage", "20", False),
        ("coverage", "41", False),
        ("base", "20", True),
        ("base", "40", False),
    ),
)
def test_setup_keeps_the_selected_profile_timeout(
    profile: str, timeout: str, accepted: bool
) -> None:
    document: Document = {
        "jobs": {
            "check": {
                "timeout-minutes": "60",
                "steps": [
                    {
                        "uses": "./.github/actions/setup-revaer",
                        "timeout-minutes": timeout,
                        "with": {"apt-profile": profile},
                    }
                ],
            }
        },
    }
    assert (not findings(document)) == accepted


@pytest.mark.parametrize("position", ("absent", "conditional", "late"))
def test_every_command_requires_completed_setup(workflow: Document, position: str) -> None:
    jobs = workflow["jobs"]
    assert isinstance(jobs, dict)
    job = jobs["check"]
    assert isinstance(job, dict)
    setup: Document = {"uses": "./.github/actions/setup-revaer", "timeout-minutes": "20"}
    command: Document = {"run": "rv ci"}
    if position == "absent":
        job["steps"] = [command]
    elif position == "conditional":
        setup["if"] = "false"
        job["steps"] = [setup, command]
    else:
        job["steps"] = [command, setup]
    assert any("earlier unconditional setup" in failure for failure in findings(workflow))


@pytest.mark.parametrize(
    "step",
    [
        {"uses": "owner/action@main"},
        {"uses": "docker://busybox:latest"},
        {"uses": "./../unrelated/action"},
        {"uses": "actions/github-script@" + "a" * 40},
        {"uses": "SonarSource/sonarqube-scan-action@" + "a" * 40},
        {"uses": "aquasecurity/trivy-action@" + "a" * 40},
        {"uses": "docker/build-push-action@" + "a" * 40},
        {"run": "cargo test"},
        {"run": "rv misspelled"},
        {"run": "rv ci; true"},
        {"run": "rv ci || true"},
        {"run": "rv ci\nrv ci"},
        {"run": "rv ci $(echo unsafe)"},
        {"run": "rv ci `echo unsafe`"},
        {"run": "rv ci > /dev/null"},
        {"run": 'rv helm-package "${{ inputs.version }}"'},
        {"run": 'rv helm-package "${{ github.event.issue.title }}"'},
        {"run": "rv ci --define sonar.exclusions=*"},
        {"run": "rv ci -Dsonar.exclusions=*"},
        {"run": "rv ci 'unclosed"},
        {"run": ""},
        {"run": ["rv", "ci"]},
        {"run": "rv ci", "uses": "./action"},
        {"name": "missing command"},
        {"run": "rv ci", "continue-on-error": "true"},
        {"run": "rv ci", "continue-on-error": "${{ false }}"},
        {"run": "rv ci", "if": []},
        {"run": "rv ci", "if": "true\nfalse"},
        {"uses": "./.github/actions/setup-revaer"},
    ],
)
def test_structural_mutations_are_rejected(step: Document) -> None:
    setup: Document = {"uses": "./.github/actions/setup-revaer", "timeout-minutes": "20"}
    document: Document = {"jobs": {"check": {"timeout-minutes": "30", "steps": [setup, step]}}}
    assert findings(document)


@pytest.mark.parametrize("timeout", (None, "0", "181", "-1", "1.5", "${{ inputs.minutes }}"))
def test_job_timeouts_are_positive_and_bounded(timeout: str | None) -> None:
    job: Document = {"steps": [{"run": "rv ci"}]}
    if timeout is not None:
        job["timeout-minutes"] = timeout
    assert any("timeout-minutes" in item for item in findings({"jobs": {"check": job}}))


def test_supported_steps_environment_inputs_and_false_are_accepted() -> None:
    document: Document = {
        "jobs": {
            "check": {
                "timeout-minutes": "180",
                "continue-on-error": "false",
                "steps": [
                    {"uses": "docker://busybox@sha256:" + "a" * 64},
                    {"uses": "./.github/actions/setup-revaer", "timeout-minutes": "20"},
                    {"run": "rv release preview", "continue-on-error": "false"},
                    {"run": ".venv/bin/rv ci", "if": "success()"},
                    {
                        "run": 'rv helm-package "$VERSION"',
                        "env": {"VERSION": "${{ inputs.version }}"},
                    },
                ],
            }
        }
    }
    assert not findings(document)
    document = {"runs": {"using": "composite", "steps": [{"run": "uv sync --locked"}]}}
    assert not findings(document, ".github/actions/setup-revaer/action.yml")
    assert findings(document, ".github/actions/other/action.yml")


def test_duplicate_names_ids_and_dependency_errors_are_detected() -> None:
    step: Document = {"run": "rv ci", "id": "ci", "name": "CI"}
    job: Document = {
        "timeout-minutes": "30",
        "steps": [{"uses": "./.github/actions/setup-revaer", "timeout-minutes": "20"}, step],
    }
    document: Document = {"jobs": {"check": job}}
    original = copy.deepcopy(document)
    job["steps"] = [step, copy.deepcopy(step)]
    assert any("duplicate nonempty step name" in item for item in findings(document))
    assert any("duplicate nonempty step id" in item for item in findings(document))
    assert not findings(original)
    invalid: tuple[Value, ...] = ("missing", "check", ["missing", "missing"], {"invalid": "value"})
    for dependency in invalid:
        job["needs"] = dependency
        assert any("dependency" in item or "job needs" in item for item in findings(document))


def test_permissions_job_and_action_shapes_fail_closed() -> None:
    documents: tuple[Document, ...] = (
        {"jobs": {}},
        {"jobs": []},
        {"jobs": {"check": "wrong"}},
        {"jobs": {"check": {"uses": "./action", "steps": []}}},
        {"jobs": {"check": {"uses": "./action", "continue-on-error": "true"}}},
        {"jobs": {"check": {"timeout-minutes": "1", "steps": ["wrong"]}}},
        {"jobs": {"check": {"timeout-minutes": "1", "steps": []}}},
        {"jobs": {"check": {"uses": "./action", "permissions": {}}}},
        {"jobs": {"check": {"uses": "./action", "permissions": {"contents": "all"}}}},
    )
    for document in documents:
        assert workflow_findings(PATH, yaml.safe_dump(document), COMMANDS)
    assert findings({"runs": {"using": "node24"}}, ".github/actions/fixture/action.yml")


def test_postgres_service_matches_all_job_urls() -> None:
    url = "postgres://fixture:local@localhost:5432/revaer"
    job: Document = {
        "uses": "./fixture",
        "services": {
            "postgres": {
                "env": {
                    "POSTGRES_USER": "fixture",
                    "POSTGRES_PASSWORD": "local",
                    "POSTGRES_DB": "revaer",
                }
            }
        },
        "env": {
            "DATABASE_URL": url,
            "REVAER_TEST_DATABASE_URL": url,
            "E2E_DB_ADMIN_URL": url.replace("/revaer", "/postgres"),
        },
    }
    assert not findings({"jobs": {"check": job}})
    job["env"] = {"DATABASE_URL": "wrong", "E2E_DB_ADMIN_URL": "wrong"}
    assert len(findings({"jobs": {"check": job}})) == 3
    job["services"] = {"postgres": {"env": {}}}
    assert any(
        "user, password, and database" in item for item in findings({"jobs": {"check": job}})
    )
