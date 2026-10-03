"""Recover only processes whose checkout and creation identity were recorded.

psutil owns process inspection, PID-reuse checks and bounded waits. Unix session
IDs connect orphaned descendants to the new session created by ProcessRunner;
ports and executable names are never sufficient authority to terminate a process.
"""

from __future__ import annotations

import math
import os
import re
from collections.abc import Callable, Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from ..errors import ToolingError

if TYPE_CHECKING:
    import psutil


@dataclass(frozen=True)
class ProcessIdentity:
    pid: int
    created: float
    token: str = ""

    def validate(self) -> None:
        if self.pid <= 1 or not math.isfinite(self.created) or self.created <= 0:
            raise ToolingError("Invalid development process identity")
        if self.token and re.fullmatch(r"[a-f0-9]{64}", self.token) is None:
            raise ToolingError("Invalid development session token")


class ProcessManager:
    def __init__(
        self,
        root: Path,
        pid: int,
        uid: int,
        clock: Callable[[], float],
        token: Callable[[], str],
    ) -> None:
        self.root, self.pid, self.uid, self.clock = root, pid, uid, clock
        self.token = token

    def verify(self) -> None:
        try:
            import psutil
        except ImportError as error:
            raise ToolingError("Process cleanup requires uv sync --locked --group dev") from error
        if not callable(psutil.Process):
            raise ToolingError("The installed psutil package has no Process API")

    def _checkout(self, process: psutil.Process) -> None:
        if process.uids().real != self.uid or not Path(process.cwd()).is_relative_to(self.root):
            raise ToolingError(f"Process {process.pid} belongs to another user or checkout")

    def identity(self, pid: int) -> ProcessIdentity | None:
        """Absence means an already-exited child, never a failed inspection."""
        import psutil

        try:
            process = psutil.Process(pid)
            if process.status() == psutil.STATUS_ZOMBIE:
                return None
            self._checkout(process)
            return ProcessIdentity(
                pid,
                process.create_time(),
                process.environ().get("RV_DEV_SESSION", ""),
            )
        except psutil.NoSuchProcess:
            return None
        except psutil.Error as error:
            raise ToolingError(f"Cannot inspect development process {pid}: {error}") from error

    def _matching(self, identity: ProcessIdentity) -> psutil.Process | None:
        import psutil

        identity.validate()
        try:
            process = psutil.Process(identity.pid)
            if process.create_time() != identity.created:
                return None  # The old PID has been reused; it is not ours.
            self._checkout(process)
            return process
        except psutil.NoSuchProcess:
            return None

    def members(self, identity: ProcessIdentity) -> tuple[psutil.Process, ...]:
        """Include surviving session members even after the group leader exits.

        A reused leader PID invalidates the old receipt. Each member is captured
        as a fresh psutil Process, which rechecks creation time when signalling.
        Descendants may change cwd during a native build; their original session
        and user, rather than their present directory, establish membership.
        """
        import psutil

        identity.validate()
        if identity.pid == self.pid:
            raise ToolingError("Refusing to stop the cleanup command's own session")
        try:
            try:
                leader = psutil.Process(identity.pid)
                if leader.create_time() != identity.created:
                    return ()
                self._checkout(leader)
                if os.getsid(leader.pid) != leader.pid:
                    raise ToolingError("Recorded development worker is not an owned session")
            except (ProcessLookupError, psutil.NoSuchProcess):
                # A session retains its ID while orphaned descendants live.
                leader = None
            selected: list[psutil.Process] = []
            if not identity.token:
                raise ToolingError("Development worker has no recorded session token")
            for pid in psutil.pids():
                try:
                    if os.getsid(pid) != identity.pid:
                        continue
                    process = psutil.Process(pid)
                    if process.uids().real != self.uid or process.create_time() < identity.created:
                        raise ToolingError("Development session membership no longer agrees")
                    if process.status() != psutil.STATUS_ZOMBIE:
                        # A session ID can itself be reused after every member
                        # exits. The inherited random launch token distinguishes
                        # its orphaned members from a later session with that ID.
                        if process.environ().get("RV_DEV_SESSION") != identity.token:
                            raise ToolingError("Development session token no longer agrees")
                        selected.append(process)
                except (ProcessLookupError, psutil.NoSuchProcess):
                    continue  # Exiting between enumeration and inspection is normal.
            if any(process.pid == self.pid for process in selected):
                raise ToolingError("Refusing to stop the cleanup command's own session")
            return tuple(selected)
        except (OSError, psutil.Error) as error:
            raise ToolingError(f"Cannot verify owned development session: {error}") from error

    def require_listener(self, owners: frozenset[int], identity: ProcessIdentity) -> bool:
        if not owners:
            return False
        members = {process.pid for process in self.members(identity)}
        if not owners.issubset(members):
            raise ToolingError("Development port is served by an unowned process")
        return True

    def stop(self, supervisor: ProcessIdentity, workers: tuple[ProcessIdentity, ...]) -> None:
        """Preflight every receipt before signalling, then allow normal cleanup."""
        import psutil

        if supervisor.pid == self.pid:
            raise ToolingError("Refusing to stop the cleanup command itself")
        try:
            parent = self._matching(supervisor)
            for worker in workers:
                self.members(worker)
            if parent is not None:
                self._terminate((parent,), grace=15)
            # The supervisor may have cleaned up normally. Refresh the sessions
            # after it exits, so its last restart cannot escape recovery.
            self.stop_workers(workers)
        except psutil.Error as error:
            raise ToolingError(f"Cannot stop owned development processes: {error}") from error

    def stop_workers(self, workers: tuple[ProcessIdentity, ...]) -> None:
        """Reap groups even when a parent exited after closing all its pipes."""
        import psutil

        try:
            groups = tuple(self.members(worker) for worker in workers)
            for group in groups:
                self._terminate(group, grace=5)
        except psutil.Error as error:
            raise ToolingError(f"Cannot stop owned development workers: {error}") from error

    @staticmethod
    def _terminate(processes: Iterable[psutil.Process], *, grace: int) -> None:
        import psutil

        pending = list(processes)
        for process in pending:
            try:
                process.terminate()
            except psutil.NoSuchProcess:
                continue
        _, alive = psutil.wait_procs(pending, timeout=grace)
        for process in alive:
            try:
                process.kill()
            except psutil.NoSuchProcess:
                continue
        _, alive = psutil.wait_procs(alive, timeout=5)
        # An exited orphan can await its system reaper; it no longer owns ports
        # or runs code. A still-running process after SIGKILL is a cleanup error.
        if any(process.status() != psutil.STATUS_ZOMBIE for process in alive):
            raise ToolingError("Owned development processes did not stop after SIGKILL")
