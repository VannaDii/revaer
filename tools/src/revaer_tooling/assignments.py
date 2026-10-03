"""Read repository literal manifests; never execute or expand dotenv syntax."""

import re
import shlex

from .errors import ToolingError


def literal_assignments(source: str, label: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for number, line in enumerate(source.splitlines(), 1):
        try:
            tokens = shlex.split(line, comments=True)
        except ValueError as error:
            raise ToolingError(f"{label} line {number} has invalid quoting") from error
        if not tokens:
            continue
        if len(tokens) != 1 or any(character in tokens[0] for character in "$`\n\r\x00"):
            raise ToolingError(f"{label} line {number} must be one literal assignment")
        name, separator, value = tokens[0].partition("=")
        if (
            not separator
            or not re.fullmatch(r"[A-Z][A-Z0-9_]*", name)
            or name in values
            or not value
        ):
            raise ToolingError(f"{label} line {number} has an invalid or duplicate assignment")
        values[name] = value
    return values
