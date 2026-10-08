"""Typed native operations for the existing disposable database SQL protocol."""

import re
from dataclasses import dataclass, field
from pathlib import Path

from ..errors import ToolingError
from .base import ExternalTool
from .postgres import container_id, identifier


@dataclass(frozen=True)
class LifecycleQuery:
    role: str
    database: str
    name: str
    script: str
    password: str = field(repr=False)
    runtime_password: str = field(default="", repr=False)
    variables: tuple[tuple[str, str], ...] = ()


class LifecycleDocker(ExternalTool):
    def select(self, selected: str, image: str) -> str:
        if not re.fullmatch(r"[a-zA-Z0-9][a-zA-Z0-9_.-]*", selected):
            raise ToolingError("Select an explicit Docker container name or ID")
        actual = self._invoke(
            ("inspect", "--format", "{{.Id}} {{.Config.Image}}", selected), capture=True
        ).stdout.split()
        if len(actual) != 2 or actual[1] != image:
            raise ToolingError("Test database container does not use the pinned PostgreSQL image")
        return container_id(actual[0])

    def binding(self, container: str) -> str:
        return self._invoke(("port", container_id(container), "5432/tcp"), capture=True).stdout

    def stage(self, container: str, directory: str) -> None:
        # mkdir without -p refuses collisions; cleanup is armed only afterward.
        self._invoke(
            ("exec", container_id(container), "mkdir", "--mode=700", "--", directory),
            capture=True,
        )

    def copy(self, container: str, source: Path, destination: str) -> None:
        # Docker's archive-copy endpoint cannot reliably address tmpfs staging.
        # Stream UTF-8 SQL through native tee in the selected container instead;
        # no shell interpolation or host mount is involved.
        self._invoke(
            ("exec", "-i", container_id(container), "tee", destination),
            input_text=source.read_text(encoding="utf-8"),
            capture=True,
        )

    def digest(self, container: str, source: str) -> str:
        fields = self._invoke(
            ("exec", container_id(container), "sha256sum", source), capture=True
        ).stdout.split()
        if len(fields) != 2 or not re.fullmatch(r"[0-9a-f]{64}", fields[0]):
            raise ToolingError("Staged initializer has no valid SHA-256 digest")
        return fields[0]

    def query(self, container: str, args: LifecycleQuery) -> None:
        variables = tuple(value for pair in args.variables for value in ("-v", "=".join(pair)))
        self._invoke(
            (
                "exec",
                "-i",
                "-e",
                "PGPASSWORD",
                "-e",
                "REVAER_TEST_RUNTIME_PASSWORD",
                container_id(container),
                "psql",
                "-X",
                "-q",
                "--no-password",
                "-h",
                "127.0.0.1",
                "-U",
                args.role,
                "-d",
                identifier(args.database),
                "-v",
                "ON_ERROR_STOP=1",
                "-v",
                "VERBOSITY=terse",
                "-v",
                "SHOW_CONTEXT=never",
                "-v",
                f"database_name={identifier(args.name)}",
                *variables,
                "-f",
                args.script,
            ),
            env={
                "PGPASSWORD": args.password,
                "REVAER_TEST_RUNTIME_PASSWORD": args.runtime_password,
            },
            capture=True,
        )

    def cleanup(self, container: str, directory: str) -> None:
        self._invoke(("exec", container_id(container), "rm", "-r", "--", directory), capture=True)
