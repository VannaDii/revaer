"""Check uv ownership and the declared interface between setup and its callers."""

from collections.abc import Mapping

from .formats import Document
from .workflows import SETUP_PATH, mapping, steps


def setup_workflow_findings(documents: Mapping[str, Document]) -> list[str]:
    failures: list[str] = []
    action = documents.get(SETUP_PATH, {})
    inputs = mapping(action.get("inputs"))
    if not inputs:
        failures.append("Revaer setup action must declare its inputs")
    actions = steps(mapping(action.get("runs")))
    uv = [
        (index, step)
        for index, step in enumerate(actions)
        if str(step.get("uses", "")).startswith("astral-sh/setup-uv@")
    ]
    sync = [index for index, step in enumerate(actions) if step.get("run") == "uv sync --locked"]
    setup = [
        index
        for index, step in enumerate(actions)
        if str(step.get("run", "")).startswith("uv run --locked -- rv setup ")
    ]
    if len(uv) != 1 or len(sync) != 1 or not setup:
        failures.append("Setup requires one official setup-uv action, one locked sync and rv setup")
    else:
        options = mapping(uv[0][1].get("with"))
        if options.get("version-file") != "pyproject.toml" or any(
            key in options for key in ("version", "python-version", "activate-environment")
        ):
            failures.append("setup-uv must consume the project pins without environment overrides")
        if not uv[0][0] < sync[0] < min(setup):
            failures.append("Install uv, synchronize the lock, then invoke rv setup")
        if any("if" in actions[index] for index in (uv[0][0], sync[0], min(setup))):
            failures.append(
                "The uv installation, locked sync and initial rv setup must be unconditional"
            )
    for path, document in documents.items():
        for job in mapping(document.get("jobs")).values():
            for step in steps(mapping(job)):
                if step.get("uses") != "./.github/actions/setup-revaer":
                    continue
                values = step.get("with", {})
                if not isinstance(values, dict):
                    failures.append(f"{path}: setup inputs must be a mapping")
                    continue
                unknown = values.keys() - inputs.keys()
                if unknown:
                    failures.append(f"{path}: unknown setup inputs: {', '.join(sorted(unknown))}")
    return failures
