"""Bound published build caches within their owning workflow ref."""

import re

from ..context import Context, TaskResult
from ..errors import ToolingError
from ..external.github import CacheEntry
from .base import Task
from .release import release_repository


def validate_cache_scope(prefix: str, ref: str, current: str) -> None:
    if not re.fullmatch(
        r"(?:Linux|macOS|Windows)-(?:X64|ARM64)-revaer-(?:compiler-(?:stable|nightly)|coverage)-",
        prefix,
    ):
        raise ToolingError("Select an exact Revaer build cache family")
    if (
        not ref.startswith("refs/")
        or any(ord(c) < 32 for c in ref)
        or not current.startswith(prefix)
    ):
        raise ToolingError("Cache retention requires an explicit ref and published family key")


def cache_evictions(
    entries: tuple[CacheEntry, ...], prefix: str, ref: str, current: str, budget: int
) -> tuple[int, ...]:
    validate_cache_scope(prefix, ref, current)
    owned = sorted(
        (entry for entry in entries if entry.ref == ref and entry.key.startswith(prefix)),
        key=lambda entry: (entry.created, entry.identifier),
        reverse=True,
    )
    if not any(entry.key == current for entry in owned):
        raise ToolingError("The published cache is missing; no caches were deleted")
    # An overlapping older run must not evict the newer run's snapshots.
    if owned[0].key != current:
        return ()
    retained, size = 0, 0
    removed: list[int] = []
    for entry in owned:
        if retained < 2 and size + entry.size <= budget:
            retained += 1
            size += entry.size
        else:
            removed.append(entry.identifier)
    return tuple(removed)


class WorkflowCachePrune(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        options = context.options
        validate_cache_scope(options.cache_prefix, options.cache_ref, options.cache_key)
        if options.cache_ref != context.settings.workflow.ref:
            raise ToolingError("Cache retention must select the current workflow ref")
        repository = release_repository(context)
        entries = context.tools.github.caches(repository, options.cache_prefix, options.cache_ref)
        removed = cache_evictions(
            entries, options.cache_prefix, options.cache_ref, options.cache_key, 1024 * 1024 * 1024
        )
        for identifier in removed:
            context.tools.github.delete_cache(repository, identifier)
        return TaskResult(f"Removed {len(removed)} obsolete build caches from the selected ref")
