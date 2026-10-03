"""Development entry points; services and failure recovery are shared components."""

import tomllib

from ..context import Context, TaskResult
from ..development.loop import DevelopmentLoop
from ..development.session import Session, directory
from ..errors import ToolingError
from .base import Task
from .build import SyncAssets
from .database import prepared_database


class Development(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.watcher.verify()
        context.tools.processes.verify()
        state = directory(context.root, context.fs)
        with context.fs.lock(state / "run.lock"):
            receipt = state / "session.json"
            if receipt.exists() or receipt.is_symlink():
                raise ToolingError("A development recovery receipt remains; run rv zombies first")
            if not context.settings.development.skip_port_check:
                for port in (7070, 8080):
                    context.tools.listeners.require_free(port)
            pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))["cargo"]
            context.tools.trunk.verify(str(pins["trunk"]["version"]))
            context.tools.rustup.target_add("wasm32-unknown-unknown")
            SyncAssets.run(context)
            context.fs.mkdir(context.root / "crates/revaer-ui/dist-serve/.stage")
            with prepared_database(context) as url:
                DevelopmentLoop(context, url, state).run()
        return TaskResult()


class Zombies(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        context.tools.processes.verify()
        state = directory(context.root, context.fs)
        receipt = state / "session.json"
        if not receipt.exists() and not receipt.is_symlink():
            return TaskResult("No recorded development processes in this checkout")
        # Read without the long-lived run lock: the supervisor must be able to
        # receive SIGTERM and release that lock through its normal cleanup.
        session = Session.read(context.fs, receipt, context.root)
        context.tools.processes.stop(session.supervisor, session.workers)
        with context.fs.lock(state / "run.lock"):
            if receipt.exists() or receipt.is_symlink():
                if Session.read(context.fs, receipt, context.root) != session:
                    raise ToolingError("Development ownership changed during cleanup; retry")
                context.fs.remove_owned(receipt, context.root)
        return TaskResult("Recorded development processes stopped")
