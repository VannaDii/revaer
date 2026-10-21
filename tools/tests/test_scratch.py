"""Repo-local scratch owns only the current command's temporary files."""

import os
import subprocess
import sys
import tempfile
from pathlib import Path

import pytest
from revaer_tooling.bootstrap.scratch import repository_scratch, uses_test_scratch
from revaer_tooling.errors import ToolingError
from revaer_tooling.filesystem import FileSystem


@pytest.mark.parametrize("failure", (False, True))
def test_scratch_is_inherited_isolated_cleaned_and_restored(tmp_path: Path, failure: bool) -> None:
    parent = tmp_path / ".test-tmp"
    parent.mkdir()
    sentinel = parent / "other-run"
    sentinel.write_text("preserve")
    previous = {name: os.environ.get(name) for name in ("TMPDIR", "REVAER_TEST_ROOT")}
    cached = tempfile.tempdir
    created: list[Path] = []

    def run() -> None:
        with repository_scratch(tmp_path, FileSystem()):
            root = Path(tempfile.gettempdir())
            created.append(root)
            assert root.parent == parent
            result = subprocess.run(
                [sys.executable, "-c", "import tempfile; print(tempfile.gettempdir())"],
                check=True,
                capture_output=True,
                text=True,
                timeout=10,
            )
            assert Path(result.stdout.strip()) == parent
            with repository_scratch(tmp_path, FileSystem()):
                nested = Path(tempfile.gettempdir())
                created.append(nested)
                assert nested != root
                assert nested.parent == parent
            assert Path(tempfile.gettempdir()) == root
            assert root.is_dir()
            if failure:
                raise ToolingError("fixture failure")

    if failure:
        with pytest.raises(ToolingError, match="fixture failure"):
            run()
    else:
        run()
    assert all(not path.exists() for path in created)
    assert list(parent.iterdir()) == [sentinel]
    assert sentinel.read_text() == "preserve"
    assert tempfile.tempdir == cached
    assert {name: os.environ.get(name) for name in previous} == previous


def test_scratch_rejects_links_without_touching_the_target(tmp_path: Path) -> None:
    target = tmp_path / "caller"
    target.mkdir()
    (tmp_path / ".test-tmp").symlink_to(target, target_is_directory=True)
    with pytest.raises(ToolingError, match="symlink"), repository_scratch(tmp_path, FileSystem()):
        pytest.fail("linked scratch admitted")
    assert target.is_dir()
    assert not list(target.iterdir())


def test_child_temporary_root_survives_command_cleanup(tmp_path: Path) -> None:
    with repository_scratch(tmp_path, FileSystem()):
        environment = dict(os.environ)
    result = subprocess.run(
        [sys.executable, "-c", "import tempfile; print(tempfile.gettempdir())"],
        env=environment,
        check=True,
        capture_output=True,
        text=True,
        timeout=10,
    )
    child_root = tmp_path / ".test-tmp"
    assert Path(result.stdout.strip()) == child_root
    with tempfile.TemporaryDirectory(dir=child_root) as child:
        assert Path(child).parent == tmp_path / ".test-tmp"


def test_runtime_image_preparation_does_not_require_writable_source() -> None:
    assert uses_test_scratch("ci-qualify")
    assert uses_test_scratch("ui-e2e")
    assert not uses_test_scratch("container-runtime")
    assert not uses_test_scratch("container-build")
