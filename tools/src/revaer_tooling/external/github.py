"""GitHub's CLI owns transfer; rv checks existing and uploaded asset digests.

A retry uploads only missing files. A conflicting remote asset is an error, even
when the caller could overwrite it: a published tag must identify one artifact.
"""

import json
import re
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import quote

from ..artifacts import digest_file
from ..errors import ToolingError
from ..json_data import array_value, decode, object_value
from .base import ExternalTool
from .release import ReleaseRepository


@dataclass(frozen=True)
class ReleaseAsset:
    name: str
    size: int
    digest: str | None
    state: str


@dataclass(frozen=True)
class PullRequestArgs:
    repository: ReleaseRepository
    branch: str

    def validate(self) -> None:
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", self.repository.name):
            raise ToolingError("Pull request lookup requires an owner/repository")
        if not self.branch or any(ord(character) < 32 for character in self.branch):
            raise ToolingError(
                "Pull request lookup requires a nonempty branch without control characters"
            )


@dataclass(frozen=True)
class GraphqlArgs:
    repository: ReleaseRepository
    query: str
    cursor: str | None = None


class GitHub(ExternalTool):
    def graphql(self, args: GraphqlArgs) -> str:
        """Return one provider page unchanged for the caller's strict decoder."""
        if (
            not args.query
            or "\0" in args.query
            or (args.cursor is not None and (not args.cursor or "\0" in args.cursor))
        ):
            raise ToolingError("GitHub GraphQL requires a query and an opaque nonempty cursor")
        return self._invoke(
            (
                "api",
                "--hostname",
                args.repository.host,
                "graphql",
                "--raw-field",
                "query=" + args.query,
                *(("--raw-field", "cursor=" + args.cursor) if args.cursor is not None else ()),
            ),
            env=self._environment(args.repository),
            capture=True,
            timeout=60,
        ).stdout

    def open_pull_request(self, args: PullRequestArgs) -> int | None:
        """Resolve a single open PR through gh's supported, encoded query fields.

        Two results are enough to detect ambiguity. Do not pick an arbitrary PR
        or conflate an authentication/transport failure with an absent match.
        """
        args.validate()
        owner = args.repository.name.split("/", 1)[0]
        response = self._invoke(
            (
                "api",
                "--method",
                "GET",
                f"repos/{args.repository.name}/pulls",
                "--raw-field",
                f"head={owner}:{args.branch}",
                "--raw-field",
                "state=open",
                "--raw-field",
                "per_page=2",
            ),
            env=self._environment(args.repository),
            capture=True,
            timeout=60,
        )
        matches = array_value(decode(response.stdout))
        if not matches:
            return None
        if len(matches) != 1:
            raise ToolingError("More than one open pull request matches the selected branch")
        item = object_value(matches[0])
        number = item.get("number")
        head = object_value(item.get("head"))
        repository = object_value(head.get("repo"))
        if (
            not isinstance(number, int)
            or isinstance(number, bool)
            or number < 1
            or item.get("state") != "open"
            or head.get("ref") != args.branch
            or str(repository.get("full_name", "")).lower() != args.repository.name.lower()
        ):
            raise ToolingError("GitHub returned an unexpected pull request identity")
        return number

    def _environment(self, repository: ReleaseRepository) -> dict[str, str]:
        """Do not let an inherited gh override redirect part of a release."""
        host = self.environment.get("GH_HOST")
        configured = self.environment.get("GH_REPO")
        expected = f"{repository.host}/{repository.name}".lower()
        if host and host.lower() != repository.host.lower():
            raise ToolingError("GH_HOST conflicts with the release engine's repository")
        if configured:
            candidate = configured.removeprefix("https://").rstrip("/").removesuffix(".git")
            if candidate.count("/") == 1:
                candidate = f"{repository.host}/{candidate}"
            if candidate.lower() != expected:
                raise ToolingError("GH_REPO conflicts with the release engine's repository")
        return {"GH_HOST": repository.host, "GH_REPO": repository.name}

    def _release_assets(
        self, repository: ReleaseRepository, tag: str
    ) -> tuple[ReleaseAsset, ...] | None:
        # Include HTTP status so an absent release can be distinguished from an
        # authentication, transport, or malformed-response failure during recovery.
        response = self._invoke(
            (
                "api",
                "--include",
                f"repos/{repository.name}/releases/tags/{quote(tag, safe='')}",
            ),
            env=self._environment(repository),
            capture=True,
            accepted_codes=(0, 1),
        )
        # gh --include emits native HTTP CRLF on some platforms. The shared
        # process runner preserves those bytes because other tools validate exact
        # evidence. Recognize the HTTP separator here without rewriting the body.
        parts = re.split(r"\r?\n\r?\n", response.stdout, maxsplit=1)
        headers = parts[0]
        status = re.match(r"HTTP/\S+ (\d{3})\b", headers)
        if status and status[1] == "404":
            return None
        if response.code != 0 or not status or status[1] != "200" or len(parts) != 2:
            raise ToolingError("Cannot inspect the expected GitHub release")
        try:
            release = json.loads(parts[1])
        except ValueError as error:
            raise ToolingError("GitHub returned invalid release metadata") from error
        if (
            not isinstance(release, dict)
            or release.get("tag_name") != tag
            or release.get("draft") is not False
            or not isinstance(release.get("assets"), list)
        ):
            raise ToolingError("GitHub did not return the expected published release")
        assets: list[ReleaseAsset] = []
        for item in release["assets"]:
            if (
                not isinstance(item, dict)
                or not isinstance(item.get("name"), str)
                or not isinstance(item.get("size"), int)
                or not (item.get("digest") is None or isinstance(item.get("digest"), str))
                or not isinstance(item.get("state"), str)
            ):
                raise ToolingError("GitHub returned incomplete asset metadata")
            assets.append(
                ReleaseAsset(item["name"], item["size"], item.get("digest"), item["state"])
            )
        return tuple(assets)

    def _missing_assets(
        self,
        repository: ReleaseRepository,
        tag: str,
        paths: tuple[Path, ...],
        *,
        require_release: bool,
    ) -> tuple[Path, ...]:
        assets = self._release_assets(repository, tag)
        if assets is None:
            if require_release:
                raise ToolingError("The expected GitHub release does not exist")
            return paths
        missing: list[Path] = []
        for path in paths:
            matching = [item for item in assets if item.name == path.name]
            if not matching:
                missing.append(path)
                continue
            expected = ReleaseAsset(
                path.name, path.stat().st_size, f"sha256:{digest_file(path)}", "uploaded"
            )
            if matching != [expected]:
                raise ToolingError(f"Conflicting GitHub release asset: {path.name}")
        return tuple(missing)

    def check_existing_assets(
        self, repository: ReleaseRepository, tag: str, paths: tuple[Path, ...]
    ) -> None:
        """Reject conflicts before publication makes any additional remote change."""
        self._missing_assets(repository, tag, paths, require_release=False)

    def upload_assets(
        self, repository: ReleaseRepository, tag: str, paths: tuple[Path, ...]
    ) -> None:
        missing = self._missing_assets(repository, tag, paths, require_release=True)
        if missing:
            self._invoke(
                ("release", "upload", tag, *(str(path) for path in missing)),
                env=self._environment(repository),
            )

    def verify_assets(
        self, repository: ReleaseRepository, tag: str, paths: tuple[Path, ...]
    ) -> None:
        missing = self._missing_assets(repository, tag, paths, require_release=True)
        if missing:
            names = ", ".join(path.name for path in missing)
            raise ToolingError(f"GitHub release assets are missing: {names}")

    def create_stable_release(self, repository: ReleaseRepository, tag: str) -> None:
        """Preserve the stable-tag workflow's title and existing release notes."""
        if self._release_assets(repository, tag) is None:
            self._invoke(
                (
                    "release",
                    "create",
                    tag,
                    "--verify-tag",
                    "--title",
                    f"Revaer {tag}",
                    "--notes",
                    "",
                ),
                env=self._environment(repository),
            )
