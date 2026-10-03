"""Narrow untyped JSON at the boundary before tests use its values."""

import json
import math
from typing import cast

from .errors import ToolingError

type Json = bool | int | float | str | list[Json] | dict[str, Json] | None
type JsonObject = dict[str, Json]


def decode(value: str) -> Json:
    # json.loads defines this recursive value domain. The cast is confined to
    # its boundary; object/array/string access below still checks runtime types.
    try:
        return cast(Json, json.loads(value))
    except ValueError as error:
        raise ToolingError("Document is not valid JSON") from error


def decode_unique(value: str) -> Json:
    """Proof/provider evidence cannot collapse duplicate keys or admit NaN values."""

    def unique(pairs: list[tuple[str, Json]]) -> JsonObject:
        result: JsonObject = {}
        for key, item in pairs:
            if key in result:
                raise ValueError("Duplicate JSON key")
            result[key] = item
        return result

    def invalid_constant(token: str) -> None:
        raise ValueError("Nonfinite JSON number")

    def finite(token: str) -> float:
        number = float(token)
        if not math.isfinite(number):
            raise ValueError("Nonfinite JSON number")
        return number

    try:
        return cast(
            Json,
            json.loads(
                value,
                object_pairs_hook=unique,
                parse_constant=invalid_constant,
                parse_float=finite,
            ),
        )
    except ValueError as error:
        raise ToolingError("Evidence is not valid unique-key JSON") from error


def object_value(value: Json) -> JsonObject:
    if not isinstance(value, dict):
        raise ToolingError("Expected a JSON object")
    return value


def array_value(value: Json) -> list[Json]:
    if not isinstance(value, list):
        raise ToolingError("Expected a JSON array")
    return value


def string_value(value: Json) -> str:
    if not isinstance(value, str) or not value:
        raise ToolingError("Expected a nonempty JSON string")
    return value
