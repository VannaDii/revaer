"""Record real API operations and UI navigation, separately from line coverage."""

import json
from collections.abc import Iterable
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import JsonObject, array_value, decode, object_value, string_value
from .shards import verify_shards

API_METHODS = frozenset(("get", "post", "put", "patch", "delete"))


def distributed_runs(
    filesystem: FileSystem, root: Path, expected: tuple[str, ...], total: int
) -> tuple[tuple[Path, str, str], ...]:
    """Validate each producer before exposing its named execution inputs.

    Actions downloads artifacts from this workflow run into separate directories;
    keeping them separate prevents summaries or V8 captures overwriting a shard.
    The existing partition validator still proves every scenario was selected.
    """
    producers = root / "coverage/producers"
    if producers.is_symlink():
        raise ToolingError("Coverage producers must not be a symlink")
    if not producers.is_dir():
        return ()
    api_phases = tuple(phase for phase in expected if phase.startswith("api-"))
    browsers = tuple(phase for phase in expected if phase.startswith("ui-"))
    complete_api_phases = (
        ("api-none", "api-api-key"),
        ("api-none-missing-catalog", "api-none", "api-api-key-missing-catalog", "api-api-key"),
    )
    if api_phases not in complete_api_phases or not browsers:
        raise ToolingError(
            "Distributed analysis requires anonymous, authenticated and browser phases"
        )
    result: list[tuple[Path, str, str]] = []
    groups: list[tuple[str, tuple[str, ...], int]] = [("api-e2e-coverage", api_phases, 1)]
    groups.extend(("ui-e2e-" + phase.removeprefix("ui-"), (phase,), total) for phase in browsers)
    for name, phases, count in groups:
        directories = {
            index: producers / (name + f"-shard-{index}" if count > 1 else name)
            for index in range(1, count + 1)
        }
        verify_shards(
            filesystem,
            producers,
            phases,
            count,
            {index: path / ".test-tmp/e2e/test-results" for index, path in directories.items()},
        )
        for index, path in directories.items():
            verify_producer(filesystem, path, phases, suffix=f"-shard-{index}" if count > 1 else "")
            suffix = f"-shard-{index}" if count > 1 else ""
            result.extend((path, phase, suffix) for phase in phases)
    return tuple(result)


def verify_producer(
    filesystem: FileSystem, path: Path, phases: tuple[str, ...], *, suffix: str
) -> None:
    if path.is_symlink():
        raise ToolingError("Coverage producer must not be a symlink")
    summary = object_value(
        decode(
            filesystem.read(path / f".test-tmp/e2e/test-results/python-e2e-summary{suffix}.json")
        )
    )
    if "ui-chromium" in phases and summary.get("browser_coverage") is not True:
        raise ToolingError("Chromium shards must collect browser line coverage")


def verify_analysis_run(
    summary: JsonObject, expected: tuple[str, ...], *, browser: bool = False
) -> None:
    """Analysis consumes all named phases of one completed, unsharded run."""
    if (
        summary.get("status") != "passed"
        or any(
            type(summary.get(key)) is not int or summary.get(key) != 1
            for key in ("shard", "total_shards")
        )
        or summary.get("phases") != dict.fromkeys(expected, "passed")
        or not {"api-none", "api-api-key"}.issubset(expected)
        or not any(name.startswith("ui-") for name in expected)
    ):
        raise ToolingError("Analysis coverage requires a completed, unsharded API and UI run")
    if browser and (summary.get("browser_coverage") is not True or "ui-chromium" not in expected):
        raise ToolingError(
            "JavaScript analysis coverage requires a completed Chromium coverage run"
        )


class RouteCoverage:
    def __init__(self, path: Path, filesystem: FileSystem) -> None:
        self.path = path
        self.filesystem = filesystem
        self.covered: set[str] = set()

    def record(self, operation: str) -> None:
        self.covered.add(operation)
        # Write after each observation, so a timeout or later failing test still
        # leaves usable evidence. Each worker owns a distinct file.
        self.filesystem.write(self.path, json.dumps(sorted(self.covered)) + "\n")


def required_api_operations(document: str) -> set[str]:
    paths = object_value(object_value(decode(document)).get("paths"))
    required = {
        f"{method.upper()} {route}"
        for route, methods in paths.items()
        for method in object_value(methods)
        if method in API_METHODS
    }
    if not required:
        raise ToolingError("The OpenAPI document contains no API operations")
    return required


def verify_routes(
    filesystem: FileSystem, directory: Path, label: str, required: Iterable[str]
) -> None:
    files = sorted(directory.rglob(f"{label.lower()}-coverage-*.json"))
    if not files:
        raise ToolingError(f"{label} coverage files were not produced")
    observed = {
        string_value(value)
        for path in files
        for value in array_value(decode(filesystem.read(path)))
    }
    missing = sorted(set(required) - observed)
    if missing:
        raise ToolingError(
            f"{label} coverage missing {len(missing)} entries:\n" + "\n".join(missing)
        )
