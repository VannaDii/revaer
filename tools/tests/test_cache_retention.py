"""Retain useful cache generations without deleting another workflow ref."""

import json
import sys
from dataclasses import replace
from datetime import UTC, datetime, timedelta
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.github import CacheEntry, GitHub
from revaer_tooling.process import Completed, Invocation, RunningProcess
from revaer_tooling.tasks.cache import WorkflowCachePrune, cache_evictions

PREFIX = "Linux-X64-revaer-compiler-stable-"
REF = "refs/pull/274/merge"


def entry(number: int, size: int = 300, *, ref: str = REF, key: str = "") -> CacheEntry:
    return CacheEntry(
        number,
        key or PREFIX + str(number),
        ref,
        size,
        datetime(2026, 10, 15, tzinfo=UTC) + timedelta(seconds=number),
    )


def test_cache_retention_keeps_two_generations_and_respects_ref_and_prefix() -> None:
    entries = (
        entry(1),
        entry(2),
        entry(3),
        entry(4, ref="refs/heads/main"),
        entry(5, key="Linux-X64-cargo-tool-cargo-nextest-pins"),
    )
    assert cache_evictions(entries, PREFIX, REF, PREFIX + "3", 1024) == (1,)


def test_cache_retention_applies_size_budget_even_to_two_generations() -> None:
    assert cache_evictions((entry(1, 800), entry(2, 800)), PREFIX, REF, PREFIX + "2", 1024) == (1,)
    assert cache_evictions((entry(1, 1200),), PREFIX, REF, PREFIX + "1", 1024) == (1,)


def test_an_older_publisher_cannot_prune_a_newer_publisher() -> None:
    assert cache_evictions((entry(1), entry(2)), PREFIX, REF, PREFIX + "1", 1024) == ()


def test_missing_published_cache_prevents_deletion() -> None:
    entries = (entry(1),)
    with pytest.raises(ToolingError, match="published cache"):
        cache_evictions(entries, PREFIX, REF, PREFIX + "2", 1024)


@pytest.mark.parametrize("prefix", ("", "Linux-X64-", "Linux-X64-cargo-tool-"))
def test_cache_retention_rejects_broad_prefixes(prefix: str) -> None:
    with pytest.raises(ToolingError, match="cache family"):
        cache_evictions((), prefix, REF, prefix + "key", 1024)


class CacheRunner:
    def __init__(self, response: str, fail_delete: bool = False) -> None:
        self.response = response
        self.fail_delete = fail_delete
        self.calls: list[Invocation] = []

    def start(self, invocation: Invocation) -> RunningProcess:
        raise AssertionError("Cache operations must not launch background processes")

    def run(self, invocation: Invocation) -> Completed:
        self.calls.append(invocation)
        if "DELETE" in invocation.argv:
            if self.fail_delete:
                raise CommandError("cache deletion failed", 1)
            return Completed(0, "")
        return Completed(0, self.response)


def response(entries: tuple[CacheEntry, ...]) -> str:
    # Separate pages prove that retention consumes the complete provider listing.
    return json.dumps(
        [
            {
                "actions_caches": [
                    {
                        "id": item.identifier,
                        "key": item.key,
                        "ref": item.ref,
                        "size_in_bytes": item.size,
                        "created_at": item.created.isoformat(),
                    }
                ]
            }
            for item in entries
        ]
    )


def cache_context(runner: CacheRunner, tmp_path: Path, *, ref: str = REF) -> Context:
    context = make_context(Options(cache_prefix=PREFIX, cache_ref=ref, cache_key=PREFIX + "3"))
    return Context(
        root=context.root,
        settings=replace(context.settings, workflow=replace(context.settings.workflow, ref=REF)),
        options=context.options,
        tools=replace(context.tools, github=GitHub(sys.executable, runner, tmp_path, {})),
        fs=context.fs,
        emit=context.emit,
        host=context.host,
        invoked_at=context.invoked_at,
        command_names=context.command_names,
    )


def test_cache_retention_uses_paginated_native_api_and_deletes_only_owned_ids(
    tmp_path: Path,
) -> None:
    runner = CacheRunner(response((entry(1), entry(2), entry(3), entry(4, ref="refs/heads/main"))))
    context = cache_context(runner, tmp_path)
    assert "Removed 1" in WorkflowCachePrune.run(context).message
    assert "--paginate" in runner.calls[0].argv
    assert "--slurp" in runner.calls[0].argv
    assert "ref=" + REF in runner.calls[0].argv
    assert "key=" + PREFIX in runner.calls[0].argv
    assert len(runner.calls) == 2
    assert runner.calls[1].argv[-1].endswith("/actions/caches/1")


@pytest.mark.parametrize("document", ("invalid json", "[{}]", '[{"actions_caches":[{"id":true}]}]'))
def test_malformed_cache_listing_cannot_trigger_deletions(tmp_path: Path, document: str) -> None:
    runner = CacheRunner(document)
    context = cache_context(runner, tmp_path)
    with pytest.raises(ToolingError):
        WorkflowCachePrune.run(context)
    assert len(runner.calls) == 1


def test_cache_deletion_failure_is_not_suppressed(tmp_path: Path) -> None:
    runner = CacheRunner(response((entry(1), entry(2), entry(3))), fail_delete=True)
    context = cache_context(runner, tmp_path)
    with pytest.raises(CommandError, match="cache deletion failed"):
        WorkflowCachePrune.run(context)


def test_cache_retention_cannot_select_another_workflow_ref(tmp_path: Path) -> None:
    runner = CacheRunner(response((entry(1), entry(2), entry(3))))
    context = cache_context(runner, tmp_path, ref="refs/heads/main")
    with pytest.raises(ToolingError, match="current workflow ref"):
        WorkflowCachePrune.run(context)
    assert not runner.calls
