"""Coordinate release preparation, publication and explicit partial-release recovery.

Python Semantic Release alone selects versions and writes release notes. GitHub's
CLI transfers the exact prepared files, including on retries. Completion output
belongs to this orchestration layer because neither individual tool can prove the
whole operation finished.
"""

import json
import re
import tomllib
from dataclasses import replace
from pathlib import Path

from ..artifacts import APPLICATION_ARTIFACTS, MANIFEST, digest_file, verify_artifacts
from ..automation.files import write_values
from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.release import ReleasePlan, ReleaseRepository
from .base import Task
from .charts import HelmPackage

STABLE_TAG = r"v(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)"


def release_repository(context: Context) -> ReleaseRepository:
    config = tomllib.loads(context.fs.read(context.root / "release/semantic-release.toml"))
    return context.tools.semantic_release.repository(
        context.tools.git.remote_url(), config["semantic_release"]["remote"]
    )


def artifact_paths(context: Context, version: str) -> tuple[str, ...]:
    files: tuple[str, ...] = (*APPLICATION_ARTIFACTS, MANIFEST)
    if context.settings.helm_release_assets:
        files += (
            f"dist/helm/revaer-{version}.tgz",
            f"dist/helm/revaer-{version}.tgz.prov",
            "dist/helm/revaer-helm-public.asc",
            "dist/helm/revaer-helm-public.gpg",
            "dist/helm/artifacthub-repo.yml",
        )
    return files


def write_release_info(
    context: Context, plan: ReleasePlan, sha: str, *, released: bool, output: bool = True
) -> None:
    context.fs.write(
        context.root / "release/next-release.json",
        json.dumps(
            {
                "version": plan.version,
                "gitTag": plan.tag,
                "sourceCommit": sha,
                "released": released,
            },
            indent=2,
        )
        + "\n",
    )
    destination = context.settings.workflow.output
    if output and destination:
        write_values(
            context.fs,
            destination,
            {"version": plan.version, "tag": plan.tag, "released": str(released).lower()},
        )


def prepared_paths(context: Context, plan: ReleasePlan, source: str) -> tuple[Path, ...]:
    verify_artifacts(context, source)
    paths = tuple(context.root / name for name in artifact_paths(context, plan.version))
    for path in paths:
        digest_file(path)
    if context.settings.helm_release_assets:
        context.tools.helm.verify_chart(
            context.root / f"dist/helm/revaer-{plan.version}.tgz",
            context.root / "dist/helm/revaer-helm-public.gpg",
        )
    return paths


def publish_assets(
    context: Context,
    repository: ReleaseRepository,
    plan: ReleasePlan,
    source: str,
    paths: tuple[Path, ...],
) -> TaskResult:
    context.tools.github.upload_assets(repository, plan.tag, paths)
    context.tools.github.verify_assets(repository, plan.tag, paths)
    write_release_info(context, plan, source, released=True)
    return TaskResult(f"Published and verified {plan.tag}")


def publish_stable_tag(context: Context, *, prepare: bool) -> TaskResult:
    """Publish an operator-created stable tag, including a detached CI checkout.

    The existing stable-tag workflow does not calculate a version or generate notes.
    Keep that contract through gh; do not invent an eligible branch for PSR, whose
    commands require an attached release branch even for existing-tag operations.
    """
    tag = context.options.release_tag
    if tag is None or not re.fullmatch(STABLE_TAG, tag):
        raise ToolingError("Stable publication requires a tag such as v1.2.3")
    context.tools.git.require_clean()
    source = context.tools.git.revision()
    if context.tools.git.revision(f"refs/tags/{tag}^{{commit}}") != source:
        raise ToolingError("Publication requires the release tag's original source commit")
    context.tools.git.require_remote_tag(tag, source)
    plan = ReleasePlan(tag[1:], tag, "", False)
    write_release_info(context, plan, source, released=False, output=False)
    verify_artifacts(context, source)
    if prepare and context.settings.helm_release_assets:
        HelmPackage.run(
            replace(
                context,
                options=replace(context.options, chart_version=plan.version, app_version=tag),
            )
        )
    paths = prepared_paths(context, plan, source)
    repository = release_repository(context)
    context.tools.github.check_existing_assets(repository, tag, paths)
    context.tools.github.create_stable_release(repository, tag)
    return publish_assets(context, repository, plan, source, paths)


class ReleasePreview(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        plan = context.tools.semantic_release.preview()
        payload = {
            "sourceCommit": context.tools.git.revision(),
            "version": plan.version,
            "tag": plan.tag,
            "releaseRequired": plan.required,
            "notes": plan.notes,
            "artifacts": artifact_paths(context, plan.version),
        }
        return TaskResult(json.dumps(payload, indent=2))


class ReleasePublish(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        if context.options.release_tag is not None:
            return publish_stable_tag(context, prepare=True)
        context.tools.git.require_clean()
        source = context.tools.git.revision()
        plan = context.tools.semantic_release.preview()
        if not plan.required:
            write_release_info(context, plan, source, released=False)
            return TaskResult(
                "No new release required; use rv release resume TAG for a partial release"
            )
        write_release_info(context, plan, source, released=False, output=False)
        verify_artifacts(context, source)
        if context.settings.helm_release_assets:
            HelmPackage.run(
                replace(
                    context,
                    options=replace(
                        context.options, chart_version=plan.version, app_version=plan.tag
                    ),
                )
            )
        paths = prepared_paths(context, plan, source)
        context.tools.github.verify()
        repository = release_repository(context)
        context.tools.github.check_existing_assets(repository, plan.tag, paths)
        if context.tools.git.revision() != source:
            raise ToolingError("Source commit changed during release preparation")
        context.tools.semantic_release.publish_version(plan)
        return publish_assets(context, repository, plan, source, paths)


class ReleaseResume(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        """Recover the existing tag at HEAD, reusing its prepared artifacts."""
        if context.options.release_tag and re.fullmatch(STABLE_TAG, context.options.release_tag):
            return publish_stable_tag(context, prepare=False)
        context.tools.git.require_clean()
        source = context.tools.git.revision()
        plan = context.tools.semantic_release.preview()
        tag = context.options.release_tag
        if not tag or plan.required or tag != plan.tag:
            raise ToolingError("Recovery requires the engine's current existing release tag")
        if context.tools.git.revision(f"refs/tags/{tag}^{{commit}}") != source:
            raise ToolingError("Recovery requires the release tag's original source commit")
        write_release_info(context, plan, source, released=False, output=False)
        paths = prepared_paths(context, plan, source)
        context.tools.github.verify()
        repository = release_repository(context)
        context.tools.github.check_existing_assets(repository, tag, paths)
        # A failed push may have created only the local tag. Git rejects a
        # different remote target; recovery never forces it or creates a tag.
        context.tools.git.push_tag(tag)
        context.fs.mkdir(context.root / "artifacts")
        context.tools.semantic_release.restore_release(tag)
        return publish_assets(context, repository, plan, source, paths)
