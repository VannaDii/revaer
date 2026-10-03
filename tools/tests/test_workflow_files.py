"""Exercise command-file framing, append semantics and filesystem boundaries."""

import os
from pathlib import Path

import pytest
from revaer_tooling.automation.files import write_values
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem


def records(path: Path) -> dict[str, str]:
    """Read GitHub's multiline framing without interpreting the payload."""
    lines = iter(path.read_text().splitlines())
    result: dict[str, str] = {}
    for header in lines:
        name, delimiter = header.split("<<", 1)
        content: list[str] = []
        for line in lines:
            if line == delimiter:
                break
            content.append(line)
        else:
            raise AssertionError("Incomplete workflow output record")
        result[name] = "\n".join(content)
    return result


def test_outputs_append_and_round_trip_literal_multiline_values(tmp_path: Path) -> None:
    path = tmp_path / "output"
    values = {"tag": "v1.2.3", "note": "é\nEOF\n$(not-a-command)\nkey=value", "empty": ""}
    fs = FileSystem()
    write_values(fs, path, values)
    write_values(fs, path, {"released": "true"})
    assert records(path) == {**values, "released": "true"}
    assert path.stat().st_mode & 0o777 == 0o600


@pytest.mark.parametrize("values", ({"bad\nname": "value"}, {"note": "nul\0value"}))
def test_invalid_record_cannot_append_a_partial_success(
    tmp_path: Path, values: dict[str, str]
) -> None:
    path = tmp_path / "environment"
    path.write_text("previous record\n")
    with pytest.raises(ToolingError, match="valid names"):
        write_values(FileSystem(), path, {"valid": "would be partial", **values})
    assert path.read_text() == "previous record\n"


def test_symlink_and_nonregular_command_files_are_rejected(tmp_path: Path) -> None:
    original = tmp_path / "unrelated"
    original.write_text("preserve")
    link = tmp_path / "link"
    link.symlink_to(original)
    with pytest.raises(OSError):
        write_values(FileSystem(), link, {"value": "replacement"})
    assert original.read_text() == "preserve"
    pipe = tmp_path / "pipe"
    os.mkfifo(pipe)
    # A malicious FIFO must fail promptly even without a reader at the far end.
    with pytest.raises(OSError):
        write_values(FileSystem(), pipe, {"value": "payload"})
    with pytest.raises(ToolingError, match="regular file"):
        write_values(FileSystem(), Path("/dev/null"), {"value": "payload"})
