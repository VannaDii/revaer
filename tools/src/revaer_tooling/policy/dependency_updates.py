"""Allow dependency reference updates without exempting workflow logic changes."""

import copy
import re
import tomllib

from .formats import Document, yaml_document
from .workflows import mapping, steps


def action_references(document: Document) -> Document:
    result = copy.deepcopy(document)

    def reference(node: Document) -> None:
        uses = node.get("uses")
        if isinstance(uses, str) and re.fullmatch(r"[^./][^@\s]+@[0-9a-f]{40}", uses):
            node["uses"] = uses.rsplit("@", 1)[0]

    for value in mapping(result.get("jobs")).values():
        job = mapping(value)
        reference(job)
        for step in steps(job):
            reference(step)
    for step in steps(mapping(result.get("runs"))):
        reference(step)
    return result


def docker_references(source: str) -> str:
    lines: list[str] = []
    for line in source.splitlines():
        match = re.fullmatch(
            r"(ARG (?:RUST_BUILDER_IMAGE|ALPINE_RUNTIME_IMAGE|UV_IMAGE)=|# syntax=)(\S+)",
            line,
        )
        if match and re.fullmatch(r"[^@\s]+@sha256:[0-9a-f]{64}", match[2]):
            image = match[2].split("@", 1)[0]
            prefix, separator, name = image.rpartition("/")
            repository = (prefix + separator if separator else "") + name.split(":", 1)[0]
            lines.append(match[1] + repository)
        else:
            lines.append(line)
    return "\n".join(lines)


def reference_only(path: str, before: str, after: str) -> bool:
    if path.startswith((".github/workflows/", ".github/actions/")) and path.endswith(
        (".yml", ".yaml")
    ):
        return action_references(yaml_document(before, path)) == action_references(
            yaml_document(after, path)
        )
    if path == "Dockerfile":
        return docker_references(before) == docker_references(after)
    if path == "rust-toolchain.toml":
        return toolchain_reference(before, after)
    if path == "tools/launcher/pyproject.toml":
        return launcher_reference(before, after)
    return False


def toolchain_reference(before: str, after: str) -> bool:
    old, new = tomllib.loads(before), tomllib.loads(after)
    for document in (old, new):
        toolchain = document.get("toolchain")
        if not isinstance(toolchain, dict) or not re.fullmatch(
            r"\d+\.\d+(?:\.\d+)?", str(toolchain.get("channel")), flags=re.ASCII
        ):
            return False
        toolchain["channel"] = "<version>"
    return old == new


def launcher_reference(before: str, after: str) -> bool:
    old, new = tomllib.loads(before), tomllib.loads(after)
    for document in (old, new):
        build = document.get("build-system")
        if not isinstance(build, dict):
            return False
        requires = build.get("requires")
        if (
            not isinstance(requires, list)
            or len(requires) != 1
            or not re.fullmatch(r"uv_build==\d+\.\d+\.\d+", str(requires[0]), flags=re.ASCII)
        ):
            return False
        build["requires"] = ["uv_build"]
    return old == new
