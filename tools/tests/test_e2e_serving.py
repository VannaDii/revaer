"""Use Cargo's real artifacts and a real listener to verify service ownership."""

import json
import os
import socket
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.serving import (
    E2E_SERVING_ENTRY,
    ApplicationArgs,
    ServingExecutable,
    ServingKind,
    select_executable,
)
from revaer_tooling.tasks.e2e import wait_for_service


@pytest.fixture
def serving_workspace(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Context:
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\nresolver="3"\nmembers=["crates/revaer-app"]\n'
    )
    app = tmp_path / "crates/revaer-app"
    (app / "src").mkdir(parents=True)
    (app / "Cargo.toml").write_text(
        '[package]\nname="revaer-app"\nversion="0.1.0"\nedition="2024"\n'
    )
    (app / "src/main.rs").write_text('fn main() { println!("production executable"); }\n')
    (app / "src/lib.rs").write_text("""#[cfg(test)] mod bootstrap { mod runtime_tests {
    #[test] fn e2e_serving_entry() -> Result<(), Box<dyn std::error::Error>> {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind(std::env::var("RV_TEST_ADDRESS")?)?;
        for connection in listener.incoming() {
            let mut stream = connection?;
            let mut request = [0; 1024];
            if stream.read(&mut request)? == 0 { continue; }
            stream.write_all(b"HTTP/1.1 200 OK\\r\\nContent-Length: 2\\r\\n")?;
            stream.write_all(b"Connection: close\\r\\n\\r\\nok")?;
        }
        Ok(())
    }
}}
""")
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        port = listener.getsockname()[1]
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("CARGO_TARGET_DIR", str(tmp_path / "target with spaces"))
    monkeypatch.setenv("RV_TEST_ADDRESS", f"127.0.0.1:{port}")
    monkeypatch.setenv("E2E_API_BASE_URL", f"http://127.0.0.1:{port}")
    context = make_context(Options())
    # Generate the fixture lock with Cargo. The production adapter requires it.
    from revaer_tooling.process import Invocation

    context.tools.cargo.runner.run(
        Invocation(
            (str(context.tools.cargo.locate()), "generate-lockfile", "--offline"),
            tmp_path,
            context.tools.cargo.environment,
        )
    )
    return context


def test_cargo_selects_exact_target_and_listener_belongs_to_owned_process(
    serving_workspace: Context,
) -> None:
    context = serving_workspace
    binary = context.tools.cargo.serving_executable(ServingKind.BINARY)
    assert binary.path == context.root / "target with spaces/debug/revaer-app"
    production_log = context.root / "production.log"
    production = context.tools.application.start(
        ApplicationArgs(
            binary,
            "postgres://fixture/unused",
            context.root,
            production_log,
        )
    )
    assert production.wait().code == 0
    assert production_log.read_text() == "production executable\n"
    executable = context.tools.cargo.serving_executable(ServingKind.LIBRARY_TEST)
    assert executable.path != binary.path
    assert executable.arguments == ("--exact", E2E_SERVING_ENTRY, "--nocapture")
    port = int(context.settings.e2e.api_url.rsplit(":", 1)[1])
    context.tools.listeners.require_free(port)
    log = context.root / "server.log"
    child = context.tools.application.start(
        ApplicationArgs(
            executable,
            "postgres://fixture/unused",
            context.root,
            log,
        )
    )
    try:
        wait_for_service(context, child, context.settings.e2e.api_url, log)
        context.tools.listeners.require_owner(port, child.pid)
        with pytest.raises(ToolingError, match=r"Port .* in use"):
            context.tools.listeners.require_free(port)
        with pytest.raises(ToolingError, match="Cannot prove"):
            context.tools.listeners.require_owner(port, os.getpid())
        assert child.poll() is None
    finally:
        child.stop()
    assert child.poll() is not None
    context.tools.listeners.require_free(port)
    with pytest.raises(ToolingError, match="exited"):
        wait_for_service(context, child, context.settings.e2e.api_url, log)
    with pytest.raises(ToolingError, match="no longer exists"):
        context.tools.application.verify(replace(executable, path=context.root / "missing"))
    with pytest.raises(ToolingError, match="no E2E serving test entry"):
        context.tools.application.verify(ServingExecutable(binary.path, ServingKind.LIBRARY_TEST))


def test_metadata_rejects_wrong_sources_profiles_and_incomplete_builds() -> None:
    root = Path("/fixture/checkout")
    artifact = {
        "reason": "compiler-artifact",
        "profile": {"test": True},
        "target": {
            "name": "revaer_app",
            "kind": ["lib"],
            "src_path": str(root / "crates/revaer-app/src/lib.rs"),
        },
        "executable": "/fixture/target with spaces/server",
    }
    finished = {"reason": "build-finished", "success": True}

    def output(*messages: object) -> str:
        return "\n".join(json.dumps(message) for message in messages)

    assert select_executable(
        output(artifact, finished), root, ServingKind.LIBRARY_TEST
    ).path.is_absolute()
    bad_artifacts = (
        {**artifact, "profile": {"test": False}},
        {**artifact, "profile": None},
        {**artifact, "executable": "relative/path"},
        {**artifact, "executable": None},
        {**artifact, "target": {"name": "unrelated", "kind": ["lib"], "src_path": "/other/lib.rs"}},
    )
    cases = [output(value, finished) for value in bad_artifacts]
    cases.extend(
        (
            output(artifact),
            output(finished),
            output(artifact, artifact, finished),
            output(artifact, finished, finished),
            output(artifact, {**finished, "success": False}),
            "not-json",
            "[]",
        )
    )
    for value in cases:
        with pytest.raises(ToolingError):
            select_executable(value, root, ServingKind.LIBRARY_TEST)
