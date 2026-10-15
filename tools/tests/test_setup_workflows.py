"""Exercise the committed composite action and its real CLI argument contract."""

import copy
import re
import shlex
import tomllib
from pathlib import Path

import pytest
from revaer_tooling.bootstrap.selection import apt_selection
from revaer_tooling.cli import COMMANDS, parser
from revaer_tooling.e2e.settings import load_e2e_settings
from revaer_tooling.policy.formats import Document, yaml_document
from revaer_tooling.policy.setup_workflows import setup_workflow_findings
from revaer_tooling.policy.workflows import SETUP_PATH, mapping, steps, workflow_findings


@pytest.fixture
def action() -> Document:
    root = Path(__file__).parents[2]
    return yaml_document((root / SETUP_PATH).read_text(), SETUP_PATH)


def test_media_api_producer_runs_every_phase_required_by_aggregation() -> None:
    root = Path(__file__).parents[2]
    path = ".github/workflows/pr.yml"
    document = yaml_document((root / path).read_text(), path)
    jobs = mapping(document["jobs"])
    producer = mapping(mapping(jobs["api-e2e"])["env"])
    aggregate = mapping(mapping(jobs["ui-e2e-coverage"])["env"])
    selected = load_e2e_settings({key: str(value) for key, value in producer.items()})
    required = load_e2e_settings({key: str(value) for key, value in aggregate.items()})
    assert selected.phases(media=True) == tuple(
        phase for phase in required.phases(media=True) if phase.startswith("api-")
    )
    upload = next(
        step
        for step in steps(mapping(jobs["api-e2e"]))
        if step.get("name") == "Upload API E2E coverage"
    )
    paths = str(mapping(upload["with"])["path"]).splitlines()
    for phase in selected.phases(media=True):
        assert f"coverage/e2e/{phase}/.coverage" in paths


@pytest.mark.parametrize("job_name", ("api-e2e", "ui-e2e"))
def test_media_e2e_jobs_provision_capability_and_mount_tools(job_name: str) -> None:
    root = Path(__file__).parents[2]
    path = ".github/workflows/pr.yml"
    document = yaml_document((root / path).read_text(), path)
    job = mapping(mapping(document["jobs"])[job_name])
    setup = [step for step in steps(job) if step.get("uses") == "./.github/actions/setup-revaer"]
    assert len(setup) == 1
    values = mapping(setup[0].get("with"))
    packages = apt_selection(
        str(values.get("apt-profile", "")),
        tuple(str(values.get("apt-packages", "")).split()),
    )
    assert "ffmpeg" in packages
    assert "e2fsprogs" in packages


def test_committed_setup_action_uses_official_uv_and_valid_commands(action: Document) -> None:
    root = Path(__file__).parents[2]
    assert not workflow_findings(SETUP_PATH, (root / SETUP_PATH).read_text(), frozenset(COMMANDS))
    assert not setup_workflow_findings({SETUP_PATH: action})
    # Parse every actual rv invocation, including optional installers. Resolve
    # only the action's declared input bindings; do not emulate a shell.
    inputs = mapping(action["inputs"])
    for profile in ("ci", "python"):
        values = {name: str(mapping(value).get("default", "")) for name, value in inputs.items()}
        values["profile"] = profile
        for step in steps(mapping(action["runs"])):
            run = str(step.get("run", ""))
            if not run.startswith("uv run --locked -- rv "):
                continue
            environment: dict[str, str] = {}
            for key, value in mapping(step.get("env")).items():
                binding = re.fullmatch(r"\$\{\{ inputs\.([a-z-]+) \}\}", str(value))
                assert binding is not None, value
                environment[f"${key}"] = values[binding[1]]
            argv = [environment.get(token, token) for token in shlex.split(run)[5:]]
            assert not any("$" in token for token in argv)
            parsed = parser().parse_args(argv)
            assert parsed.task == "setup"
            assert parsed.no_launcher


def test_tool_caches_reuse_independent_native_installation_roots(action: Document) -> None:
    pins = tomllib.loads((Path(__file__).parents[2] / "tools/versions.toml").read_text())
    stages = steps(mapping(action["runs"]))
    caches = {step.get("name"): step for step in stages}
    installed = next(
        index
        for index, step in enumerate(stages)
        if step.get("name") == "Set up selected Revaer tools"
    )
    for package in pins["cargo"]:
        restore = caches[f"Cache pinned {package}"]
        cache = mapping(restore.get("with"))
        assert cache["path"] == f"~/.local/revaer/cargo-tools/{package}"
        assert "inputs.cargo-tools" not in str(cache["key"])
        assert package in str(cache["key"])
        assert str(restore.get("uses")).startswith("actions/cache/restore@")
        saved = caches[f"Save pinned {package}"]
        assert stages.index(restore) < installed < stages.index(saved)
        assert str(saved.get("uses")).startswith("actions/cache/save@")
        assert mapping(saved.get("with"))["path"] == cache["path"]
        assert str(restore["id"]) in str(mapping(saved.get("with"))["key"])
        assert "cache-hit != 'true'" in str(saved.get("if"))
    kcov = caches["Cache pinned kcov source and installation"]
    saved_kcov = caches["Save pinned kcov"]
    assert str(kcov.get("uses")).startswith("actions/cache/restore@")
    assert str(saved_kcov.get("uses")).startswith("actions/cache/save@")
    assert (
        stages.index(kcov) < stages.index(caches["Install pinned kcov"]) < stages.index(saved_kcov)
    )
    assert mapping(saved_kcov.get("with"))["path"] == mapping(kcov.get("with"))["path"]


@pytest.mark.parametrize("name", ("docs", "helm-oci-verify", "build-images", "ci"))
def test_converted_workflows_use_the_declared_setup_and_cli(action: Document, name: str) -> None:
    root = Path(__file__).parents[2]
    path = f".github/workflows/{name}.yml"
    source = (root / path).read_text()
    document = yaml_document(source, path)
    assert not workflow_findings(path, source, frozenset(COMMANDS))
    assert not setup_workflow_findings({SETUP_PATH: action, path: document})
    for job in mapping(document["jobs"]).values():
        for step in steps(mapping(job)):
            if "run" in step:
                command = shlex.split(str(step["run"]))
                assert command[:5] == ["uv", "run", "--locked", "--", "rv"]
                if "$PR_NUMBER" in command:
                    assert mapping(step.get("env")).get("PR_NUMBER") == "${{ inputs.pr_number }}"
                    command = ["61" if token == "$PR_NUMBER" else token for token in command]
                parser().parse_args(command[5:])


@pytest.mark.parametrize(
    "stage", ("Install uv", "Sync locked Python environment", "Set up selected Revaer tools")
)
def test_bootstrap_stages_cannot_be_disabled(action: Document, stage: str) -> None:
    for step in steps(mapping(action["runs"])):
        if step.get("name") == stage:
            step["if"] = "false"
    assert any("unconditional" in item for item in setup_workflow_findings({SETUP_PATH: action}))


@pytest.mark.parametrize("override", ("version", "python-version", "activate-environment"))
def test_uv_overrides_cannot_replace_project_pins(action: Document, override: str) -> None:
    uv = steps(mapping(action["runs"]))[0]
    mapping(uv["with"])[override] = "unexpected"
    assert any("project pins" in item for item in setup_workflow_findings({SETUP_PATH: action}))


def test_missing_duplicate_reordered_and_wrong_pin_sources_fail(action: Document) -> None:
    assert setup_workflow_findings({})
    for mutation in ("missing", "duplicate", "reordered", "pin"):
        altered = copy.deepcopy(action)
        stages = steps(mapping(altered["runs"]))
        if mutation == "missing":
            stages.pop(0)
        elif mutation == "duplicate":
            stages.insert(0, copy.deepcopy(stages[0]))
        elif mutation == "reordered":
            stages[0], stages[1] = stages[1], stages[0]
        else:
            mapping(stages[0]["with"])["version-file"] = ".python-version"
        mapping(altered["runs"])["steps"] = list(stages)
        assert setup_workflow_findings({SETUP_PATH: altered}), mutation


@pytest.mark.parametrize("retired", ("toolchain", "components", "node-version"))
def test_callers_cannot_pass_retired_inputs(action: Document, retired: str) -> None:
    caller: Document = {
        "jobs": {
            "build": {
                "steps": [{"uses": "./.github/actions/setup-revaer", "with": {retired: "old"}}]
            }
        }
    }
    failures = setup_workflow_findings({SETUP_PATH: action, ".github/workflows/caller.yml": caller})
    assert any(f"unknown setup inputs: {retired}" in item for item in failures)
    job = mapping(mapping(caller["jobs"])["build"])
    steps(job)[0]["with"] = "invalid"
    assert any(
        "must be a mapping" in item
        for item in setup_workflow_findings({SETUP_PATH: action, "caller": caller})
    )
