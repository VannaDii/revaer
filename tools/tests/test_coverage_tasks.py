"""Exercise real Rust/C++ instrumentation, report retention, and per-crate gates."""

import json
import os
import shutil
import subprocess
import tomllib
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks.coverage import Coverage, CoverageReport, rust_line_gate


@pytest.fixture
def coverage_workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    channel = tomllib.loads((source / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    monkeypatch.setenv("RUSTUP_TOOLCHAIN", channel)
    monkeypatch.setenv("REVAER_TEST_DATABASE_URL", "postgres://fixture.invalid/unused")
    for name in ("CARGO_TARGET_DIR", "CARGO_LLVM_COV_TARGET_DIR", "RUSTC_COMMAND", "RUSTC"):
        monkeypatch.delenv(name, raising=False)
    # CI supplies Clang on PATH. Homebrew keeps its LLVM formula keg-only;
    # explicitly select that installed prerequisite for this native fixture.
    for variable, command in (("CC", "clang"), ("CXX", "clang++")):
        executable = os.environ.get(variable) or shutil.which(f"{command}-19")
        formula = Path("/opt/homebrew/opt/llvm@19/bin") / command
        if executable is None and formula.is_file():
            executable = str(formula)
        monkeypatch.setenv(variable, executable or command)
    monkeypatch.chdir(tmp_path)
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    shutil.copy2(source / "tools/versions.toml", tmp_path / "tools/versions.toml")
    (tmp_path / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["crates/*"]\n')
    for name in ("native", "pure"):
        crate = tmp_path / "crates" / name
        (crate / "src").mkdir(parents=True)
        (crate / "Cargo.toml").write_text(
            f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2024"\n'
            "[features]\nextra = []\n"
        )
    native = tmp_path / "crates/native"
    (native / "build.rs").write_text(
        r"""fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let object = directory.join("fixture.o");
    let compiler = std::env::var("CXX")?;
    let target = std::env::var("TARGET")?.replace('-', "_");
    let flags = std::env::var(format!("CXXFLAGS_{target}"))?;
    let compiled = std::process::Command::new(compiler)
        .args(flags.split_whitespace()).args(["-c", "src/fixture.cpp", "-o"])
        .arg(&object).status()?;
    if !compiled.success() { return Err("native fixture failed to compile".into()); }
    let archive = std::process::Command::new("ar").arg("crs")
        .arg(directory.join("libfixture.a")).arg(object).status()?;
    if !archive.success() { return Err("native fixture failed to archive".into()); }
    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rustc-link-lib=static=fixture");
    println!("cargo:rerun-if-changed=src/fixture.cpp");
    Ok(())
}
"""
    )
    (native / "src/fixture.cpp").write_text(
        'extern "C" int native_value(int input) noexcept {\n'
        "    if (input > 0) return input + 1;\n"
        "    return 0;\n}\n"
    )
    (native / "src/lib.rs").write_text(
        """unsafe extern "C" { fn native_value(input: i32) -> i32; }
pub fn value(input: i32) -> i32 { unsafe { native_value(input) } }
#[cfg(test)] mod tests {
    #[test] fn native_coverage() { assert_eq!(super::value(1), 2); assert_eq!(super::value(0), 0); }
}
"""
    )
    (tmp_path / "crates/pure/src/lib.rs").write_text(
        """pub fn value() -> u32 { 42 }
#[cfg(test)] mod tests {
    #[test] fn rust_coverage() { assert_eq!(super::value(), 42); assert!(cfg!(feature = "extra")); }
}
"""
    )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], check=True, capture_output=True, timeout=30
    )
    return make_context(Options())


def test_native_and_rust_reports_preserve_other_producers_and_gate_each_crate(
    coverage_workspace: Context,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    context = coverage_workspace
    compiler = context.tools.rustc.information()
    aliased = context.root / "compiler sources with 'quotes'"
    aliased.symlink_to(compiler.sysroot, target_is_directory=True)
    monkeypatch.setattr(
        context.tools.rustc, "information", lambda: replace(compiler, sysroot=aliased)
    )
    output = context.root / "coverage"
    output.mkdir()
    (output / "python.xml").write_text("other producer")
    Coverage.run(context)
    assert (output / "python.xml").read_text() == "other producer"
    assert (output / "html/index.html").is_file()
    report = (output / "lcov.info").read_text()
    assert "src/fixture.cpp" in report
    assert "crates/native/src/lib.rs" in report
    assert "crates/pure/src/lib.rs" in report
    assert "build.rs" in report
    assert "LLVM_PROFDATA_VERSION=" in (output / "toolchain.txt").read_text()
    assert {path.name for path in (output / "crates").iterdir()} == {"native.json", "pure.json"}
    summaries = json.loads((output / "crates/native.json").read_text())
    assert summaries["data"][0]["totals"]["lines"]["percent"] >= 90
    # Adding native measurements must preserve the existing Rust-only numeric
    # criterion. Uncovered C++ is still present in every diagnostic report.
    native = context.root / "crates/native/src/fixture.cpp"
    native.write_text(
        native.read_text().replace(
            "    if (input > 0)",
            "    if (input > 100) {\n"
            + "\n".join(f"        input += {value};" for value in range(1, 30))
            + "\n    }\n    if (input > 0)",
        )
    )
    Coverage.run(context)
    native_report = json.loads((output / "crates/native.json").read_text())
    assert native_report["data"][0]["totals"]["lines"]["percent"] < 90
    assert "src/fixture.cpp" in (output / "lcov.info").read_text()
    # Introduce real uncovered Rust code. A workspace-wide average must not hide
    # a package regression, and failed gates must still leave inspectable output.
    library = context.root / "crates/pure/src/lib.rs"
    library.write_text(
        library.read_text()
        + "\npub fn uncovered(value: u32) -> u32 {\n"
        + "\n".join(f"    let value = value.wrapping_add({value});" for value in range(12))
        + "\n    value\n}\n"
    )
    with pytest.raises(ToolingError, match=r"Coverage failed.*diagnostic reports retained"):
        Coverage.run(context)
    assert "src/fixture.cpp" in (output / "lcov.info").read_text()
    assert (output / "html/index.html").is_file()
    assert (output / "python.xml").read_text() == "other producer"
    failed = json.loads((output / "crates/pure.json").read_text())
    assert failed["data"][0]["totals"]["lines"]["percent"] < 90
    library.write_text("use std::io;\n" + library.read_text())
    with pytest.raises(ToolingError):
        Coverage.run(context)
    assert not (output / "lcov.info").exists()
    assert not (output / "html").exists()
    assert (output / "python.xml").read_text() == "other producer"


def test_coverage_refuses_symlinked_output_without_touching_the_destination(
    coverage_workspace: Context,
    tmp_path: Path,
) -> None:
    outside = tmp_path / "other-producer"
    outside.mkdir()
    (outside / "lcov.info").write_text("must survive")
    (coverage_workspace.root / "coverage").symlink_to(outside, target_is_directory=True)
    with pytest.raises(ToolingError, match="must not be a symlink"):
        CoverageReport.run(coverage_workspace)
    assert (outside / "lcov.info").read_text() == "must survive"


def test_rust_threshold_uses_counts_instead_of_rounded_percentages() -> None:
    def report(covered: int) -> str:
        return json.dumps(
            {
                "data": [
                    {
                        "files": [
                            {
                                "filename": "src/lib.rs",
                                "summary": {
                                    "lines": {
                                        "count": 100000,
                                        "covered": covered,
                                        "percent": 90.0,
                                    }
                                },
                            }
                        ]
                    }
                ]
            }
        )

    with pytest.raises(ToolingError, match="below 90%"):
        rust_line_gate(report(89999), "example")
    assert "90.00%" in rust_line_gate(report(90000), "example")
    with pytest.raises(ToolingError, match="Invalid coverage line counts"):
        rust_line_gate(report(100001), "example")
    with pytest.raises(ToolingError, match="no line records"):
        rust_line_gate('{"data": []}', "example")
