"""Small semantic checks shared by the repository's workflow contracts.

These helpers compare parsed values and command arguments, not source substrings.
A comment, folded YAML block, or unrelated environment value cannot satisfy an
executable requirement. Structural validation runs first and rejects duplicates.
"""

import shlex
from collections.abc import Mapping, Sequence

from .formats import Document
from .workflows import mapping, named_steps, steps


def runs(step: Document, *arguments: str) -> bool:
    value = step.get("run")
    if not isinstance(value, str):
        return False
    try:
        command = shlex.split(value)
    except ValueError:
        return False
    if command[:4] == ["uv", "run", "--locked", "--"]:
        command = command[4:]
    return command in (["rv", *arguments], [".venv/bin/rv", *arguments])


class Contract:
    """Collect actionable findings for one already-parsed workflow."""

    def __init__(self, path: str, document: Document) -> None:
        self.path = path
        self.document = document
        self.failures: list[str] = []

    def require(self, condition: bool, message: str) -> None:
        if not condition:
            self.failures.append(f"{self.path}: {message}")

    def job(self, name: str) -> Document:
        result = mapping(mapping(self.document.get("jobs")).get(name))
        self.require(bool(result), f"required job {name} is missing")
        return result

    def step(self, job: Document, name: str) -> Document:
        result = named_steps(job).get(name, {})
        self.require(bool(result), f"required step {name!r} is missing")
        return result

    def order(self, job: Document, names: Sequence[str]) -> None:
        positions = {
            step.get("name"): index
            for index, step in enumerate(steps(job))
            if isinstance(step.get("name"), str)
        }
        actual = [positions[name] for name in names if name in positions]
        self.require(
            len(actual) == len(names) and actual == sorted(actual),
            "required step order: " + " -> ".join(names),
        )

    def paths(self, step: Document, required: Sequence[str]) -> None:
        value = mapping(step.get("with")).get("path", "")
        actual = {line.strip() for line in value.splitlines()} if isinstance(value, str) else set()
        for path in required:
            self.require(path in actual, f"artifact {step.get('name')!r} must retain {path}")
        self.require(
            mapping(step.get("with")).get("if-no-files-found") == "error",
            f"artifact {step.get('name')!r} must fail when evidence is missing",
        )


def workflow(documents: Mapping[str, Document], name: str) -> Contract:
    path = f".github/workflows/{name}.yml"
    contract = Contract(path, documents.get(path, {}))
    contract.require(bool(contract.document), "required workflow is missing")
    return contract
