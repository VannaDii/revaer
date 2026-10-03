"""Verify that CI collected every scenario on exactly one requested shard.

Each pytest worker records the same deterministic selection for its phase. The
coordinator writes a separate completion summary per shard. Requiring both avoids
mistaking a collected, interrupted suite for a completed run. CI must download
only artifacts from the current workflow attempt into the input directory.
"""

from pathlib import Path

from ..errors import ToolingError
from ..filesystem import FileSystem
from ..json_data import Json, array_value, decode, object_value, string_value


def scenario_list(value: Json) -> list[str]:
    scenarios = [string_value(item) for item in array_value(value)]
    if scenarios != sorted(set(scenarios)):
        raise ToolingError("Shard scenarios must be sorted and unique")
    return scenarios


def verify_shards(
    filesystem: FileSystem, directory: Path, phases: tuple[str, ...], total: int
) -> None:
    if total < 1:
        raise ToolingError("The expected shard count must be positive")
    universes: dict[str, list[str]] = {}
    for index in range(1, total + 1):
        suffix = f"-shard-{index}" if total > 1 else ""
        summary_path = directory / f"python-e2e-summary{suffix}.json"
        if not summary_path.is_file():
            raise ToolingError(f"Shard {index} did not provide a completion summary")
        summary = object_value(decode(filesystem.read(summary_path)))
        if (
            summary.get("status") != "passed"
            or summary.get("shard") != index
            or summary.get("total_shards") != total
            or summary.get("phases") != dict.fromkeys(phases, "passed")
        ):
            raise ToolingError(f"Shard {index} did not complete every expected phase")
        for phase in phases:
            files = sorted(directory.glob(f"selection-{phase}{suffix}-*.json"))
            if not files:
                raise ToolingError(f"Shard {index} did not provide {phase} selection evidence")
            # xdist workers collect the complete phase before distributing its
            # selected scenarios; all workers must agree on that collection.
            for path in files:
                selection = object_value(decode(filesystem.read(path)))
                universe = scenario_list(selection.get("all"))
                chosen = scenario_list(selection.get("selected"))
                if (
                    not universe
                    or selection.get("phase") != phase
                    or selection.get("shard") != index
                    or selection.get("total_shards") != total
                    or universes.setdefault(phase, universe) != universe
                    or chosen != universe[index - 1 :: total]
                ):
                    raise ToolingError(f"Shard {index} has inconsistent {phase} selection evidence")
            if universe[index - 1 :: total]:
                label = "ui" if phase.startswith("ui-") else "api"
                coverage_files = sorted(directory.glob(f"{label}-coverage-{phase}{suffix}-*.json"))
                # Setup requests do not prove the phase's selected tests ran.
                recorded = {
                    string_value(item)
                    for path in coverage_files
                    if not path.name.endswith("-setup.json")
                    for item in array_value(decode(filesystem.read(path)))
                }
                if not recorded:
                    raise ToolingError(f"Shard {index} did not provide nonempty {phase} coverage")
