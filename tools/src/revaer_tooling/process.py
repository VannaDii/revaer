"""Shell-free subprocess execution with bounded cancellation and redacted output."""

import os
import re
import signal
import stat
import subprocess
import sys
import threading
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO, Protocol, TextIO

from .errors import CommandError, ToolingError


@dataclass(frozen=True)
class Invocation:
    argv: tuple[str, ...]
    cwd: Path
    env: Mapping[str, str]
    capture: bool = False
    input_text: str | None = None
    timeout: float | None = None
    accepted_codes: tuple[int, ...] = (0,)
    log_path: Path | None = None
    stream_log: bool = False
    capture_binary: bool = False


@dataclass(frozen=True)
class Completed:
    code: int
    stdout: str
    stderr: str = ""
    stdout_bytes: bytes | None = None


class RunningProcess(Protocol):
    @property
    def pid(self) -> int: ...
    def poll(self) -> int | None: ...
    def wait(self) -> Completed: ...
    def stop(self) -> None: ...


class Runner(Protocol):
    def run(self, invocation: Invocation) -> Completed: ...
    def start(self, invocation: Invocation) -> RunningProcess: ...


class Redactor:
    """Redact explicit credentials and userinfo in database URLs."""

    def __init__(self, environment: Mapping[str, str]) -> None:
        self._secrets = tuple(
            sorted(
                {
                    value
                    for key, value in environment.items()
                    if value and re.search(r"PASSWORD|TOKEN|SECRET|PRIVATE|API_KEY", key)
                },
                key=len,
                reverse=True,
            )
        )

    def __call__(self, value: str) -> str:
        for secret in self._secrets:
            value = value.replace(secret, "[redacted]")
        return re.sub(r"(\w+://)[^\s/@]+:[^\s/@]*@", r"\1[redacted]@", value)


def _open_log(path: Path) -> TextIO:
    """Keep process evidence private, including when replacing an older log.

    Refuse links and special files before truncation. A tool may print a newly
    generated credential that was not in its initial environment for redaction.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_NOFOLLOW | os.O_NONBLOCK, 0o600)
    try:
        information = os.fstat(descriptor)
        if not stat.S_ISREG(information.st_mode) or information.st_nlink != 1:
            raise ToolingError(
                f"Process log must be a regular file without extra hard links: {path}"
            )
        os.fchmod(descriptor, 0o600)
        os.ftruncate(descriptor, 0)
        return os.fdopen(descriptor, "w", encoding="utf-8", newline="")
    except BaseException:
        os.close(descriptor)
        raise


class ChildProcess:
    def __init__(self, invocation: Invocation, emit: Callable[[str], None]) -> None:
        if not invocation.argv:
            raise ToolingError("Cannot execute an empty command")
        self._invocation = invocation
        self._emit = emit
        self._redact = Redactor(invocation.env)
        self._output: list[str] = []
        self._binary_output: list[bytes] = []
        self._diagnostics: list[str] = []
        self._closed = False
        self._io_errors: list[OSError | UnicodeError] = []
        self._log_lock = threading.Lock()
        self._log: TextIO | None = None
        if invocation.log_path:
            try:
                self._log = _open_log(invocation.log_path)
            except OSError as error:
                raise ToolingError(
                    f"Cannot open process log {invocation.log_path}: {error}"
                ) from error
        emit(self._redact(f"+ {' '.join(invocation.argv)}"))
        try:
            self._process = subprocess.Popen(
                invocation.argv,
                cwd=invocation.cwd,
                env=dict(invocation.env),
                stdin=subprocess.PIPE if invocation.input_text is not None else subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except OSError as error:
            if self._log:
                self._log.close()
            raise ToolingError(
                self._redact(f"Cannot start {invocation.argv[0]}: {error}")
            ) from error
        # Drain both pipes independently: warnings must not corrupt JSON/LCOV
        # captured from stdout, and a full stderr pipe must not deadlock a tool.
        self._workers = [
            threading.Thread(target=self._read, args=(self._process.stdout, False), daemon=True),
            threading.Thread(target=self._read, args=(self._process.stderr, True), daemon=True),
        ]
        if self._process.stdin is not None:
            # Writing can block too. Keep timeout/cancellation in the caller
            # even when a child stops consuming a large stdin payload.
            self._workers.append(threading.Thread(target=self._write, daemon=True))
        for worker in self._workers:
            worker.start()

    @property
    def pid(self) -> int:
        return self._process.pid

    def poll(self) -> int | None:
        return self._process.poll()

    def _write(self) -> None:
        stream = self._process.stdin
        if stream is None:
            return
        try:
            with stream:
                stream.write((self._invocation.input_text or "").encode("utf-8"))
        except (OSError, UnicodeError) as error:
            self._io_errors.append(error)
            if not isinstance(error, BrokenPipeError):
                self._signal_group(signal.SIGKILL)

    def _read(self, stream: BinaryIO | None, diagnostic: bool) -> None:
        if stream is None:
            return
        try:
            if self._invocation.capture_binary and not diagnostic:
                # Git blob evidence includes arbitrary bytes. Keep it out of
                # text decoding, redaction and logs; diagnostics still use the
                # normal independent, redacted stderr path.
                while chunk := stream.read(65_536):
                    self._binary_output.append(chunk)
                return
            for raw_line in stream:
                # Binary pipes preserve CR, CRLF and NUL exactly. Text-mode
                # Popen normalizes newlines before callers can validate native
                # evidence. UTF-8 decoding is strict and LF cannot split a
                # multibyte character; malformed output follows the I/O failure
                # path and terminates only this owned process group.
                line = raw_line.decode("utf-8")
                if self._invocation.capture or self._invocation.capture_binary:
                    (self._diagnostics if diagnostic else self._output).append(line)
                if self._log:
                    with self._log_lock:
                        self._log.write(self._redact(line))
                        self._log.flush()
                if (not self._log or self._invocation.stream_log) and (
                    diagnostic or not self._invocation.capture
                ):
                    self._emit(self._redact(line.rstrip("\n")))
        except (OSError, UnicodeError) as error:
            self._io_errors.append(error)

            # A failed reader can leave the child blocked on a full pipe forever.
            # End only this owned process group and report the I/O error to wait().
            self._signal_group(signal.SIGKILL)

    def _signal_group(self, number: int) -> None:
        try:
            os.killpg(self.pid, number)
        except ProcessLookupError:
            self._process.wait()

    def stop(self) -> None:
        if self._closed:
            return
        self._signal_group(signal.SIGTERM)
        try:
            self._process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self._signal_group(signal.SIGKILL)
            self._process.wait()
        self._finish()
        self._check_io(ignore_broken_pipe=True)

    def _finish(self) -> None:
        if self._closed:
            return
        for worker in self._workers:
            worker.join(timeout=5)
        if any(worker.is_alive() for worker in self._workers):
            self._signal_group(signal.SIGKILL)
            for worker in self._workers:
                worker.join(timeout=5)
        if any(worker.is_alive() for worker in self._workers):
            raise ToolingError("Process pipes did not close after group termination")
        if self._process.stdout:
            self._process.stdout.close()
        if self._process.stderr:
            self._process.stderr.close()
        if self._log:
            self._log.close()
        self._closed = True
        # Cancellation deliberately closes stdin; a resulting BrokenPipeError
        # must keep the timeout/interrupt result. Other I/O failures remain errors.

    def _check_io(self, ignore_broken_pipe: bool = False) -> None:
        errors = [
            error
            for error in self._io_errors
            if not (ignore_broken_pipe and isinstance(error, BrokenPipeError))
        ]
        if errors:
            error = errors[0]
            raise ToolingError(
                self._redact(f"Cannot transfer process input/output: {error}")
            ) from error

    def wait(self) -> Completed:
        try:
            code = self._process.wait(timeout=self._invocation.timeout)
        except subprocess.TimeoutExpired as error:
            self.stop()
            raise CommandError(f"{self._invocation.argv[0]} exceeded its timeout", 124) from error
        except BaseException:
            self.stop()
            raise
        self._finish()
        self._check_io(ignore_broken_pipe=True)
        captured = "".join(self._output)
        if code not in self._invocation.accepted_codes:
            if captured:
                self._emit(self._redact(captured.rstrip()))
            raise CommandError(
                f"{Path(self._invocation.argv[0]).name} exited with status {code}", code
            )
        self._check_io()
        return Completed(
            code,
            captured,
            "".join(self._diagnostics),
            b"".join(self._binary_output) if self._invocation.capture_binary else None,
        )


class ProcessRunner:
    def __init__(self, emit: Callable[[str], None]) -> None:
        self._emit = emit

    def start(self, invocation: Invocation) -> RunningProcess:
        return ChildProcess(invocation, self._emit)

    def run(self, invocation: Invocation) -> Completed:
        return self.start(invocation).wait()


def stderr_message(message: str) -> None:
    print(message, file=sys.stderr, flush=True)
