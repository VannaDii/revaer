"""Measured compilation counters form one source record without losing misses."""

import pytest
from revaer_tooling.errors import ToolingError
from revaer_tooling.sonar.lcov import merge_compilations


def test_union_sums_counters_and_retains_uncovered_lines_functions_and_branches() -> None:
    first = [
        "SF:source.rs",
        "FN:1,first",
        "FNDA:2,first",
        "DA:1,2,hash",
        "DA:2,0",
        "BRDA:1,0,0,2",
        "BRDA:1,0,1,-",
    ]
    second = [
        "SF:tests/../source.rs",
        "FN:1,first",
        "FNDA:3,first",
        "FN:3,unused",
        "FNDA:0,unused",
        "DA:1,3,hash",
        "DA:3,0",
        "BRDA:1,0,0,3",
        "BRDA:1,0,1,0",
        "BRDA:3,1,0,-",
    ]
    merged = merge_compilations("source.rs", [first, second])
    assert "DA:1,5,hash" in merged
    assert "DA:2,0" in merged
    assert "DA:3,0" in merged
    assert "FNDA:5,first" in merged
    assert "FNDA:0,unused" in merged
    assert "FNF:2" in merged
    assert "FNH:1" in merged
    assert "LF:3" in merged
    assert "LH:1" in merged
    assert "BRDA:1,0,0,5" in merged
    assert "BRDA:1,0,1,0" in merged
    assert "BRDA:3,1,0,-" in merged
    assert "BRF:3" in merged
    assert "BRH:1" in merged
    assert first[3] == "DA:1,2,hash"
    assert second[5] == "DA:1,3,hash"


@pytest.mark.parametrize(
    ("records", "message"),
    [
        ([["DA:1,-1"]], "nonnegative"),
        ([["DA:1,2,hash"], ["DA:1,3,other"]], "checksum"),
        ([["FN:1,first"], ["FN:2,first"]], "definitions disagree"),
        ([["DA:1"]], "malformed"),
        ([["FN:1"]], "malformed"),
        ([["FNDA:1"]], "malformed"),
        ([["BRDA:1,0"]], "malformed"),
    ],
)
def test_incompatible_or_malformed_measurements_fail(
    records: list[list[str]],
    message: str,
) -> None:
    with pytest.raises(ToolingError, match=message):
        merge_compilations("source.rs", records)
