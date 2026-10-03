"""Git queries used by policy, releases, and worktree validation."""

import re
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from .base import ExternalTool


def object_id(value: str) -> str:
    if not re.fullmatch(r"[0-9a-f]{40,64}", value):
        raise ToolingError("Git evidence requires a complete lowercase object ID")
    return value


@dataclass(frozen=True)
class DiffArgs:
    """Resolved commit identities; names are resolved once before reading a diff."""

    base: str
    head: str

    def revisions(self) -> tuple[str, str]:
        return object_id(self.base), object_id(self.head)


@dataclass(frozen=True)
class CommitFileArgs:
    revision: str
    path: str

    def expression(self) -> str:
        if (
            not self.path
            or Path(self.path).is_absolute()
            or ".." in Path(self.path).parts
            or "\0" in self.path
        ):
            raise ToolingError("Git evidence requires a repository-relative path")
        return object_id(self.revision) + ":" + self.path


@dataclass(frozen=True)
class ScmArgs:
    base: str
    branch: str
    head: str

    def validate(self) -> None:
        for name, value in (("SONAR_BASE_SHA", self.base), ("SONAR_HEAD_SHA", self.head)):
            if re.fullmatch(r"[a-f0-9]{40}", value) is None:
                raise ToolingError(f"{name} must be an exact lowercase 40-character Git SHA")
        if (
            re.fullmatch(r"[A-Za-z0-9._/-]+", self.branch) is None
            or self.branch.startswith("/")
            or self.branch.endswith("/")
            or ".." in self.branch
        ):
            raise ToolingError("SONAR_BASE_REF must be a valid branch name")


class Git(ExternalTool):
    def exact_commit(self, ref: str) -> str:
        if not ref:
            raise ToolingError("Git diff reference must not be empty")
        return object_id(self.revision(ref + "^{commit}"))

    def numstat(self, args: DiffArgs) -> str:
        return self._invoke(
            (
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--numstat",
                "-z",
                *args.revisions(),
                "--",
            ),
            capture=True,
        ).stdout

    def raw_diff(self, args: DiffArgs) -> str:
        return self._invoke(
            (
                "diff",
                "--no-ext-diff",
                "--no-textconv",
                "--no-renames",
                "--raw",
                "--abbrev=40",
                "-z",
                *args.revisions(),
                "--",
            ),
            capture=True,
        ).stdout

    def is_ancestor(self, args: DiffArgs) -> bool:
        result = self._invoke(
            ("merge-base", "--is-ancestor", *args.revisions()),
            capture=True,
            accepted_codes=(0, 1),
        )
        return result.code == 0

    def blob(self, oid: str) -> bytes:
        result = self._invoke(("cat-file", "blob", object_id(oid)), capture_binary=True)
        if result.stdout_bytes is None:
            raise ToolingError("Git blob transport did not return binary evidence")
        return result.stdout_bytes

    def committed_file(self, args: CommitFileArgs) -> bytes:
        result = self._invoke(("show", args.expression()), capture_binary=True)
        if result.stdout_bytes is None:
            raise ToolingError("Git file transport did not return binary evidence")
        return result.stdout_bytes

    def ignored(self, names: tuple[str, ...]) -> frozenset[str]:
        """Ask Git to apply nested/global ignores without inventing a glob parser."""
        if not names:
            return frozenset()
        if any(
            Path(name).is_absolute() or ".." in Path(name).parts or "\0" in name for name in names
        ):
            raise ToolingError("Git ignore queries require paths within the selected checkout")
        result = self._invoke(
            ("check-ignore", "--stdin", "-z"),
            input_text="\0".join(names) + "\0",
            capture=True,
            accepted_codes=(0, 1),
        )
        ignored = frozenset(name for name in result.stdout.split("\0") if name)
        if not ignored.issubset(names):
            raise ToolingError("Git returned an unexpected ignored path")
        return ignored

    def prepare_scm(self, args: ScmArgs) -> str:
        """Fetch complete history and bind analysis to the event's exact commits.

        Only origin's tracking refs are refreshed. The selected worktree's HEAD,
        branch, index, and working files are never moved by history preparation.
        """
        args.validate()
        self._invoke(("check-ref-format", "refs/heads/" + args.branch))
        if self.revision("HEAD^{commit}") != args.head:
            raise ToolingError("Checked-out HEAD does not match SONAR_HEAD_SHA")
        self.remote_url()
        shallow = self._invoke(
            ("rev-parse", "--is-shallow-repository"), capture=True
        ).stdout.strip()
        if shallow not in ("true", "false"):
            raise ToolingError("Git did not report its shallow-history state")
        self._invoke(
            (
                "fetch",
                "--no-tags",
                "--prune",
                *(("--unshallow",) if shallow == "true" else ()),
                "origin",
            )
        )
        self._invoke(
            (
                "fetch",
                "--no-tags",
                "origin",
                f"+refs/heads/{args.branch}:refs/remotes/origin/{args.branch}",
            )
        )
        self._invoke(("fetch", "--no-tags", "origin", args.base))
        self.revision(args.base + "^{commit}")
        self.revision(args.head + "^{commit}")
        if (
            self._invoke(("rev-parse", "--is-shallow-repository"), capture=True).stdout.strip()
            != "false"
        ):
            raise ToolingError("Sonar checkout remains shallow after history preparation")
        ancestor = self._invoke(("merge-base", args.base, args.head), capture=True).stdout.strip()
        if ancestor != args.base:
            raise ToolingError(
                "Exact Sonar base is not an ancestor of HEAD; restack onto the event base"
            )
        self._remove_empty_shallow_marker()
        count = self._invoke(("rev-list", "--count", "--all"), capture=True).stdout.strip()
        if not count.isdecimal() or int(count) < 1:
            raise ToolingError("Git did not report a positive reachable commit count")
        return (
            f"base_ref={args.branch}\nbase_sha={args.base}\nhead_sha={args.head}\n"
            f"merge_base={ancestor}\nshallow=false\nreachable_commit_count={count}\n"
        )

    def _remove_empty_shallow_marker(self) -> None:
        # Git sometimes leaves an empty marker; the scanner treats its presence
        # as shallow. Git resolves its own metadata path, including worktrees.
        # Never remove a nonempty marker or follow a link into unrelated data.
        name = self._invoke(
            ("rev-parse", "--path-format=absolute", "--git-path", "shallow"), capture=True
        ).stdout.strip()
        marker = Path(name)
        if not marker.is_absolute() or marker.is_symlink():
            raise ToolingError("Git shallow marker must be an absolute, regular metadata path")
        if marker.exists():
            if not marker.is_file() or marker.stat().st_size:
                raise ToolingError("Nonempty Git shallow history must not be discarded")
            marker.unlink()

    def remote_url(self) -> str:
        """Use Git's expanded URL, including its configured insteadOf rules."""
        return self._invoke(("remote", "get-url", "origin"), capture=True).stdout.strip()

    def files(self, include_untracked: bool = False) -> tuple[str, ...]:
        args = ("ls-files", "-z")
        tracked = self._invoke(args, capture=True).stdout.split("\0")
        if include_untracked:
            tracked.extend(
                self._invoke((*args, "--others", "--exclude-standard"), capture=True).stdout.split(
                    "\0"
                )
            )
        return tuple(sorted({path for path in tracked if path}))

    def revision(self, ref: str = "HEAD") -> str:
        return self._invoke(
            ("rev-parse", "--verify", "--end-of-options", ref), capture=True
        ).stdout.strip()

    def changed_files(self, base: str | None = None, head: str = "HEAD") -> tuple[str, ...]:
        if base:
            return self._diff(base, head)
        unstaged = self._invoke(("diff", "--name-only", "-z"), capture=True).stdout
        staged = self._invoke(("diff", "--cached", "--name-only", "-z"), capture=True).stdout
        untracked = self._invoke(
            ("ls-files", "--others", "--exclude-standard", "-z"), capture=True
        ).stdout
        changed = {path for path in (unstaged + staged + untracked).split("\0") if path}
        if changed:
            return tuple(sorted(changed))
        # A clean feature checkout still needs instruction-drift checks against
        # its branch diff. Only an absent ref permits the initial-repo fallback.
        main = self._optional_revision("origin/main")
        if main is not None:
            ancestor = self._invoke(
                ("merge-base", main, self.revision("HEAD")), capture=True
            ).stdout.strip()
            return self._diff(ancestor, "HEAD")
        parent = self._optional_revision("HEAD^")
        return self._diff(parent, "HEAD") if parent is not None else ()

    def _optional_revision(self, ref: str) -> str | None:
        result = self._invoke(
            ("rev-parse", "--verify", "--quiet", "--end-of-options", f"{ref}^{{commit}}"),
            capture=True,
            accepted_codes=(0, 1),
        )
        return result.stdout.strip() if result.code == 0 else None

    def _diff(self, base: str, head: str) -> tuple[str, ...]:
        # Resolve names before passing them to diff: user-provided refs cannot
        # be interpreted as Git options, and invalid refs are always errors.
        before = self.revision(f"{base}^{{commit}}")
        after = self.revision(f"{head}^{{commit}}")
        output = self._invoke(
            ("diff", "--name-only", "-z", before, after, "--"), capture=True
        ).stdout
        return tuple(path for path in output.split("\0") if path)

    def require_clean(self) -> None:
        if self._invoke(("status", "--porcelain"), capture=True).stdout.strip():
            raise ToolingError("Release publication requires a clean checkout")

    def check_assets(self) -> None:
        changed = self._invoke(
            (
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--",
                "crates/revaer-ui/static/nexus",
            ),
            capture=True,
        ).stdout
        if changed:
            raise ToolingError(
                "Generated UI assets differ from Git; run rv sync-assets and commit them"
            )

    def push_tag(self, tag: str) -> None:
        """Retry one existing tag without changing the branch or forcing a ref."""
        self._invoke(("push", "origin", f"refs/tags/{tag}:refs/tags/{tag}"))

    def require_remote_tag(self, tag: str, source: str) -> None:
        """Resolve lightweight and annotated tags without creating or moving refs."""
        ref = f"refs/tags/{tag}"
        output = self._invoke(
            ("ls-remote", "--exit-code", "origin", ref, f"{ref}^{{}}"), capture=True
        ).stdout
        refs = {name: sha for line in output.splitlines() for sha, name in [line.split()]}
        if refs.get(f"{ref}^{{}}", refs.get(ref)) != source:
            raise ToolingError("Remote release tag does not match the source commit")
