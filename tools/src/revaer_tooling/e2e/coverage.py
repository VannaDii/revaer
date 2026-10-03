"""Record real API operations and UI navigation, separately from line coverage."""

import json
from collections.abc import Iterable
from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import JsonObject, array_value, decode, object_value, string_value

API_METHODS = frozenset(("get", "post", "put", "patch", "delete"))


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
    files = sorted(directory.glob(f"{label.lower()}-coverage-*.json"))
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
