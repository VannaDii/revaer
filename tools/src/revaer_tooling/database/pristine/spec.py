"""The reviewed catalog projection is data, separate from query construction."""

import re
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType

from ...errors import ToolingError
from ...filesystem import FileSystem
from ...json_data import array_value, decode_unique, object_value, string_value


def name(value: str) -> str:
    if not re.fullmatch(r"[a-z][a-z0-9_]*", value):
        raise ToolingError("Catalog specification requires a lowercase SQL name")
    return value


@dataclass(frozen=True)
class Specification:
    columns: Mapping[str, tuple[str, ...]]
    excluded: Mapping[str, str]
    references: Mapping[str, str]
    expressions: Mapping[str, str]

    @staticmethod
    def load(fs: FileSystem, path: Path) -> "Specification":
        root = object_value(decode_unique(fs.read_regular_bytes(path).decode("utf-8")))
        if root.keys() != {"columns", "excluded", "references", "expressions"}:
            raise ToolingError("Catalog specification has unknown or missing sections")
        columns = {
            name(catalog): tuple(name(string_value(value)) for value in array_value(values))
            for catalog, values in object_value(root["columns"]).items()
        }
        if len(columns) != 27 or any(
            not fields or len(fields) != len(set(fields)) for fields in columns.values()
        ):
            raise ToolingError(
                "Catalog specification must cover all 27 classes with distinct columns"
            )
        mappings = [
            {name(key): string_value(value) for key, value in object_value(root[section]).items()}
            for section in ("excluded", "references", "expressions")
        ]
        for catalog in mappings[1].values():
            name(catalog)
        return Specification(
            MappingProxyType(columns), *(MappingProxyType(item) for item in mappings)
        )
