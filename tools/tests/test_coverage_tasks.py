"""Exercise real Rust/C++ instrumentation, report retention, and per-crate gates."""

import json
import os
import shutil
import subprocess
import tomllib
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.tasks import e2e
from revaer_tooling.tasks.coverage import Coverage, CoverageReport, rust_lcov, rust_line_gate


@pytest.fixture
def coverage_workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    source = Path(__file__).resolve().parents[2]
    channel = tomllib.loads((source / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
    monkeypatch.setenv("RUSTUP_TOOLCHAIN", channel)
    monkeypatch.setenv("CARGO_TERM_COLOR", "always")
    monkeypatch.setenv("REVAER_TEST_DATABASE_URL", "postgres://fixture.invalid/unused")
    for name in (
        "CARGO_TARGET_DIR",
        "CARGO_LLVM_COV_TARGET_DIR",
        "RUSTC_COMMAND",
        "RUSTC",
        "REVAER_NATIVE_RECOVERY_ROOT",
    ):
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
    subprocess.run(["git", "init", "--quiet", str(tmp_path)], check=True, timeout=30)
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
        "#define POSITIVE(value) ((value) > 0)\n"
        "#define INCREMENT(value) ((value) + 1)\n"
        'extern "C" int native_value(int input) noexcept {\n'
        "    if (input > 0) return POSITIVE(input) ? INCREMENT(input) : 0;\n"
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
    static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    #[test] fn isolated_first() {
        assert_eq!(CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);
    }
    #[test] fn isolated_second() {
        assert_eq!(CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst), 0);
    }
    #[test] fn rust_coverage() { assert_eq!(super::value(), 42); assert!(cfg!(feature = "extra")); }
}
"""
    )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"], check=True, capture_output=True, timeout=30
    )
    subprocess.run(["git", "add", "."], check=True, capture_output=True, timeout=30)
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
    assert "src/fixture.cpp" not in report
    assert "src/fixture.cpp" in (output / "workspace-lcov.info").read_text()
    assert "crates/native/src/lib.rs" in report
    assert "fixture.cpp" in (output / "llvm-cov.txt").read_text()
    assert "crates/pure/src/lib.rs" in report
    assert "build.rs" in report
    native_text = (output / "llvm-cov.txt").read_text()
    assert "src/fixture.cpp" in native_text
    assert "|    if (input > 0) return POSITIVE(input) ? INCREMENT(input) : 0;" in native_text
    assert "  |  |" not in native_text
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
    assert "crates/native/src/lib.rs" in (output / "lcov.info").read_text()
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
    assert "crates/native/src/lib.rs" in (output / "lcov.info").read_text()
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


def test_rust_threshold_uses_counts_instead_of_rounded_percentages(tmp_path: Path) -> None:
    def report(covered: int, filename: str = "crates/example/src/lib.rs") -> str:
        return json.dumps(
            {
                "data": [
                    {
                        "files": [
                            {
                                "filename": filename,
                                "summary": {
                                    "lines": {"count": 100000, "covered": covered, "percent": 90.0}
                                },
                            }
                        ]
                    }
                ]
            }
        )

    tracked = ("crates/example/src/lib.rs", "crates/revaer-test-support/src/lib.rs")
    with pytest.raises(ToolingError, match="below 90%"):
        rust_line_gate(report(89999), "example", tmp_path, tracked)
    assert "90.00%" in rust_line_gate(report(90000), "example", tmp_path, tracked)
    utility = report(0, tracked[1])
    assert "no production coverage target" in rust_line_gate(
        utility, "revaer-test-support", tmp_path, tracked
    )
    with pytest.raises(ToolingError, match="Invalid coverage line counts"):
        rust_line_gate(report(100001), "example", tmp_path, tracked)
    for package in ("example", "revaer-test-support"):
        with pytest.raises(ToolingError, match="no line records"):
            rust_line_gate('{"data": []}', package, tmp_path, tracked)


@pytest.mark.parametrize("production_hits", (89, 90))
def test_rust_gate_neither_counts_uncovered_scaffolding_nor_credits_covered_tests(
    tmp_path: Path, production_hits: int
) -> None:
    production = "crates/example/src/lib.rs"
    scaffolding = (
        "crates/example/src/tests.rs",
        "crates/example/examples/demo.rs",
        "crates/example/benches/benchmark.rs",
        "crates/revaer-ui/tools/asset_sync/src/lib.rs",
        "vendor/dependency/src/lib.rs",
        "target/generated.rs",
    )
    for helper_hits in (0, 100000):
        files = [
            {
                "filename": str(tmp_path / production),
                "summary": {"lines": {"count": 100, "covered": production_hits}},
            },
            *(
                {"filename": name, "summary": {"lines": {"count": 100000, "covered": helper_hits}}}
                for name in scaffolding
            ),
        ]
        document = json.dumps({"data": [{"files": files}]})
        tracked = (production, *scaffolding[:-1])
        if production_hits == 89:
            with pytest.raises(ToolingError, match="below 90%"):
                rust_line_gate(document, "example", tmp_path, tracked)
        else:
            assert "90/100" in rust_line_gate(document, "example", tmp_path, tracked)


def test_rust_sensor_input_retains_uncovered_lines_and_routes_native_separately(
    tmp_path: Path,
) -> None:
    rust = "SF:src/lib.rs\nDA:1,0\nDA:2,7\nend_of_record\n"
    native = "SF:src/lib.cpp\nDA:1,0\nBRDA:1,0,0,-\nend_of_record\n"
    dependency = "SF:/cargo/registry/dependency.rs\nDA:1,3\nend_of_record\n"
    assert rust_lcov(native + rust + dependency, tmp_path, ("src/lib.rs", "src/lib.cpp")) == rust
    with pytest.raises(ToolingError, match="no authored Rust"):
        rust_lcov(native + dependency, tmp_path, ("src/lib.rs",))
    with pytest.raises(ToolingError, match="exactly one source"):
        rust_lcov("DA:1,1\nend_of_record", tmp_path, ("src/lib.rs",))


@pytest.mark.parametrize("absolute", (False, True))
def test_rust_sensor_retains_tracked_source_path_aliases(tmp_path: Path, absolute: bool) -> None:
    name = "crates/example/src/../tests/unit.rs"
    source = str(tmp_path / name) if absolute else name
    measured = "DA:1,0\nDA:2,7\nBRDA:2,0,0,7\nend_of_record\n"
    document = "SF:" + source + "\n" + measured
    expected = "crates/example/tests/unit.rs"
    assert rust_lcov(document, tmp_path, (expected,)) == document
    with pytest.raises(ToolingError, match="no authored Rust"):
        rust_lcov(document, tmp_path, ())


def test_rust_sensor_merges_separately_compiled_source_measurements(tmp_path: Path) -> None:
    name = "crates/example/build_support.rs"
    alias = "crates/example/tests/../build_support.rs"
    direct = "SF:" + name + "\nDA:1,0\nDA:2,3\nend_of_record\n"
    included = "SF:" + alias + "\nDA:1,7\nDA:2,2\nend_of_record\n"
    merged = rust_lcov(direct + included, tmp_path, (name,))
    assert merged.count("SF:") == 1
    assert "DA:1,7\nDA:2,5\n" in merged
    assert "LF:2\nLH:2\n" in merged


@pytest.mark.parametrize("managed", (False, True))
def test_production_worker_coverage_preserves_workspace_native_profiles(
    coverage_workspace: Context,
    monkeypatch: pytest.MonkeyPatch,
    managed: bool,
) -> None:
    root = coverage_workspace.root
    mounted = root / "mounted"
    mounted.mkdir()
    context: Context = Context(
        root=coverage_workspace.root,
        settings=coverage_workspace.settings,
        options=coverage_workspace.options,
        tools=coverage_workspace.tools,
        fs=coverage_workspace.fs,
        emit=coverage_workspace.emit,
        host=replace(coverage_workspace.host, system="linux"),
        invoked_at=coverage_workspace.invoked_at,
        command_names=coverage_workspace.command_names,
    )
    if managed:

        @contextmanager
        def fixture(selected: Context, paths: e2e.RunPaths) -> Iterator[tuple[e2e.RunPaths, Path]]:
            yield replace(paths, filesystem=mounted), mounted / "catalog.json"

        monkeypatch.setattr(e2e, "media_fixture", fixture)
    else:
        context = Context(
            root=context.root,
            settings=replace(
                context.settings,
                coverage=replace(context.settings.coverage, native_recovery_root=mounted),
            ),
            options=context.options,
            tools=context.tools,
            fs=context.fs,
            emit=context.emit,
            host=context.host,
            invoked_at=context.invoked_at,
            command_names=context.command_names,
        )
    app = root / "crates/revaer-app"
    (app / "src").mkdir(parents=True)
    (app / "Cargo.toml").write_text(
        '[package]\nname="revaer-app"\nversion="0.1.0"\nedition="2024"\n[features]\nextra=[]\n'
    )
    (app / "src/lib.rs").write_text("""
pub fn production_value() -> u32 { 4242 }
pub fn recovery_value() -> u32 { 4243 }
#[cfg(test)] mod media_job_runtime { mod tests {
    #[test] #[ignore = "production selection"]
    fn production_media_job_runtime_executes_and_persists_verified_replacement() {
        assert!(cfg!(feature = "extra"));
        assert_eq!(crate::production_value(), 4242);
    }
} }
#[cfg(test)] mod bootstrap { mod service_recovery_tests {
    #[test] #[ignore = "recovery selection"]
    fn native_service_recovery_0() -> Result<(), Box<dyn std::error::Error>> {
        assert!(std::path::Path::new(&std::env::var("REVAER_NATIVE_RECOVERY_ROOT")?).is_dir());
        assert_eq!(crate::recovery_value(), 4243);
        Ok(())
    }
} }
""")
    library = app / "src/lib.rs"
    scenario_source = library.read_text()
    start = scenario_source.index('    #[test] #[ignore = "recovery selection"]')
    end = scenario_source.index("\n} }", start)
    scenario = scenario_source[start:end]
    library.write_text(
        scenario_source[:start]
        + "\n".join(
            scenario.replace("native_service_recovery_0", f"native_service_recovery_{index}")
            for index in range(9)
        )
        + scenario_source[end:]
    )
    subprocess.run(
        ["cargo", "generate-lockfile", "--offline"],
        check=True,
        capture_output=True,
        timeout=30,
    )
    subprocess.run(["git", "add", "crates/revaer-app"], check=True, capture_output=True, timeout=30)
    Coverage.run(context)
    lcov = (root / "coverage/lcov.info").read_text()
    original = (root / "coverage/workspace-lcov.info").read_text()
    assert "src/fixture.cpp" in original
    assert "src/fixture.cpp" not in lcov
    assert "crates/pure/src/lib.rs" in lcov
    assert "crates/revaer-app/src/lib.rs" in lcov
    report = json.loads((root / "coverage/crates/revaer-app.json").read_text())
    assert report["data"][0]["totals"]["lines"]["percent"] >= 90
