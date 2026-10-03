"""Parse configuration without hiding ambiguous keys or applying YAML coercion.

GitHub Actions treats keys such as ``on`` as text. PyYAML's ordinary YAML 1.1
loader instead coerces them to booleans and silently replaces duplicate keys.
Inspecting its syntax tree preserves the authored strings and source locations.
Aliases are rejected before composition, including otherwise harmless aliases:
the workflow contract requires each setting to have one explicit definition.
"""

import re
from dataclasses import dataclass

import yaml
from yaml.events import AliasEvent
from yaml.nodes import MappingNode, Node, ScalarNode, SequenceNode

from ..errors import ToolingError

type Value = str | list[Value] | dict[str, Value]
type Document = dict[str, Value]


def yaml_document(source: str, path: str) -> Document:
    """Require one mapping document, unique scalar keys, and no YAML aliases."""
    try:
        for event in yaml.parse(source, Loader=yaml.BaseLoader):
            if isinstance(event, AliasEvent):
                raise ToolingError(f"{path}: YAML aliases are forbidden")
        nodes = list(yaml.compose_all(source, Loader=yaml.BaseLoader))
        if len(nodes) != 1:
            raise ToolingError(f"{path}: expected one YAML document")
        value = _yaml_value(nodes[0], path)
        if not isinstance(value, dict):
            raise ToolingError(f"{path}: root must be a mapping")
        return value
    except yaml.YAMLError as error:
        raise ToolingError(f"{path}: invalid YAML ({type(error).__name__})") from error


def _yaml_value(node: Node, path: str) -> Value:
    if isinstance(node, ScalarNode):
        value = node.value
        if not isinstance(value, str):
            raise ToolingError(f"{path}: YAML scalars must be strings")
        return value
    if isinstance(node, SequenceNode):
        return [_yaml_value(child, path) for child in node.value]
    if isinstance(node, MappingNode):
        document: Document = {}
        for key, value in node.value:
            if not isinstance(key, ScalarNode):
                raise ToolingError(f"{path}: non-scalar mapping key")
            if key.value in document:
                raise ToolingError(f"{path}:{key.start_mark.line + 1}: duplicate key {key.value}")
            if key.value == "<<":
                raise ToolingError(f"{path}: YAML merge keys are forbidden")
            document[key.value] = _yaml_value(value, path)
        return document
    raise ToolingError(f"{path}: unsupported YAML node {type(node).__name__}")


@dataclass(frozen=True)
class Property:
    key: str
    value: str
    line: int


def properties(source: str, path: str) -> tuple[Property, ...]:
    """Read Java properties while rejecting spelling that can conceal an override.

    All reviewed keys are plain ASCII. Rejecting escaped keys and continuation
    lines makes duplicate detection unambiguous. Values still use Java escape
    semantics so an escaped ``true`` or path cannot bypass the value policy.
    """
    result: list[Property] = []
    first_lines: dict[str, int] = {}
    for line, text in enumerate(source.splitlines(), 1):
        if not text.strip() or text.lstrip().startswith(("#", "!")):
            continue
        location = f"{path}:{line}"
        if text[0] in " \t\f":
            raise ToolingError(f"{location}: leading property whitespace is forbidden")
        if (len(text) - len(text.rstrip("\\"))) % 2:
            raise ToolingError(f"{location}: property continuations are forbidden")
        # The first *unescaped* separator owns the split. An escaped separator
        # remains part of the key, which is then rejected rather than truncated.
        boundary = re.search(r"(?<!\\)[=: \t\f]", text)
        key = text[: boundary.start()] if boundary else text
        if "\\" in key:
            raise ToolingError(f"{location}: escaped property keys are forbidden")
        raw_value = text[boundary.start() :] if boundary else ""
        raw_value = raw_value.lstrip(" \t\f")
        if raw_value.startswith(("=", ":")):
            raw_value = raw_value[1:]
        value = _decode_property(raw_value.lstrip(" \t\f"), location)
        if key in first_lines:
            raise ToolingError(
                f"{location}: duplicate property {key}; first defined on line {first_lines[key]}"
            )
        first_lines[key] = line
        result.append(Property(key, value, line))
    return tuple(result)


def _decode_property(source: str, location: str) -> str:
    output: list[str] = []
    index = 0
    while index < len(source):
        character = source[index]
        index += 1
        if character != "\\":
            output.append(character)
            continue
        if index == len(source):
            raise ToolingError(f"{location}: dangling property escape")
        escaped = source[index]
        index += 1
        if escaped == "u":
            digits = source[index : index + 4]
            if not re.fullmatch(r"[0-9a-fA-F]{4}", digits):
                raise ToolingError(f"{location}: invalid Unicode property escape")
            output.append(chr(int(digits, 16)))
            index += 4
        else:
            output.append({"t": "\t", "n": "\n", "r": "\r", "f": "\f"}.get(escaped, escaped))
    return "".join(output)
