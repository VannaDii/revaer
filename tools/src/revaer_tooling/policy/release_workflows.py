"""Preserve release eligibility, verified outputs and signed artifact handoff.

The release task owns completion. A successful no-change preview must not send
an old tag or missing chart to downstream publication. Stable image jobs remain
independent of the main-only prerelease branch.
"""

from collections.abc import Mapping

from .contracts import Contract, runs, workflow
from .formats import Document
from .workflows import mapping, needs, steps

MAIN = "github.ref == 'refs/heads/main'"
STABLE = "startsWith(github.ref, 'refs/tags/') && !contains(github.ref_name, '-')"
DEV_COMPLETE = "needs.release-dev.outputs.released == 'true'"


def release_workflow_findings(documents: Mapping[str, Document]) -> list[str]:
    policy = workflow(documents, "ci")
    policy.require(
        policy.document.get("on") == {"push": {"branches": ["main"], "tags": ["v*.*.*"]}},
        "release CI must retain its main and version-tag push triggers",
    )
    build = policy.job("build-release")
    policy.require(
        build.get("if") == "github.ref_type != 'tag' || !contains(github.ref_name, '-')",
        "release artifact builds must exclude prerelease tags",
    )
    identity = policy.step(build, "Resolve source identity")
    policy.require(
        identity.get("id") == "source" and runs(identity, "workflow-metadata"),
        "release artifact names must use the actual checkout identity",
    )
    policy.require(
        build.get("outputs") == {"short_sha": "${{ steps.source.outputs.short_sha }}"},
        "release consumers must receive the artifact producer's source identity",
    )
    policy.require(
        runs(policy.step(build, "Build release artifacts"), "release-artifacts"),
        "release artifacts must be prepared and source-bound through rv",
    )
    upload = policy.step(build, "Upload release artifacts")
    policy.paths(upload, ("dist",))
    policy.require(
        mapping(upload.get("with")).get("name")
        == "revaer-release-${{ steps.source.outputs.short_sha }}",
        "release artifact upload must use the producer's short SHA",
    )
    policy.order(
        build, ("Resolve source identity", "Build release artifacts", "Upload release artifacts")
    )
    for name, condition, stage in (
        ("release-dev", MAIN, "Publish dev release"),
        ("publish-release", STABLE, "Publish stable release"),
    ):
        _publication(policy, name, condition, stage)
    _charts(policy, "publish-dev-helm", "release-dev", f"{MAIN} && {DEV_COMPLETE}")
    _charts(policy, "publish-release-helm", "publish-release", STABLE)
    matrix = policy.job("load-matrix")
    policy.require(
        needs(matrix) == ("build-release",)
        and runs(
            policy.step(matrix, "Select release image matrix"), "workflow-matrix", "--release-only"
        ),
        "release matrix must follow artifact build and use the shared release selector",
    )
    for name, prerequisites, condition, tag, alias in (
        (
            "build-images-dev",
            ("load-matrix", "release-dev"),
            f"{MAIN} && needs.release-dev.result == 'success' && "
            f"{DEV_COMPLETE} && needs.load-matrix.result == 'success'",
            "${{ needs.release-dev.outputs.release_tag }}",
            "dev",
        ),
        (
            "build-images-release",
            ("load-matrix",),
            f"{STABLE} && needs.load-matrix.result == 'success'",
            "${{ github.ref_name }}",
            "latest",
        ),
    ):
        job = policy.job(name)
        policy.require(
            needs(job) == prerequisites and job.get("if") == condition,
            f"{name} must preserve release eligibility and completion dependencies",
        )
        policy.require(
            job.get("uses") == "./.github/workflows/build-images.yml"
            and job.get("with")
            == {
                "matrix": "${{ needs.load-matrix.outputs.matrix }}",
                "image_name": "revaer",
                "version_tag": tag,
                "alias_tag": alias,
                "include_sha_tag": "true",
            },
            f"{name} must preserve the shared image workflow, version, alias and SHA tags",
        )
    return policy.failures


def _publication(policy: Contract, name: str, condition: str, stage: str) -> None:
    job = policy.job(name)
    policy.require(
        needs(job) == ("build-release",) and job.get("if") == condition,
        f"{name} must publish only its eligible branch/tag after artifact preparation",
    )
    policy.require(
        job.get("outputs")
        == {
            "release_tag": "${{ steps.release-info.outputs.tag }}",
            "release_version": "${{ steps.release-info.outputs.version }}",
            "released": "${{ steps.release-info.outputs.released }}",
        },
        f"{name} outputs must come from verified rv release completion",
    )
    checkout = [
        step for step in steps(job) if str(step.get("uses", "")).startswith("actions/checkout@")
    ]
    policy.require(
        len(checkout) == 1 and mapping(checkout[0].get("with")).get("fetch-depth") == "0",
        f"{name} must preserve complete release history",
    )
    download = policy.step(job, "Download release artifacts")
    policy.require(
        download.get("with")
        == {"name": "revaer-release-${{ needs.build-release.outputs.short_sha }}", "path": "dist"},
        f"{name} must consume the source-bound build artifact",
    )
    publish = policy.step(job, stage)
    arguments = (
        ("release", "publish", "--tag", "$RELEASE_TAG")
        if name == "publish-release"
        else ("release", "publish")
    )
    policy.require(
        publish.get("id") == "release-info" and runs(publish, *arguments),
        f"{name} must invoke the corresponding rv publication operation",
    )
    environment = mapping(publish.get("env"))
    policy.require(
        environment.get("REVAER_ENABLE_HELM_RELEASE_ASSETS") == "1",
        f"{name} must retain signed Helm release assets",
    )
    if name == "publish-release":
        policy.require(
            environment.get("RELEASE_TAG") == "${{ github.ref_name }}",
            "stable publication must use the event's existing tag",
        )
    upload = policy.step(job, "Upload Helm release assets")
    policy.paths(upload, ("dist/helm",))
    policy.require(
        mapping(upload.get("with")).get("name")
        == "revaer-helm-${{ steps.release-info.outputs.tag }}",
        f"{name} chart artifact must use the completed release tag",
    )
    policy.require(
        upload.get("if") == "steps.release-info.outputs.released == 'true'"
        if name == "release-dev"
        else "if" not in upload,
        f"{name} chart handoff must follow the corresponding release completion",
    )
    policy.order(job, ("Download release artifacts", stage, "Upload Helm release assets"))


def _charts(policy: Contract, name: str, producer: str, condition: str) -> None:
    job = policy.job(name)
    policy.require(
        needs(job) == (producer,) and job.get("if") == condition,
        f"{name} must wait for its completed release",
    )
    download = policy.step(job, "Download Helm release assets")
    policy.require(
        download.get("with")
        == {
            "name": f"revaer-helm-${{{{ needs.{producer}.outputs.release_tag }}}}",
            "path": "dist/helm",
        },
        f"{name} must consume the existing signed chart artifact",
    )
    stage = "Publish dev Helm chart" if producer == "release-dev" else "Publish release Helm chart"
    publish = policy.step(job, stage)
    environment = mapping(publish.get("env"))
    policy.require(
        runs(publish, "helm-publish", "$RELEASE_VERSION", "$RELEASE_TAG")
        and environment.get("RELEASE_VERSION")
        == f"${{{{ needs.{producer}.outputs.release_version }}}}"
        and environment.get("RELEASE_TAG") == f"${{{{ needs.{producer}.outputs.release_tag }}}}",
        f"{name} must verify and publish the original chart identity through rv",
    )
    for owner in (policy.document, job, *steps(job)):
        policy.require(
            not {"HELM_GPG_PRIVATE", "HELM_GPG_PUBLIC"}.intersection(mapping(owner.get("env"))),
            f"{name} registry jobs must consume signed artifacts without signing material",
        )
    policy.order(job, ("Download Helm release assets", stage))
