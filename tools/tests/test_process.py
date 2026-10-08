"""Real process tests exercise cancellation, failures and credential redaction."""

import json
import os
import secrets
import stat
import sys
import time
from pathlib import Path

import pytest
from revaer_tooling.errors import CommandError
from revaer_tooling.process import Invocation, ProcessRunner, Redactor


def test_arguments_are_literal_and_captured(tmp_path: Path) -> None:
    messages: list[str] = []
    value = 'spaces; $(touch should-not-exist) "quoted"'
    result = ProcessRunner(messages.append).run(
        Invocation(
            (sys.executable, "-c", "import sys; print(sys.argv[1])", value),
            tmp_path,
            dict(os.environ),
            capture=True,
        )
    )
    assert result.stdout == value + "\n"
    assert not (tmp_path / "should-not-exist").exists()


def test_stdin_and_failure_diagnostics_redact_secrets(tmp_path: Path) -> None:
    messages: list[str] = []
    token = "private-test-value"
    environment = {**os.environ, "GITHUB_TOKEN": token}
    with pytest.raises(CommandError) as failure:
        ProcessRunner(messages.append).run(
            Invocation(
                (sys.executable, "-c", "import sys; print(sys.stdin.read()); sys.exit(7)"),
                tmp_path,
                environment,
                capture=True,
                input_text=token,
            )
        )
    assert failure.value.exit_code == 7
    assert token not in "\n".join(messages)
    assert "[redacted]" in "\n".join(messages)


def test_timeout_stops_process(tmp_path: Path) -> None:
    with pytest.raises(CommandError, match="timeout") as failure:
        ProcessRunner(lambda message: None).run(
            Invocation(
                (sys.executable, "-c", "import time; time.sleep(30)"),
                tmp_path,
                dict(os.environ),
                timeout=0.1,
            )
        )
    assert failure.value.exit_code == 124


def test_expected_nonzero_result_is_explicit(tmp_path: Path) -> None:
    result = ProcessRunner(lambda message: None).run(
        Invocation(
            (sys.executable, "-c", "raise SystemExit(3)"),
            tmp_path,
            dict(os.environ),
            accepted_codes=(0, 3),
        )
    )
    assert result.code == 3


def test_diagnostics_do_not_corrupt_structured_stdout(tmp_path: Path) -> None:
    messages: list[str] = []
    result = ProcessRunner(messages.append).run(
        Invocation(
            (sys.executable, "-c", 'import sys; print("diagnostic", file=sys.stderr); print(42)'),
            tmp_path,
            dict(os.environ),
            capture=True,
        )
    )
    assert json.loads(result.stdout) == 42
    assert result.stderr == "diagnostic\n"
    assert "diagnostic" in messages


def test_native_output_preserves_exact_bytes_and_line_endings(tmp_path: Path) -> None:
    payload = "雪\r\nsecond\rthird\x00\nlast".encode()
    log = tmp_path / "native.log"
    result = ProcessRunner(lambda message: None).run(
        Invocation(
            (
                sys.executable,
                "-c",
                "import os, sys; data = bytes.fromhex(sys.argv[1]); "
                "os.write(1, data); os.write(2, data)",
                payload.hex(),
            ),
            tmp_path,
            dict(os.environ),
            capture=True,
            log_path=log,
        )
    )
    assert result.stdout.encode() == payload
    assert result.stderr.encode() == payload
    # The streams may interleave in the combined log. Their byte counts must
    # survive without the universal-newline transformation used by text pipes.
    logged = log.read_bytes()
    assert len(logged) == len(payload) * 2
    assert logged.count(b"\r\n") == 2
    assert logged.count(b"\r") == 4
    assert logged.count(b"\x00") == 2


@pytest.mark.parametrize("status", (0, 7))
def test_binary_capture_preserves_blobs_without_logging_them(tmp_path: Path, status: int) -> None:
    payload = bytes(range(256)) * 1024 + b"\0\r\nfinal"
    source = tmp_path / "blob"
    source.write_bytes(payload)
    messages: list[str] = []
    token = secrets.token_hex(24)
    log = tmp_path / "command.log"
    result = ProcessRunner(messages.append).run(
        Invocation(
            (
                sys.executable,
                "-c",
                "import os, pathlib, sys; "
                "sys.stdout.buffer.write(pathlib.Path(sys.argv[1]).read_bytes()); "
                "print(os.environ['GITHUB_TOKEN'], file=sys.stderr); sys.exit(int(sys.argv[2]))",
                str(source),
                str(status),
            ),
            tmp_path,
            {**os.environ, "GITHUB_TOKEN": token},
            capture_binary=True,
            accepted_codes=(0, 7),
            log_path=log,
        )
    )
    assert result.code == status
    assert result.stdout_bytes == payload
    assert result.stdout == ""
    assert result.stderr == token + "\n"
    assert log.read_text() == "[redacted]\n"
    assert token not in "\n".join(messages)


def test_binary_failure_reports_status_without_decoding_its_output(tmp_path: Path) -> None:
    with pytest.raises(CommandError, match="status 7"):
        ProcessRunner(lambda message: None).run(
            Invocation(
                (
                    sys.executable,
                    "-c",
                    "import os; os.write(1, bytes(range(256))); raise SystemExit(7)",
                ),
                tmp_path,
                dict(os.environ),
                capture_binary=True,
            )
        )


def test_development_logs_stream_and_retain_the_same_redacted_output(tmp_path: Path) -> None:
    messages: list[str] = []
    log = tmp_path / "development.log"
    token = secrets.token_hex(24)
    ProcessRunner(messages.append).run(
        Invocation(
            (sys.executable, "-c", "import os; print(os.environ['API_KEY'])"),
            tmp_path,
            {**os.environ, "API_KEY": token},
            log_path=log,
            stream_log=True,
        )
    )
    assert log.read_text() == "[redacted]\n"
    assert "[redacted]" in messages
    assert token not in "\n".join(messages)


def test_large_stdin_does_not_prevent_timeout(tmp_path: Path) -> None:
    started = time.monotonic()
    with pytest.raises(CommandError, match="timeout"):
        ProcessRunner(lambda message: None).run(
            Invocation(
                (sys.executable, "-c", "import time; time.sleep(30)"),
                tmp_path,
                dict(os.environ),
                input_text="payload" * 100_000,
                timeout=0.1,
            )
        )
    assert time.monotonic() - started < 5


def test_stop_waits_for_owned_child_and_closes_redacted_log(tmp_path: Path) -> None:
    log = tmp_path / "process.log"
    log.write_text("old run\n")
    log.chmod(0o644)
    token = "child-log-test-secret"
    ready = tmp_path / "ready"
    child = ProcessRunner(lambda message: None).start(
        Invocation(
            (
                sys.executable,
                "-c",
                "import os, pathlib, time; print(os.environ['API_TOKEN'], flush=True); "
                "pathlib.Path('ready').touch(); time.sleep(30)",
            ),
            tmp_path,
            {**os.environ, "API_TOKEN": token},
            log_path=log,
        )
    )
    try:
        deadline = time.monotonic() + 5
        while not ready.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        assert ready.exists()
    finally:
        child.stop()
    assert child.poll() is not None
    assert token not in log.read_text()
    assert "[redacted]" in log.read_text()
    assert "old run" not in log.read_text()
    assert stat.S_IMODE(log.stat().st_mode) == 0o600


@pytest.mark.parametrize("link_type", ["symbolic", "hard"])
def test_process_log_cannot_overwrite_a_link_target(tmp_path: Path, link_type: str) -> None:
    from revaer_tooling.errors import ToolingError

    target = tmp_path / "existing"
    target.write_text("retain this evidence\n")
    log = tmp_path / "process.log"
    if link_type == "symbolic":
        log.symlink_to(target)
    else:
        log.hardlink_to(target)
    with pytest.raises(ToolingError, match="log"):
        ProcessRunner(lambda message: None).start(
            Invocation((sys.executable, "-c", "print('new run')"), tmp_path, {}, log_path=log)
        )
    assert target.read_text() == "retain this evidence\n"


def test_redactor_handles_urls_and_overlapping_values() -> None:
    redact = Redactor({"API_KEY": "long-secret", "API_TOKEN": "secret"})
    assert redact("long-secret secret") == "[redacted] [redacted]"
    assert redact("postgres://user:pass@localhost/db") == "postgres://[redacted]@localhost/db"


def test_invalid_command_fails_cleanly_and_closes_its_log(tmp_path: Path) -> None:
    from revaer_tooling.errors import ToolingError

    runner = ProcessRunner(lambda message: None)
    for arguments in ((), (str(tmp_path / "missing"),)):
        with pytest.raises(ToolingError):
            runner.start(Invocation(arguments, tmp_path, {}, log_path=tmp_path / "run.log"))
    # Reusing the path after a failed spawn must not leave an open writer behind.
    result = runner.run(
        Invocation(
            (sys.executable, "-c", "print('ready')"),
            tmp_path,
            dict(os.environ),
            log_path=tmp_path / "run.log",
        )
    )
    assert result.code == 0
    assert (tmp_path / "run.log").read_text() == "ready\n"


def test_decode_failure_terminates_the_child_and_reports_io_error(tmp_path: Path) -> None:
    from revaer_tooling.errors import ToolingError

    child = ProcessRunner(lambda message: None).start(
        Invocation(
            (sys.executable, "-c", "import os, time; os.write(1, b'\\xff\\n'); time.sleep(30)"),
            tmp_path,
            dict(os.environ),
            timeout=5,
        )
    )
    with pytest.raises(ToolingError, match="input/output"):
        child.wait()
    assert child.poll() is not None


def test_unconsumed_input_is_not_reported_as_success(tmp_path: Path) -> None:
    from revaer_tooling.errors import ToolingError

    with pytest.raises(ToolingError, match="input/output"):
        ProcessRunner(lambda message: None).run(
            Invocation(
                (sys.executable, "-c", "import os; os.close(0)"),
                tmp_path,
                dict(os.environ),
                input_text="payload" * 100_000,
            )
        )


def test_stop_escalates_when_child_ignores_termination(tmp_path: Path) -> None:
    ready = tmp_path / "ready"
    child = ProcessRunner(lambda message: None).start(
        Invocation(
            (
                sys.executable,
                "-c",
                "import signal, pathlib, time; "
                "signal.signal(signal.SIGTERM, signal.SIG_IGN); pathlib.Path('ready').touch(); "
                "time.sleep(30)",
            ),
            tmp_path,
            dict(os.environ),
        )
    )
    try:
        deadline = time.monotonic() + 5
        while not ready.exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        assert ready.exists()
    finally:
        child.stop()
    assert child.poll() == -9
    child.stop()  # Stopping an already reaped child is safe.


def test_inherited_pipes_do_not_leave_orphaned_processes(tmp_path: Path) -> None:
    child = ProcessRunner(lambda message: None).start(
        Invocation(
            (
                sys.executable,
                "-c",
                "import subprocess, sys; "
                "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)']); "
                "print('parent finished')",
            ),
            tmp_path,
            dict(os.environ),
            capture=True,
        )
    )
    started = time.monotonic()
    assert child.wait().stdout == "parent finished\n"
    assert time.monotonic() - started < 15


def test_interruption_reaps_the_owned_process(tmp_path: Path) -> None:
    import signal

    def interrupt(number: int, frame: object) -> None:
        raise KeyboardInterrupt

    child = ProcessRunner(lambda message: None).start(
        Invocation(
            (sys.executable, "-c", "import time; time.sleep(30)"), tmp_path, dict(os.environ)
        )
    )
    previous = signal.signal(signal.SIGALRM, interrupt)
    try:
        signal.setitimer(signal.ITIMER_REAL, 0.1)
        with pytest.raises(KeyboardInterrupt):
            child.wait()
        assert child.poll() is not None
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)
        child.stop()
