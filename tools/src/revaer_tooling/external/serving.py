"""Owned application processes and listener checks for development and E2E."""

import json
import re
import socket
from collections.abc import Mapping
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path

from ..errors import ToolingError
from ..process import Invocation, Runner, RunningProcess
from .base import ExternalTool

E2E_SERVING_ENTRY = "bootstrap::runtime_tests::e2e_serving_entry"


class ServingKind(StrEnum):
    BINARY = "binary"
    LIBRARY_TEST = "lib-test"


@dataclass(frozen=True)
class ServingExecutable:
    path: Path
    kind: ServingKind

    @property
    def arguments(self) -> tuple[str, ...]:
        return (
            ("--exact", E2E_SERVING_ENTRY, "--nocapture")
            if self.kind == ServingKind.LIBRARY_TEST
            else ()
        )


def select_executable(output: str, root: Path, kind: ServingKind) -> ServingExecutable:
    """Accept only Cargo's completed artifact from this checkout and target.

    This preserves media's instrumented library-test server and respects Cargo's
    configured target directory, instead of guessing a debug binary pathname.
    """
    test = kind == ServingKind.LIBRARY_TEST
    expected_name = "revaer_app" if test else "revaer-app"
    expected_kind = ["lib"] if test else ["bin"]
    expected_source = root / "crates/revaer-app/src" / ("lib.rs" if test else "main.rs")
    candidates: list[Path] = []
    finished = False
    for line in output.splitlines():
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except ValueError as error:
            raise ToolingError("Cargo returned malformed executable metadata") from error
        if not isinstance(message, dict):
            raise ToolingError("Cargo returned malformed executable metadata")
        if message.get("reason") == "build-finished":
            if finished or message.get("success") is not True:
                raise ToolingError("The E2E build did not finish successfully exactly once")
            finished = True
        target, profile = message.get("target"), message.get("profile")
        if (
            message.get("reason") == "compiler-artifact"
            and isinstance(target, dict)
            and target.get("name") == expected_name
            and target.get("kind") == expected_kind
            and target.get("src_path") == str(expected_source)
            and isinstance(profile, dict)
            and profile.get("test") is test
            and isinstance(message.get("executable"), str)
            and Path(message["executable"]).is_absolute()
        ):
            candidates.append(Path(message["executable"]))
    if not finished or len(candidates) != 1:
        raise ToolingError("Expected exactly one completed Revaer serving executable")
    return ServingExecutable(candidates[0], kind)


@dataclass(frozen=True)
class ApplicationArgs:
    executable: ServingExecutable
    database_url: str = field(repr=False)
    media_workspace: Path
    log_path: Path
    media_root_catalog: Path | None = None


class Application:
    def __init__(self, runner: Runner, root: Path, environment: Mapping[str, str]) -> None:
        self.runner, self.root, self.environment = runner, root, environment

    def verify(self, executable: ServingExecutable) -> None:
        if not executable.path.is_file():
            raise ToolingError("Cargo's serving executable no longer exists")
        if executable.kind == ServingKind.LIBRARY_TEST:
            result = self.runner.run(
                Invocation(
                    (str(executable.path), "--list", "--format=terse"),
                    self.root,
                    self.environment,
                    capture=True,
                    timeout=10,
                )
            )
            if f"{E2E_SERVING_ENTRY}: test" not in result.stdout.splitlines():
                raise ToolingError("The selected application has no E2E serving test entry")

    def start(self, args: ApplicationArgs) -> RunningProcess:
        self.verify(args.executable)
        return self.runner.start(
            Invocation(
                (str(args.executable.path), *args.executable.arguments),
                self.root,
                {
                    **self.environment,
                    **(
                        {"REVAER_MEDIA_ROOT_CATALOG_FILE": str(args.media_root_catalog)}
                        if args.media_root_catalog is not None
                        else {}
                    ),
                    "DATABASE_URL": args.database_url,
                    "REVAER_MEDIA_WORKSPACE_ROOT": str(args.media_workspace),
                    "REVAER_E2E_SERVING_ENTRY": "1"
                    if args.executable.kind == ServingKind.LIBRARY_TEST
                    else "0",
                },
                log_path=args.log_path,
            )
        )


class Listeners(ExternalTool):
    version_args = ("-v",)

    def require_free(self, port: int) -> None:
        # A socket probe also detects processes lsof cannot inspect. An occupied
        # service is never terminated merely because it uses an expected port.
        for family, host in ((socket.AF_INET, "127.0.0.1"), (socket.AF_INET6, "::1")):
            with socket.socket(family, socket.SOCK_STREAM) as connection:
                connection.settimeout(0.2)
                if connection.connect_ex((host, port)) == 0:
                    raise ToolingError(
                        f"Port {port} is in use; stop its service before running E2E"
                    )

    def require_owner(self, port: int, pid: int) -> None:
        if self.owners(port) != {pid}:
            raise ToolingError(f"Cannot prove that the owned process {pid} serves port {port}")

    def owners(self, port: int) -> frozenset[int]:
        output = self._invoke(
            ("-nP", f"-iTCP:{port}", "-sTCP:LISTEN", "-Fp"),
            capture=True,
            accepted_codes=(0, 1),
            timeout=10,
        ).stdout
        return frozenset(
            int(line[1:]) for line in output.splitlines() if re.fullmatch(r"p[0-9]+", line)
        )
