"""Project V8 block counters onto grammar-derived source-code locations.

V8 offsets and columns count UTF-16 code units, not Python characters or UTF-8
bytes. The narrowest containing range supplies each location's counter, including
zero-count nested branches. A line is covered when any of its locations ran.
Tree-sitter identifies source code, including modules, without running it.
Coverage remains zero until a matching V8 execution range supplies a count.
"""

from __future__ import annotations

import hashlib
import heapq
import re
from bisect import bisect_right
from dataclasses import dataclass
from typing import TYPE_CHECKING

from ..errors import ToolingError
from ..json_data import Json, array_value, object_value

if TYPE_CHECKING:
    from tree_sitter import Parser


def nonnegative(value: Json) -> int:
    if type(value) is not int or value < 0:
        raise ToolingError("V8 coverage requires nonnegative integer locations and counters")
    return value


def utf16_length(source: str) -> int:
    return len(source.encode("utf-16-le")) // 2


def source_digest(source: str) -> str:
    return hashlib.sha256(source.encode("utf-8")).hexdigest()


def source_lines(source: str) -> tuple[list[str], list[int]]:
    # JavaScript recognizes exactly these four line terminators. Python's
    # splitlines also splits vertical tabs and NEL, which would shift counters.
    lines = re.split(r"(?<=\n)|(?<=\r)(?!\n)|(?<=\u2028)|(?<=\u2029)", source)
    starts = [0]
    for line in lines:
        starts.append(starts[-1] + utf16_length(line))
    return lines, starts


@dataclass(frozen=True, order=True)
class Location:
    offset: int
    line: int


class JavaScriptSyntax:
    """The injected official grammar supplies locations; it supplies no hits.

    Named leaves identify code tokens rather than whitespace or punctuation.
    Statements and declarations also retain keyword-only lines such as return,
    break, and multiline declarations. Structural containers and comments do
    not create source-code records. V8's nested ranges decide whether code ran.
    """

    def __init__(self, parser: Parser) -> None:
        self.parser = parser

    def locations(self, source: str) -> tuple[Location, ...]:
        tree = self.parser.parse(source.encode("utf-16-le"), encoding="utf16le")
        if tree.root_node.has_error:
            raise ToolingError("JavaScript syntax contains a parse error")
        _, starts = source_lines(source)
        points: set[Location] = set()
        pending = [tree.root_node]
        containers = {
            "program",
            "statement_block",
            "class_body",
            "formal_parameters",
            "arguments",
            "comment",
            "hash_bang_line",
            "empty_statement",
        }
        while pending:
            node = pending.pop()
            pending.extend(reversed(node.named_children))
            if node.type in containers or node.end_byte == node.start_byte:
                continue
            if (
                node.named_child_count == 0
                or node.type.endswith(("_statement", "_declaration"))
                or node.type == "field_definition"
            ):
                # The parser consumes explicit UTF-16LE, so its byte offsets
                # divide exactly into V8 code units. Derive line numbers from
                # JavaScript line terminators rather than Tree-sitter's LF rows.
                offset = node.start_byte // 2
                points.add(Location(offset, bisect_right(starts, offset)))
        return tuple(sorted(points))


def line_counts(
    functions: list[Json], points: tuple[Location, ...], source_length: int
) -> dict[int, int]:
    ranges: list[tuple[int, int, int]] = []
    for entry in functions:
        function = object_value(entry)
        if not isinstance(function.get("isBlockCoverage"), bool):
            raise ToolingError("V8 function coverage is missing its block-coverage flag")
        for value in array_value(function.get("ranges")):
            block = object_value(value)
            start, end, count = (
                nonnegative(block.get(name)) for name in ("startOffset", "endOffset", "count")
            )
            if not start < end <= source_length:
                raise ToolingError("V8 coverage range exceeds the compiled source")
            ranges.append((start, end, count))
    ranges.sort()
    active: list[tuple[int, int, int, int]] = []
    index = 0
    result = dict.fromkeys((point.line for point in points), 0)
    # Sweep in source order. The heap avoids comparing every source location with
    # every function in large vendor bundles; expired enclosing ranges are
    # removed when they reach the top, and cannot hide a narrower live range.
    for point in sorted(points):
        while index < len(ranges) and ranges[index][0] <= point.offset:
            start, end, count = ranges[index]
            heapq.heappush(active, (end - start, -start, end, count))
            index += 1
        while active and active[0][2] <= point.offset:
            heapq.heappop(active)
        count = active[0][3] if active else 0
        result[point.line] = max(result[point.line], count)
    return result
