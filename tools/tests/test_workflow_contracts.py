"""Mutation checks against the media workflow snapshot adapted to rv commands.

The fixture records its source commit. Adaptations replace executable recipes,
add Python coverage, and keep scanner inputs together in their producing job.
The tests independently remove or alter executable steps, dependencies, source
bindings and evidence paths. Comments or duplicate command text cannot satisfy
these checks.
"""

import copy
import json
import shlex
from dataclasses import dataclass
from pathlib import Path

import pytest
from revaer_tooling.cli import parser
from revaer_tooling.e2e.settings import load_e2e_settings
from revaer_tooling.json_data import JsonObject, array_value, decode, object_value, string_value
from revaer_tooling.policy.contracts import runs
from revaer_tooling.policy.formats import Document, Value, yaml_document
from revaer_tooling.policy.image_workflows import image_workflow_findings
from revaer_tooling.policy.required_checks import required_check_findings
from revaer_tooling.policy.scanner_workflows import scanner_workflow_findings
from revaer_tooling.policy.workflows import mapping, named_steps, steps


@dataclass
class Fixture:
    documents: dict[str, Document]
    matrix: JsonObject
    contexts: tuple[str, ...]

    def job(self, workflow: str, job: str) -> Document:
        return mapping(mapping(self.documents[f".github/workflows/{workflow}.yml"]["jobs"])[job])

    def step(self, workflow: str, job: str, name: str) -> Document:
        return named_steps(self.job(workflow, job))[name]


@pytest.fixture
def fixture() -> Fixture:
    source = Path(__file__).parent / "fixtures/workflow-contracts.json"
    record = object_value(decode(source.read_text()))
    assert record["source_revision"] == "f1a8624f8c9c80325bfd14769bc1305c0fa5b793"
    return Fixture(
        documents={
            path: yaml_document(json.dumps(document), path)
            for path, document in object_value(record["documents"]).items()
        },
        matrix=object_value(record["matrix"]),
        contexts=tuple(string_value(value) for value in array_value(record["required_contexts"])),
    )


def test_existing_media_contracts_survive_executor_migration(fixture: Fixture) -> None:
    assert not scanner_workflow_findings(fixture.documents)
    assert not image_workflow_findings(fixture.documents, fixture.matrix)
    assert not required_check_findings(fixture.documents, fixture.contexts, True)
    assert not required_check_findings(fixture.documents, fixture.contexts, False)


def test_committed_image_workflow_preserves_the_media_contract(fixture: Fixture) -> None:
    root = Path(__file__).parents[2]
    path = ".github/workflows/build-images.yml"
    fixture.documents[path] = yaml_document((root / path).read_text(), path)
    assert not image_workflow_findings(fixture.documents, fixture.matrix)


@pytest.mark.parametrize(("workflow", "job_name"), (("pr", "coverage"), ("sonar", "sonar")))
def test_scanner_execution_and_merges_select_the_same_browser_phases(
    workflow: str, job_name: str
) -> None:
    root = Path(__file__).parents[2]
    path = f".github/workflows/{workflow}.yml"
    document = yaml_document((root / path).read_text(), path)
    job = mapping(mapping(document["jobs"])[job_name])
    shared = {key: str(value) for key, value in mapping(job["env"]).items()}
    execution = mapping(named_steps(job)["Run Playwright with browser coverage"]["env"])
    selected = {**shared, **{key: str(value) for key, value in execution.items()}}
    expected = ("api-none", "api-api-key", "ui-chromium", "ui-firefox", "ui-webkit")
    assert load_e2e_settings(selected).phases() == expected
    assert load_e2e_settings(shared).phases() == expected


def test_committed_media_job_preserves_required_conversion_contract(fixture: Fixture) -> None:
    root = Path(__file__).parents[2]
    path = ".github/workflows/pr.yml"
    document = yaml_document((root / path).read_text(), path)
    job = mapping(mapping(document["jobs"])["media-conversion"])
    mapping(fixture.documents[path]["jobs"])["media-conversion"] = job
    assert not required_check_findings(fixture.documents, fixture.contexts, False)
    cache = named_steps(job)["Restore media fixture cache"]
    options = mapping(cache["with"])
    assert options["key"] == (
        "${{ runner.os }}-media-fixtures-${{ steps.media-tools.outputs.tool_version }}-"
        "${{ steps.media-tools.outputs.fixtures }}"
    )
    assert options["restore-keys"] == (
        "${{ runner.os }}-media-fixtures-${{ steps.media-tools.outputs.tool_version }}-\n"
    )
    assert runs(named_steps(job)["Media tool cache key"], "fixture-cache-key")
    assert named_steps(job)["Publish media conversion report"]["run"] == (
        "uv run --locked -- rv workflow-report target/media-conversion-report.md"
    )
    # Validate the real CLI shape as well as the workflow's expected command.
    command = str(named_steps(job)["Publish media conversion report"]["run"])
    arguments = parser().parse_args(shlex.split(command)[5:])
    assert arguments.report == Path("target/media-conversion-report.md")


@pytest.mark.parametrize(
    "name",
    (
        "Coverage",
        "Python tooling coverage",
        "Run Playwright with browser coverage",
        "Merge Python coverage inputs",
        "Merge browser coverage inputs",
        "Build native compile database",
        "Verify Sonar analysis inputs",
        "Prepare Sonar source inputs",
        "Prepare exact Sonar SCM context",
        "SonarQube scan",
        "Package Sonar analysis evidence",
        "Verify Sonar published result",
    ),
)
def test_every_scanner_gate_requires_its_actual_executor(fixture: Fixture, name: str) -> None:
    step = fixture.step("pr", "coverage", name)
    step["env"] = {"DECEPTIVE_COMMAND": step["run"]}
    step["run"] = "echo rv sonar-scan"
    assert scanner_workflow_findings(fixture.documents)


@pytest.mark.parametrize(
    ("name", "key", "value"),
    [
        ("SonarQube scan", "if", "false"),
        ("Run Playwright with browser coverage", "env", {"E2E_BROWSER_COVERAGE": "0"}),
        ("SonarQube scan", "env", {"SONAR_TOKEN": "wrong-source"}),
        ("Prepare exact Sonar SCM context", "env", {"SONAR_HEAD_SHA": "wrong-source"}),
        ("Verify Sonar published result", "env", {"SONAR_PROJECT_KEY": "wrong"}),
        ("Verify Sonar published result", "env", {"SONAR_PROJECT_KEY": "VannaDii_Revaer"}),
        ("Upload coverage artifact", "with", {"path": "coverage/lcov.info"}),
        ("Upload Sonar analysis evidence", "if", "success()"),
        ("Upload Sonar analysis evidence", "with", {"path": "artifacts/sonar/scanner.log"}),
    ],
)
def test_scanner_scope_and_evidence_cannot_drift(
    fixture: Fixture, name: str, key: str, value: Value
) -> None:
    fixture.step("pr", "coverage", name)[key] = value
    assert scanner_workflow_findings(fixture.documents)


def test_scanner_rejects_missing_duplicate_and_reordered_execution(fixture: Fixture) -> None:
    job = fixture.job("pr", "coverage")
    original = copy.deepcopy(job["steps"])
    job["steps"] = [*steps(job), copy.deepcopy(fixture.step("pr", "coverage", "SonarQube scan"))]
    assert any("exactly one" in error for error in scanner_workflow_findings(fixture.documents))
    job["steps"] = original
    job["steps"] = list(reversed(steps(job)))
    assert any("step order" in error for error in scanner_workflow_findings(fixture.documents))
    job["steps"] = []
    assert scanner_workflow_findings(fixture.documents)
    assert scanner_workflow_findings({})


def test_exact_sonar_checkout_toolchain_and_main_result_scope(fixture: Fixture) -> None:
    job = fixture.job("sonar", "sonar")
    original = copy.deepcopy(job)
    for step in steps(job):
        if str(step.get("uses", "")).startswith("actions/checkout@"):
            step["with"] = {"fetch-depth": "1", "ref": "main"}
    assert scanner_workflow_findings(fixture.documents)
    job.update(copy.deepcopy(original))
    for step in steps(job):
        if step.get("uses") == "./.github/actions/setup-revaer":
            step["with"] = {"apt-profile": "none"}
    assert scanner_workflow_findings(fixture.documents)
    job.update(copy.deepcopy(original))
    job["env"] = {"REVAER_REQUIRE_NATIVE_COVERAGE": "0"}
    assert scanner_workflow_findings(fixture.documents)
    job.update(copy.deepcopy(original))
    step = fixture.step("sonar", "sonar", "Verify Sonar published result")
    mapping(step["env"])["SONAR_PULL_REQUEST"] = "1"
    assert scanner_workflow_findings(fixture.documents)


@pytest.mark.parametrize(
    "name",
    (
        "Build & Push Image",
        "Validate supplied compliance authorization against built digest",
        "Build Image (verification only)",
        "Inventory digest-qualified image",
        "Scan digest-qualified image",
        "Scan verification image",
        "Generate digest-bound compliance evidence",
        "Validate digest-bound compliance evidence",
        "Sign image and compliance evidence",
        "Verify signed compliance evidence",
    ),
)
def test_image_steps_cannot_bypass_authorization_or_selected_executor(
    fixture: Fixture, name: str
) -> None:
    step = fixture.step("build-images", "build", name)
    del step["if"]
    assert image_workflow_findings(fixture.documents, fixture.matrix)


@pytest.mark.parametrize(
    "name",
    (
        "Validate supplied compliance authorization against built digest",
        "Inventory digest-qualified image",
        "Scan digest-qualified image",
        "Scan verification image",
        "Generate digest-bound compliance evidence",
        "Validate digest-bound compliance evidence",
        "Sign image and compliance evidence",
        "Verify signed compliance evidence",
    ),
)
def test_image_evidence_cannot_use_another_digest(fixture: Fixture, name: str) -> None:
    step = fixture.step("build-images", "build", name)
    mapping(step["env"])["IMAGE_REFERENCE"] = "image:mutable-tag"
    assert image_workflow_findings(fixture.documents, fixture.matrix)


@pytest.mark.parametrize(
    ("name", "key", "value"),
    [
        ("Log in to GHCR", "if", "always()"),
        ("Verify HIGH and CRITICAL findings", "run", "echo clean"),
        ("Resolve code scanning category", "if", "success()"),
        ("Upload scan results", "if", "success()"),
        ("Install Trivy", "with", {"version": "latest"}),
        ("Upload compliance evidence", "with", {"if-no-files-found": "warn"}),
    ],
)
def test_scans_and_reports_remain_fail_closed(
    fixture: Fixture, name: str, key: str, value: Value
) -> None:
    fixture.step("build-images", "build", name)[key] = value
    assert image_workflow_findings(fixture.documents, fixture.matrix)


def test_image_matrix_call_graph_and_step_order_are_preserved(fixture: Fixture) -> None:
    assert image_workflow_findings(fixture.documents, {})
    build = fixture.job("build-images", "build")
    build["steps"] = list(reversed(steps(build)))
    build["strategy"] = {"fail-fast": "true"}
    build["timeout-minutes"] = "999"
    fixture.job("build-images", "create-manifest")["needs"] = []
    fixture.job("build-images", "publish-dev-helm")["if"] = "always()"
    fixture.job("pr", "build-pr-images")["with"] = {"checkout_ref": "wrong-source"}
    errors = image_workflow_findings(fixture.documents, fixture.matrix)
    for message in (
        "step order",
        "fail-fast",
        "bounded timeout",
        "manifest handling",
        "dev Helm handling",
        "caller",
    ):
        assert any(message in error for error in errors)


@pytest.mark.parametrize("job", ("create-manifest", "publish-dev-helm"))
def test_manifest_and_helm_steps_keep_their_publication_conditions(
    fixture: Fixture, job: str
) -> None:
    for step in steps(fixture.job("build-images", job)):
        step.pop("if", None)
    assert image_workflow_findings(fixture.documents, fixture.matrix)


def test_required_contexts_trigger_and_shards_are_not_narrowed(fixture: Fixture) -> None:
    assert required_check_findings(fixture.documents, fixture.contexts[:-1], True)
    pr = fixture.documents[".github/workflows/pr.yml"]
    pr["on"] = {"pull_request": {"branches": ["main"]}}
    shard = fixture.job("pr", "ui-e2e")
    shard["strategy"] = {"matrix": {"shard": ["1", "2"]}}
    shard["if"] = "false"
    errors = required_check_findings(fixture.documents, fixture.contexts, True)
    for message in ("narrowing", "shards 1, 2, and 3", "conditionally skipped", "not emitted"):
        assert any(message in error for error in errors)


@pytest.mark.parametrize(
    ("job", "name", "key", "value"),
    [
        ("supply-chain", "Verify supply chain results", "env", {"AUDIT_RESULT": "success"}),
        ("udeps", "Restore exact cargo-udeps binary", "with", {"key": "cargo-latest"}),
        ("udeps", "Upload cargo-udeps toolchain evidence", "if", "success()"),
        ("ui-e2e", "Upload E2E coverage", "with", {"if-no-files-found": "warn"}),
        ("api-e2e", "Upload API E2E coverage", "with", {"if-no-files-found": "warn"}),
        ("ui-e2e-coverage", "Download API E2E coverage", "with", {"name": "another-artifact"}),
        (
            "ui-e2e-coverage",
            "Download E2E coverage shard 2",
            "with",
            {"name": "ui-e2e-coverage-shard-1"},
        ),
        ("ui-e2e-coverage", "Verify all UI E2E shard coverage inputs", "run", "echo passed"),
        ("feature-matrix", "Database rebaseline contract", "run", "rv db-migrate"),
    ],
)
def test_required_job_artifact_and_execution_contracts(
    fixture: Fixture, job: str, name: str, key: str, value: Value
) -> None:
    fixture.step("pr", job, name)[key] = value
    assert required_check_findings(fixture.documents, fixture.contexts, True)


def test_required_dependency_graph_and_exact_udeps_pin(fixture: Fixture) -> None:
    fixture.job("pr", "ui-e2e-coverage")["needs"] = ["ui-e2e"]
    fixture.job("pr", "supply-chain")["needs"] = ["audit"]
    fixture.job("pr", "supply-chain")["if"] = "success()"
    fixture.job("pr", "udeps")["env"] = {"REVAER_UDEPS_VERSION": "latest"}
    errors = required_check_findings(fixture.documents, fixture.contexts, True)
    assert len(errors) >= 4
    feature = fixture.job("pr", "feature-matrix")
    feature["steps"] = list(reversed(steps(feature)))
    assert any(
        "step order" in error
        for error in required_check_findings(fixture.documents, fixture.contexts, True)
    )


def test_command_matching_requires_a_complete_invocation() -> None:
    assert runs({"run": "uv run --locked -- rv ci"}, "ci")
    assert runs({"run": ".venv/bin/rv ci"}, "ci")
    assert not runs({}, "ci")
    assert not runs({"run": "rv ci 'unclosed"}, "ci")
    assert not runs({"run": "echo rv ci"}, "ci")
    assert not runs({"run": "rv ci; true"}, "ci")


@pytest.mark.parametrize(
    ("name", "key", "value"),
    [
        ("Clean media fixtures", "if", "success()"),
        ("Clean media fixtures", "run", "echo cleanup"),
        ("Upload media conversion report", "if", "success()"),
        ("Upload media conversion report", "uses", "actions/download-artifact@wrong"),
        ("Upload media conversion report", "with", {"path": "wrong", "if-no-files-found": "error"}),
        (
            "Upload media conversion report",
            "with",
            {"path": "target/media-conversion-report.md", "if-no-files-found": "warn"},
        ),
        ("Media conversion integration tests", "run", "echo tests"),
        ("Media conversion integration tests", "if", "false"),
    ],
)
def test_media_evidence_and_cleanup_cannot_be_bypassed(
    fixture: Fixture, name: str, key: str, value: Value
) -> None:
    fixture.step("pr", "media-conversion", name)[key] = value
    assert required_check_findings(fixture.documents, fixture.contexts, True)


@pytest.mark.parametrize("name", ("build-pr-images", "build-release"))
def test_media_gate_is_required_by_each_downstream_build(fixture: Fixture, name: str) -> None:
    fixture.job("pr", name)["needs"] = ["supply-chain"]
    assert required_check_findings(fixture.documents, fixture.contexts, True)


def test_media_cleanup_cannot_precede_report_upload(fixture: Fixture) -> None:
    job = fixture.job("pr", "media-conversion")
    job["steps"] = list(reversed(steps(job)))
    assert required_check_findings(fixture.documents, fixture.contexts, True)


def test_release_build_cannot_skip_pull_requests(fixture: Fixture) -> None:
    fixture.job("pr", "build-release")["if"] = "github.ref == 'refs/heads/main'"
    assert required_check_findings(fixture.documents, fixture.contexts, True)


def test_single_init_workflow_requires_fixture_gate_and_rejects_migration(fixture: Fixture) -> None:
    feature = fixture.job("pr", "feature-matrix")
    feature["steps"] = [
        step
        for step in steps(feature)
        if step.get("name") not in ("Database rebaseline contract", "Run migrations")
    ]
    feature_steps = steps(feature)
    index = next(
        index
        for index, step in enumerate(feature_steps)
        if step.get("name") == "Feature matrix tests (no default features)"
    )
    feature_steps.insert(
        index,
        {
            "name": "Single-init application fixtures",
            "run": "uv run --locked -- rv ui-e2e-app-test",
        },
    )
    feature["steps"] = list[Value](feature_steps)
    for job in mapping(fixture.documents[".github/workflows/pr.yml"].get("jobs")).values():
        selected = mapping(job)
        selected["steps"] = [step for step in steps(selected) if not runs(step, "db-migrate")]
    assert not required_check_findings(fixture.documents, fixture.contexts, True, single_init=True)
    fixture.step("pr", "feature-matrix", "Single-init application fixtures")["run"] = (
        "rv db-migrate"
    )
    findings = required_check_findings(fixture.documents, fixture.contexts, True, single_init=True)
    assert any("initialized application fixtures" in item for item in findings)
    assert any("historical migrations" in item for item in findings)
