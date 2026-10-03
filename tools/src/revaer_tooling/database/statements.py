"""Map PostgreSQL tokens to the frozen candidate's exact byte boundaries.

Only the small indexing/line-alignment contract belongs here. PostgreSQL's
scanner owns quoting, escape strings, dollar tags and nested comment syntax.
"""

from dataclasses import dataclass
from importlib import import_module

from ..errors import ToolingError


@dataclass(frozen=True)
class Boundary:
    ordinal: int
    byte_count: int
    line_number: int


def _semicolons(source: str) -> tuple[int, ...]:
    try:
        parser = import_module("pglast.parser")
    except ImportError as error:
        raise ToolingError("Database scanning requires uv sync --locked --group dev") from error
    version: object = parser.get_postgresql_version()
    if not isinstance(version, tuple) or not version or version[0] != 16:
        raise ToolingError("Database scanning requires the locked PostgreSQL 16 binding")
    try:
        tokens: object = parser.scan(source)
    except parser.ParseError as error:
        # SQL may contain private fixture data. Retain the native exception for
        # debugging, but never echo its input into the ordinary CLI diagnostic.
        raise ToolingError(
            "PostgreSQL scanner rejected an incomplete or invalid SQL token"
        ) from error
    if not isinstance(tokens, (list, tuple)):
        raise ToolingError("PostgreSQL scanner returned an invalid token sequence")
    result = []
    previous = -1
    for token in tokens:
        if (
            not isinstance(token, tuple)
            or len(token) != 4
            or not isinstance(token[0], int)
            or isinstance(token[0], bool)
            or not isinstance(token[1], int)
            or isinstance(token[1], bool)
            or not isinstance(token[2], str)
            or not isinstance(token[3], str)
            or not previous < token[0] <= token[1] < len(source)
        ):
            raise ToolingError("PostgreSQL scanner returned an invalid token location")
        previous = token[1]
        if token[2] == "ASCII_59":
            if token[0] != token[1] or source[token[0]] != ";":
                raise ToolingError("PostgreSQL scanner returned an invalid semicolon")
            result.append(token[0])
    return tuple(result)


@dataclass(frozen=True)
class Statements:
    boundaries: tuple[Boundary, ...]

    @staticmethod
    def parse(raw: bytes) -> "Statements":
        if b"\x00" in raw:
            raise ToolingError("SQL input must not contain NUL bytes")
        try:
            source = raw.decode("utf-8")
        except UnicodeDecodeError as error:
            raise ToolingError("SQL input must use valid UTF-8") from error
        boundaries: list[Boundary] = []
        character_offset = byte_offset = 0
        line = 1
        for position in _semicolons(source):
            # Each intervening substring is visited once; large dump files do
            # not require a per-character offset table or quadratic rescanning.
            prefix = source[character_offset:position]
            byte_offset += len(prefix.encode("utf-8"))
            line += prefix.count("\n")
            character_offset = position
            end = byte_offset + 1
            newline = raw.find(b"\n", end)
            if newline >= 0 and not raw[end:newline].strip(b"\t\r "):
                end = newline + 1
            boundaries.append(Boundary(len(boundaries) + 1, end, line))
        if not boundaries:
            raise ToolingError("SQL candidate contains no complete statements")
        return Statements(tuple(boundaries))

    @property
    def count(self) -> int:
        return len(self.boundaries)

    def complete_prefix(self, byte_count: int) -> bool:
        return any(boundary.byte_count == byte_count for boundary in self.boundaries)

    def mapping(self) -> str:
        return "ordinal\tbyte_count\tline_number\n" + "".join(
            f"{boundary.ordinal}\t{boundary.byte_count}\t{boundary.line_number}\n"
            for boundary in self.boundaries
        )
