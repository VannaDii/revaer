"""Prepare the complete input set and protect authored files during cleanup."""

import json
import shutil
import subprocess
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.native import BRIDGE_HEADERS
from revaer_tooling.tasks.sonar import SonarPrepareSources, SonarVerifyInputs


@pytest.fixture
def preparation_context(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    subprocess.run(["git", "init", str(tmp_path)], check=True, capture_output=True, timeout=10)
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").write_text("value = 1\n")
    (tmp_path / "coverage").mkdir()
    (tmp_path / ".gitignore").write_text("coverage/\nartifacts/\n")
    monkeypatch.chdir(tmp_path)
    context = make_context(Options())
    coverage = tmp_path / "coverage"
    (coverage / "lcov.info").write_text("SF:crates/sample/src/lib.rs\nDA:1,1\nend_of_record\n")
    (coverage / "js-lcov.info").write_text(
        "SF:crates/ui/static/app.js\n"
        + "".join(f"DA:{number},{int(number != 1000)}\n" for number in range(1, 1001))
        + "end_of_record\n"
    )
    (coverage / "script-coverage.xml").write_text(
        '<coverage version="1"><file path="setup.sh">'
        '<lineToCover lineNumber="2" covered="true"/>'
        '<lineToCover lineNumber="3" covered="false"/></file></coverage>'
    )
    (coverage / "python.xml").write_text(
        "<coverage><sources><source>.</source></sources><packages><package><classes>"
        '<class filename="tools/src/revaer_tooling/cli.py"><lines>'
        '<line number="1" hits="1"/></lines></class></classes></package></packages></coverage>'
    )
    native = tmp_path / "crates/revaer-torrent-libt/src/ffi/session.cpp"
    native.parent.mkdir(parents=True)
    native.write_text("int value = 1;\n")
    (coverage / "llvm-cov.txt").write_text(f"{native}:\n  1| 1|int value = 1;\n")
    include = coverage / "cxxbridge/include"
    for name in BRIDGE_HEADERS:
        context.fs.write(include / name, "// retained fixture header\n")
    (coverage / "compile_commands.json").write_text(
        json.dumps(
            [
                {
                    "directory": str(tmp_path.resolve()),
                    "file": str(native),
                    "arguments": ["c++", "-I", str(include), "-c", str(native)],
                }
            ]
        )
    )
    return replace(
        context,
        settings=replace(
            context.settings, sonar=replace(context.settings.sonar, native_required=True)
        ),
    )


def test_input_task_requires_every_report_and_retained_header(preparation_context: Context) -> None:
    context = preparation_context
    assert "verified" in SonarVerifyInputs.run(context).message
    header = context.root / "coverage/cxxbridge/include" / BRIDGE_HEADERS[0]
    header.unlink()
    with pytest.raises(ToolingError, match="retained header"):
        SonarVerifyInputs.run(context)
    (context.root / "coverage/python.xml").unlink()
    with pytest.raises(ToolingError, match="missing or empty"):
        SonarVerifyInputs.run(context)


def test_source_preparation_preserves_coverage_and_never_deletes_authored_content(
    preparation_context: Context,
) -> None:
    context = preparation_context
    paths = (
        "tests/node_modules/package/index.js",
        "release/node_modules/package/index.js",
        "tests/support/api/schema.ts",
        "crates/revaer-ui/dist-serve/index.html",
        "tests/test-results/result.json",
        "tests/playwright-report/index.html",
        "tests/logs/api.log",
        "tools/src/revaer_tooling/__pycache__/cli.cpython-313.pyc",
        "tests/__pycache__/conftest.cpython-313.pyc",
    )
    for name in paths:
        context.fs.write(context.root / name, "generated\n")
    subprocess.run(
        ["git", "add", "tools/src/revaer_tooling/cli.py"],
        cwd=context.root,
        check=True,
        capture_output=True,
        timeout=10,
    )
    SonarPrepareSources.run(context)
    assert all(not (context.root / path).exists() for path in paths)
    assert (context.root / "tools/src/revaer_tooling/cli.py").read_text() == "value = 1\n"
    assert not (context.root / "artifacts/sonar/browser-evidence").exists()
    assert (context.root / "coverage/python.xml").is_file()
    # A checkout still tracking an older generated schema must be migrated
    # explicitly; cleanup cannot erase it to satisfy scanner policy.
    protected = context.root / "tests/support/api/schema.ts"
    context.fs.write(protected, "tracked\n")
    subprocess.run(
        ["git", "add", "tests/support/api/schema.ts"],
        cwd=context.root,
        check=True,
        capture_output=True,
        timeout=10,
    )
    with pytest.raises(ToolingError, match="tracked content"):
        SonarPrepareSources.run(context)
    assert protected.read_text() == "tracked\n"


def test_source_preparation_rejects_links_to_another_checkout(
    preparation_context: Context, tmp_path: Path
) -> None:
    context = preparation_context
    source = context.root / "tests/test-results"
    source.parent.mkdir(parents=True)
    outside = tmp_path / "unrelated"
    outside.mkdir()
    (outside / "keep.txt").write_text("keep\n")
    source.symlink_to(outside, target_is_directory=True)
    with pytest.raises(ToolingError, match="Refusing cleanup outside"):
        SonarPrepareSources.run(context)
    assert (outside / "keep.txt").read_text() == "keep\n"
    source.unlink()
    shutil.rmtree(outside)
