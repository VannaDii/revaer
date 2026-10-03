"""Exercise repeated Cargo exports and reject incomplete native analyzer inputs."""

import json
import subprocess
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.native import (
    BRIDGE_HEADERS,
    SonarCompileDatabase,
    verify_compilation_database,
)


def test_repeated_cargo_export_regenerates_missing_inputs_and_invalidates_failed_output(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    crate = tmp_path / "crates/revaer-torrent-libt"
    (crate / "src/ffi").mkdir(parents=True)
    source = crate / "src/ffi/session.cpp"
    source.write_text("int value() { return 42; }\n")
    (crate / "src/lib.rs").write_text("pub const VALUE: u32 = 42;\n")
    (crate / "Cargo.toml").write_text(
        '[package]\nname="revaer-torrent-libt"\nversion="0.1.0"\nedition="2024"\n'
    )
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nresolver="3"\nmembers=["crates/revaer-torrent-libt"]\n'
    )
    includes = tmp_path / "coverage/cxxbridge/include"
    command = {
        "directory": str(crate),
        "file": str(source),
        "arguments": ["clang++", "-I", str(includes), "-c", str(source)],
    }
    document = json.dumps([command])
    build = crate / "build.rs"
    build.write_text(
        "fn main() -> Result<(), Box<dyn std::error::Error>> {\n"
        'println!("cargo:rerun-if-env-changed=REVAER_NATIVE_COMPILE_COMMANDS_PATH");\n'
        'let target = std::env::var("REVAER_NATIVE_COMPILE_COMMANDS_PATH")?;\n'
        'let parent = std::path::Path::new(&target).parent().ok_or("missing parent")?;\n'
        'for name in ["rust/cxx.h", "revaer-torrent-libt/src/ffi/bridge.rs.h"] {\n'
        'let header = parent.join("cxxbridge/include").join(name);\n'
        'std::fs::create_dir_all(header.parent().ok_or("missing header parent")?)?;\n'
        'std::fs::write(header, "// retained bridge header\\n")?; }\n'
        f'std::fs::write(target, r###"{document}"###)?; Ok(()) }}\n'
    )
    monkeypatch.chdir(tmp_path)
    subprocess.run(["cargo", "generate-lockfile", "--offline"], check=True, timeout=30)
    context = make_context(Options())
    output = tmp_path / "coverage/compile_commands.json"
    context.fs.write(output, "stale")
    for _ in range(2):
        SonarCompileDatabase.run(context)
        assert json.loads(output.read_text()) == [command]
        for name in BRIDGE_HEADERS:
            assert (includes / name).read_text() == "// retained bridge header\n"
        verify_compilation_database(output.read_text(), tmp_path, source, includes)
        output.unlink()
    context.fs.write(output, "old success")
    build.write_text('fn main() -> Result<(), String> { Err("fixture failure".into()) }\n')
    with pytest.raises(ToolingError):
        SonarCompileDatabase.run(context)
    assert not output.exists()
    # A successful build that stops producing the expected file must also fail.
    build.write_text('fn main() { println!("cargo:rerun-if-changed=build.rs"); }\n')
    with pytest.raises(ToolingError, match="did not produce"):
        SonarCompileDatabase.run(context)
    assert not list(output.parent.glob("native-input-*"))


def test_retained_headers_are_required_and_include_paths_are_parsed(tmp_path: Path) -> None:
    includes = tmp_path / "coverage with spaces/cxxbridge/include"
    source = tmp_path / "session.cpp"
    source.touch()
    for name in BRIDGE_HEADERS:
        header = includes / name
        header.parent.mkdir(parents=True, exist_ok=True)
        header.write_text("// retained header\n")
    entry = {
        "directory": str(tmp_path),
        "file": str(source),
        "command": f'clang++ -I"{includes}" -c "{source}"',
    }
    verify_compilation_database(json.dumps([entry]), tmp_path, source, includes)
    with pytest.raises(ToolingError, match="retained bridge headers"):
        verify_compilation_database(json.dumps([entry]), tmp_path, source, tmp_path / "absent")
    (includes / BRIDGE_HEADERS[0]).unlink()
    with pytest.raises(ToolingError, match="missing retained header"):
        verify_compilation_database(json.dumps([entry]), tmp_path, source, includes)


@pytest.mark.parametrize(
    "mutation",
    ["json", "empty", "entry", "missing", "directory", "outside", "arguments", "command", "bridge"],
)
def test_native_input_validation_rejects_incomplete_or_foreign_commands(
    tmp_path: Path, mutation: str
) -> None:
    source = tmp_path / "bridge.cpp"
    source.touch()
    entry: dict[str, object] = {
        "directory": str(tmp_path),
        "file": "bridge.cpp",
        "command": "clang++ -c bridge.cpp",
    }
    document = json.dumps([entry])
    required = source
    if mutation == "json":
        document = "{broken"
    elif mutation == "empty":
        document = "[]"
    elif mutation == "entry":
        document = "[42]"
    elif mutation == "missing":
        entry.pop("file")
    elif mutation == "directory":
        entry["directory"] = "."
    elif mutation == "outside":
        entry["file"] = str(Path(__file__).resolve())
    elif mutation == "arguments":
        entry["arguments"] = [42]
    elif mutation == "command":
        entry["command"] = " "
    elif mutation == "bridge":
        required = tmp_path / "required.cpp"
    if mutation not in ("json", "empty", "entry"):
        document = json.dumps([entry])
    with pytest.raises(ToolingError):
        verify_compilation_database(document, tmp_path, required)


def test_native_command_string_and_relative_source_are_supported(tmp_path: Path) -> None:
    source = tmp_path / "with space.cpp"
    source.touch()
    verify_compilation_database(
        json.dumps(
            [
                {
                    "directory": str(tmp_path),
                    "file": source.name,
                    "command": 'c++ -c "with space.cpp"',
                }
            ]
        ),
        tmp_path,
        source,
    )
