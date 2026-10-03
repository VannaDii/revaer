"""Mutate the actual release workflow at its artifact and eligibility boundaries."""

from pathlib import Path

import pytest
from revaer_tooling.policy.formats import Document, Value, yaml_document
from revaer_tooling.policy.release_workflows import release_workflow_findings
from revaer_tooling.policy.workflows import mapping, named_steps, steps

PATH = ".github/workflows/ci.yml"


@pytest.fixture
def document() -> Document:
    return yaml_document((Path(__file__).parents[2] / PATH).read_text(), PATH)


def test_actual_release_workflow_preserves_artifacts_and_branch_behavior(
    document: Document,
) -> None:
    assert not release_workflow_findings({PATH: document})
    assert release_workflow_findings({})


@pytest.mark.parametrize(
    "name,key,value",
    (
        ("build-release", "if", "true"),
        ("build-release", "outputs", {"short_sha": "${{ github.sha }}"}),
        ("release-dev", "if", "true"),
        ("release-dev", "needs", "load-matrix"),
        ("release-dev", "outputs", {"released": "true"}),
        ("publish-release", "if", "github.ref_type == 'tag'"),
        ("publish-dev-helm", "if", "github.ref == 'refs/heads/main'"),
        ("publish-release-helm", "needs", "release-dev"),
        ("load-matrix", "needs", "release-dev"),
        ("build-images-dev", "if", "github.ref == 'refs/heads/main'"),
        ("build-images-dev", "with", {"alias_tag": "latest"}),
        ("build-images-release", "needs", ["release-dev", "load-matrix"]),
        ("build-images-release", "with", {"version_tag": "dev"}),
    ),
)
def test_publication_eligibility_and_source_outputs_cannot_drift(
    document: Document, name: str, key: str, value: Value
) -> None:
    mapping(mapping(document["jobs"])[name])[key] = value
    assert release_workflow_findings({PATH: document})


@pytest.mark.parametrize(
    "job,stage,key,value",
    (
        ("build-release", "Resolve source identity", "run", "echo rv workflow-metadata"),
        ("build-release", "Build release artifacts", "run", "rv build-release"),
        ("build-release", "Upload release artifacts", "with", {"path": "dist"}),
        ("release-dev", "Download release artifacts", "with", {"name": "latest-build"}),
        ("release-dev", "Publish dev release", "id", "unrelated-output"),
        ("release-dev", "Publish dev release", "env", {"REVAER_ENABLE_HELM_RELEASE_ASSETS": "0"}),
        ("release-dev", "Upload Helm release assets", "if", "success()"),
        ("publish-release", "Publish stable release", "run", "rv release publish"),
        ("publish-release", "Publish stable release", "env", {"RELEASE_TAG": "v1.0.0"}),
        ("publish-release", "Upload Helm release assets", "if", "false"),
        ("publish-dev-helm", "Download Helm release assets", "with", {"name": "another-chart"}),
        (
            "publish-dev-helm",
            "Publish dev Helm chart",
            "run",
            'rv helm-package "$RELEASE_VERSION" "$RELEASE_TAG"',
        ),
        ("publish-release-helm", "Publish release Helm chart", "env", {"RELEASE_TAG": "dev"}),
        ("load-matrix", "Select release image matrix", "run", "rv workflow-matrix"),
    ),
)
def test_required_release_operations_cannot_be_replaced_or_detached(
    document: Document, job: str, stage: str, key: str, value: Value
) -> None:
    named_steps(mapping(mapping(document["jobs"])[job]))[stage][key] = value
    assert release_workflow_findings({PATH: document})


def test_stable_history_artifact_order_and_registry_signing_ownership(document: Document) -> None:
    job = mapping(mapping(document["jobs"])["publish-release"])
    checkout = steps(job)[0]
    checkout["with"] = {"fetch-depth": "1"}
    assert any("history" in item for item in release_workflow_findings({PATH: document}))
    checkout["with"] = {"fetch-depth": "0"}
    job["steps"] = list(reversed(steps(job)))
    assert any("step order" in item for item in release_workflow_findings({PATH: document}))
    job["steps"] = list(reversed(steps(job)))
    document["env"] = {"HELM_GPG_PRIVATE": "${{ secrets.HELM_GPG_PRIVATE }}"}
    assert any("signing material" in item for item in release_workflow_findings({PATH: document}))


def test_release_workflow_cannot_take_over_pull_request_validation(document: Document) -> None:
    mapping(document["on"])["pull_request"] = {}
    assert any("push triggers" in item for item in release_workflow_findings({PATH: document}))
