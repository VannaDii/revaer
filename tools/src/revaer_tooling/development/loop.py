"""Small coordinator around native Cargo/Trunk and injected filesystem events."""

from contextlib import closing
from dataclasses import dataclass
from pathlib import Path

from ..context import Context
from ..errors import CommandError, ToolingError
from ..external.rust import DevelopmentArgs
from ..process import RunningProcess
from .processes import ProcessIdentity
from .session import Session


@dataclass
class Service:
    process: RunningProcess
    identity: ProcessIdentity | None
    ready: bool = False


class DevelopmentLoop:
    """One API process at a time; Trunk owns its existing live-reload pipeline."""

    def __init__(self, context: Context, database_url: str, state: Path) -> None:
        self.context, self.database_url, self.state = context, database_url, state
        self.services: dict[str, Service] = {}
        self.generation = 0
        self.deadline = 0.0
        self.started = False
        supervisor = context.tools.processes.identity(context.tools.processes.pid)
        if supervisor is None:
            raise ToolingError("Cannot record the development supervisor")
        self.supervisor = supervisor

    def _record(self) -> None:
        Session(
            self.context.root,
            self.supervisor,
            tuple(item.identity for item in self.services.values() if item.identity is not None),
        ).write(self.context.fs, self.state / "session.json")

    def _start(self, name: str) -> None:
        context = self.context
        api = name == "API"
        number = str(self.generation) if api else "serve"
        args = DevelopmentArgs(
            self.database_url,
            context.settings.development.api_log if api else context.settings.development.ui_log,
            self.state / f"{name.lower()}-{number}.log",
            context.tools.processes.token(),
        )
        process = (
            context.tools.cargo.development(args) if api else context.tools.trunk.development(args)
        )
        # Register the child for in-process cleanup before any fallible identity
        # lookup or receipt write. Even a failed ownership check cannot leak it.
        service = Service(process, None)
        self.services[name] = service
        service.identity = context.tools.processes.identity(process.pid)
        self._record()

    def _restart(self) -> None:
        previous = self.services.get("API")
        if previous is not None:
            previous.process.stop()
            if previous.identity is not None:
                self.context.tools.processes.stop_workers((previous.identity,))
            del self.services["API"]
        self.generation += 1
        self.deadline = (
            self.context.tools.processes.clock() + self.context.settings.development.startup_timeout
        )
        self._start("API")

    def _check(self) -> None:
        context = self.context
        for name, service in tuple(self.services.items()):
            code = service.process.poll()
            if code is not None:
                if name == "Trunk":
                    service.process.wait()
                    raise ToolingError("Trunk exited; development services stopped")
                try:
                    service.process.wait()
                except CommandError as error:
                    context.emit(f"API build/run failed ({error.exit_code}); waiting for an edit")
                else:
                    context.emit("API exited; waiting for an edit")
                # wait() drains evidence; stop() also owns any surviving group.
                service.process.stop()
                if service.identity is not None:
                    context.tools.processes.stop_workers((service.identity,))
                del self.services[name]
                self._record()
                continue
            if not service.ready:
                port = 7070 if name == "API" else 8080
                if service.identity is None:
                    raise ToolingError("A running development process has no ownership identity")
                service.ready = context.tools.processes.require_listener(
                    context.tools.listeners.owners(port),
                    service.identity,
                )
                if service.ready:
                    context.emit(f"{name} ready on port {port}")
                elif context.tools.processes.clock() >= self.deadline:
                    raise ToolingError(f"Timed out waiting for {name} to listen on port {port}")

    def run(self) -> None:
        try:
            self._record()
            self.started = True
            self._restart()
            self._start("Trunk")
            self.context.emit("Watching this checkout; Ctrl+C or rv zombies stops its services")
            with closing(self.context.tools.watcher.changes()) as changes:
                for paths in changes:
                    if paths:
                        self.context.emit(f"Restarting API after {len(paths)} changed path(s)")
                        self._restart()
                    self._check()
            raise ToolingError("Development file watcher stopped unexpectedly")
        finally:
            failures: list[str] = []
            for service in reversed(tuple(self.services.values())):
                try:
                    service.process.stop()
                    if service.identity is not None:
                        self.context.tools.processes.stop_workers((service.identity,))
                except (OSError, ToolingError) as error:
                    failures.append(str(error))
            if failures:
                raise ToolingError("Development cleanup failed: " + "; ".join(failures))
            if self.started:
                self.context.fs.remove_owned(self.state / "session.json", self.context.root)
