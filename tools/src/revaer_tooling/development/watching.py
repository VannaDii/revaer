"""Use watchfiles for native notifications and Git for Git-ignore semantics.

Imports are lazy because container commands use the core dependency group. They
must not import optional development SDKs just to dispatch an unrelated command.
"""

from collections.abc import Generator
from contextlib import closing
from pathlib import Path

from ..errors import ToolingError
from ..external.git import Git

# These are the original development loop's generated-output exceptions. The
# additional target directory prevents Cargo's output from restarting itself.
GENERATED = (
    "docs/api/openapi.json",
    "crates/revaer-ui/dist",
    "crates/revaer-ui/dist-serve",
    "artifacts",
    "target",
)


class FileWatcher:
    def __init__(self, root: Path, git: Git) -> None:
        self.root, self.git = root, git

    def verify(self) -> None:
        try:
            import watchfiles
        except ImportError as error:
            raise ToolingError(
                "Development watching requires uv sync --locked --group dev"
            ) from error
        if not callable(watchfiles.watch):
            raise ToolingError("The installed watchfiles package has no watch API")

    def changes(self) -> Generator[frozenset[Path]]:
        import watchfiles

        filtering = watchfiles.DefaultFilter(
            ignore_paths=tuple(self.root / name for name in GENERATED)
        )
        # Timeout yields allow readiness/child-exit checks while the tree is
        # quiet. Filesystem permission failures are never silently ignored.
        with closing(
            watchfiles.watch(
                self.root,
                watch_filter=filtering,
                rust_timeout=500,
                yield_on_timeout=True,
                ignore_permission_denied=False,
            )
        ) as changes:
            for batch in changes:
                names = tuple(sorted({str(Path(path).relative_to(self.root)) for _, path in batch}))
                ignored = self.git.ignored(names) if names else frozenset()
                yield frozenset(self.root / name for name in names if name not in ignored)
