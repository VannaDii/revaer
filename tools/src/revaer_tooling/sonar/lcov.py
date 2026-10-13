"""Combine measured LLVM compilation records for one physical Rust source."""

from collections import defaultdict

from ..errors import ToolingError


def _count(value: str) -> int:
    if not value.isdecimal():
        raise ToolingError("LLVM coverage counters must be nonnegative integers")
    return int(value)


def _pair(value: str) -> tuple[str, str]:
    fields = value.split(",", 1)
    if len(fields) != 2 or not fields[1]:
        raise ToolingError("LLVM function coverage record is malformed")
    return fields[0], fields[1]


def _lines(records: list[list[str]]) -> dict[int, tuple[int, str]]:
    observations: dict[int, tuple[int, str]] = {}
    for record in records:
        for entry in record:
            if not entry.startswith("DA:"):
                continue
            fields = entry[3:].split(",")
            if len(fields) not in (2, 3):
                raise ToolingError("LLVM line coverage record is malformed")
            number, hits = _count(fields[0]), _count(fields[1])
            checksum = fields[2] if len(fields) == 3 else ""
            previous, original = observations.get(number, (0, checksum))
            if original != checksum:
                raise ToolingError("LLVM compilation records disagree on a source checksum")
            observations[number] = previous + hits, checksum
    return observations


def _functions(records: list[list[str]]) -> tuple[dict[str, str], dict[str, int]]:
    definitions: dict[str, str] = {}
    hits: dict[str, int] = defaultdict(int)
    for record in records:
        for entry in record:
            if entry.startswith("FN:"):
                location, name = _pair(entry[3:])
                if name in definitions and definitions[name] != location:
                    raise ToolingError("LLVM function definitions disagree across compilations")
                definitions[name] = location
            elif entry.startswith("FNDA:"):
                count, name = _pair(entry[5:])
                hits[name] += _count(count)
    return definitions, hits


def _branch_count(previous: int | None, value: str) -> int | None:
    if value == "-":
        return previous
    return (previous or 0) + _count(value)


def _branches(records: list[list[str]]) -> dict[tuple[str, str, str], int | None]:
    observations: dict[tuple[str, str, str], int | None] = {}
    for record in records:
        for entry in record:
            if not entry.startswith("BRDA:"):
                continue
            fields = entry[5:].split(",")
            if len(fields) != 4:
                raise ToolingError("LLVM branch coverage record is malformed")
            number, block, branch, count = fields
            key = number, block, branch
            observations[key] = _branch_count(observations.get(key), count)
    return observations


def merge_compilations(source: str, records: list[list[str]]) -> list[str]:
    """Sum real counters, union observed locations and recompute LCOV summaries.

    The complete engine report retains the separate compilation records. This
    derived report represents each source once, as required by the Rust sensor.
    """
    lines = _lines(records)
    definitions, functions = _functions(records)
    branches = _branches(records)
    merged = ["SF:" + source]
    merged.extend(f"FN:{location},{name}" for name, location in sorted(definitions.items()))
    merged.extend(f"FNDA:{hits},{name}" for name, hits in sorted(functions.items()))
    merged.extend(
        (f"FNF:{len(definitions)}", f"FNH:{sum(hits > 0 for hits in functions.values())}")
    )
    merged.extend(
        f"DA:{number},{hits}" + ("," + checksum if checksum else "")
        for number, (hits, checksum) in sorted(lines.items())
    )
    merged.extend((f"LF:{len(lines)}", f"LH:{sum(hits > 0 for hits, _ in lines.values())}"))
    merged.extend(
        f"BRDA:{number},{block},{branch},{hits if hits is not None else '-'}"
        for (number, block, branch), hits in sorted(branches.items())
    )
    merged.extend((f"BRF:{len(branches)}", f"BRH:{sum(bool(hits) for hits in branches.values())}"))
    return merged
