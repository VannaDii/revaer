"""Native Cargo and Trunk actually serve from the typed development adapters."""

import os
import secrets
import shutil
import socket
import subprocess
import time
from pathlib import Path
from urllib.error import URLError
from urllib.request import urlopen

import pytest
from revaer_tooling.development.processes import ProcessIdentity, ProcessManager
from revaer_tooling.external.rust import Cargo, DevelopmentArgs, Trunk
from revaer_tooling.external.serving import Listeners
from revaer_tooling.process import ProcessRunner, RunningProcess


def read_page(port: int, running: RunningProcess) -> str:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if running.poll() is not None:
            running.wait()
            pytest.fail("Native development server exited before serving")
        try:
            with urlopen(f"http://127.0.0.1:{port}/", timeout=1) as response:
                return str(response.read().decode())
        except (URLError, TimeoutError):
            time.sleep(0.05)
    pytest.fail("Native development server did not become ready")


def record(processes: ProcessManager, running: RunningProcess) -> ProcessIdentity:
    result = processes.identity(running.pid)
    assert result is not None
    return result


def test_cargo_development_builds_and_runs_real_rust(tmp_path: Path) -> None:
    (tmp_path / "src").mkdir()
    (tmp_path / "Cargo.toml").write_text(
        '[workspace]\n[package]\nname="revaer-app"\nversion="0.1.0"\nedition="2024"\n'
    )
    (tmp_path / "src/main.rs").write_text("""
use std::{env, fs, io::{Read, Write}, net::TcpListener};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("DATABASE_URL")? != "postgres://fixture.invalid/development"
        || env::var("RUST_LOG")? != "debug"
        || env::var("RV_DEV_SESSION")?.len() != 64 {
        return Err(std::io::Error::other("development environment mismatch").into());
    }
    let listener = TcpListener::bind("127.0.0.1:0")?;
    fs::write("api-port", listener.local_addr()?.port().to_string())?;
    for connection in listener.incoming() {
        let mut stream = connection?;
        let mut bytes = [0_u8; 4096];
        stream.read(&mut bytes)?;
        stream.write_all(concat!("HTTP/1.1 200 OK\\r\\nContent-Length: 8\\r\\n",
            "Connection: close\\r\\n\\r\\nRust API").as_bytes())?;
    }
    Ok(())
}
""")
    environment = {**os.environ, "CARGO_TARGET_DIR": str(tmp_path / "target")}
    subprocess.run(
        ["cargo", "generate-lockfile"],
        cwd=tmp_path,
        env=environment,
        check=True,
        capture_output=True,
        timeout=20,
    )
    messages: list[str] = []
    runner = ProcessRunner(messages.append)
    processes = ProcessManager(
        tmp_path, os.getpid(), os.getuid(), time.monotonic, lambda: secrets.token_hex(32)
    )
    log = tmp_path / "cargo.log"
    running = Cargo("cargo", runner, tmp_path, environment).development(
        DevelopmentArgs("postgres://fixture.invalid/development", "debug", log, processes.token())
    )
    identity = record(processes, running)
    try:
        deadline = time.monotonic() + 30
        while not (tmp_path / "api-port").is_file() and time.monotonic() < deadline:
            if running.poll() is not None:
                running.wait()
                pytest.fail("Cargo did not start the API")
            time.sleep(0.05)
        port = int((tmp_path / "api-port").read_text())
        assert read_page(port, running) == "Rust API"
        listeners = Listeners("lsof", runner, tmp_path, environment)
        assert processes.require_listener(listeners.owners(port), identity)
        assert "Running" in log.read_text()
        assert any("Running" in message for message in messages)
    finally:
        running.stop()
        processes.stop_workers((identity,))


def test_trunk_development_serves_real_html_and_stops(tmp_path: Path) -> None:
    source = Path(__file__).resolve().parents[2]
    root = tmp_path / "crates/revaer-ui"
    root.mkdir(parents=True)
    with socket.socket() as reservation:
        reservation.bind(("127.0.0.1", 0))
        port = reservation.getsockname()[1]
    (root / "Trunk.toml").write_text(f'[serve]\nport={port}\naddresses=["127.0.0.1"]\n')
    (root / "index.html").write_text("<!doctype html><html><body>Trunk fixture</body></html>")
    shutil.copy(source / "rust-toolchain.toml", tmp_path / "rust-toolchain.toml")
    runner = ProcessRunner(lambda message: None)
    processes = ProcessManager(
        tmp_path, os.getpid(), os.getuid(), time.monotonic, lambda: secrets.token_hex(32)
    )
    environment = {**os.environ}
    log = tmp_path / "trunk.log"
    running = Trunk("trunk", runner, tmp_path, environment).development(
        DevelopmentArgs("postgres://fixture.invalid/development", "info", log, processes.token())
    )
    identity = record(processes, running)
    try:
        assert "Trunk fixture" in read_page(port, running)
        assert (root / "dist-serve/index.html").is_file()
        listeners = Listeners("lsof", runner, tmp_path, environment)
        assert processes.require_listener(listeners.owners(port), identity)
    finally:
        running.stop()
        processes.stop_workers((identity,))
    assert "error" not in log.read_text().lower()
