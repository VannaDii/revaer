"""Python Semantic Release remains the authority for versions and notes."""

import tempfile
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from ..errors import ToolingError
from .base import ExternalTool


@dataclass(frozen=True)
class ReleasePlan:
    version: str
    tag: str
    notes: str
    required: bool


@dataclass(frozen=True)
class ReleaseRepository:
    """One publication destination shared by the release engine and asset CLI."""

    host: str
    name: str


def parse_outputs(content: str) -> dict[str, str]:
    result: dict[str, str] = {}
    lines = iter(content.splitlines())
    for line in lines:
        if "<<" in line:
            key, separator = line.split("<<", 1)
            values: list[str] = []
            for item in lines:
                if item == separator:
                    break
                values.append(item)
            else:
                raise ToolingError("Unterminated release output value")
            result[key] = "\n".join(values)
        elif "=" in line:
            key, value = line.split("=", 1)
            result[key] = value
        elif line:
            raise ToolingError("Malformed release output")
    return result


class SemanticRelease(ExternalTool):
    def repository(self, origin: str, remote: Mapping[str, object]) -> ReleaseRepository:
        """Resolve the engine's destination without duplicating Git URL parsing.

        Revaer's tag recovery uses origin. Reject a configuration change that
        would send engine and recovery operations to different remotes. The
        parser is part of PSR's documented Python API and covered by release tests.
        """
        # PSR imports GitPython, which probes Git at import time. Bootstrap and
        # diagnostics must remain usable before native prerequisites are installed.
        from semantic_release.helpers import parse_git_url

        if remote.get("name", "origin") != "origin" or remote.get("type") != "github":
            raise ToolingError("Revaer publication requires the GitHub origin remote")
        url = remote.get("url") or origin
        domain = (
            remote.get("domain")
            or self.environment.get("GITHUB_SERVER_URL")
            or "https://github.com"
        )
        if not isinstance(url, str) or not isinstance(domain, str):
            raise ToolingError("Release remote URL and domain must be explicit strings")
        parsed = parse_git_url(url)
        server = urlsplit(domain if "://" in domain else f"https://{domain}")
        if (
            not server.netloc
            or server.path.rstrip("/")
            or server.scheme != "https"
            or server.username
            or server.query
            or server.fragment
        ):
            raise ToolingError("Release server must be an HTTPS GitHub host")
        if parsed.scheme == "file" or "/" in parsed.namespace:
            raise ToolingError("Release origin must identify a GitHub owner and repository")
        origin_server = urlsplit(f"{parsed.scheme}://{parsed.netloc}")
        if origin_server.hostname != server.hostname or (
            parsed.scheme == "https" and origin_server.port != server.port
        ):
            raise ToolingError("Release origin and configured GitHub server disagree")
        expected_api = (
            "https://api.github.com"
            if server.netloc == "github.com"
            else f"https://{server.netloc}/api/v3"
        )
        api = remote.get("api_domain") or self.environment.get("GITHUB_API_URL") or expected_api
        if not isinstance(api, str) or api.rstrip("/") != expected_api:
            raise ToolingError("Release engine and GitHub CLI must use the same GitHub API")
        return ReleaseRepository(server.netloc, f"{parsed.namespace}/{parsed.repo_name}")

    def preview(self) -> ReleasePlan:
        # Preview uses exactly the publishing policy. A feature branch is not
        # silently treated as main; callers must preview an eligible checkout.
        with tempfile.TemporaryDirectory(prefix="revaer-release-preview-") as temporary:
            output = Path(temporary) / "output"
            self._invoke(
                (
                    "--config",
                    "release/semantic-release.toml",
                    "--noop",
                    "version",
                    "--no-commit",
                    "--no-tag",
                    "--no-changelog",
                    "--no-push",
                    "--no-vcs-release",
                    "--skip-build",
                ),
                env={"GITHUB_OUTPUT": str(output)},
            )
            if not output.is_file():
                raise ToolingError("Semantic Release did not produce a release result")
            values = parse_outputs(output.read_text(encoding="utf-8"))
        if (
            values.get("released") not in ("true", "false")
            or not values.get("version")
            or not values.get("tag")
        ):
            raise ToolingError("Semantic Release returned incomplete release metadata")
        return ReleasePlan(
            values["version"],
            values["tag"],
            values.get("release_notes", ""),
            values["released"] == "true",
        )

    def publish_version(self, plan: ReleasePlan) -> None:
        # The engine emits released=true before its remote release operation.
        # Isolate that output; only the task can report end-to-end completion.
        with tempfile.TemporaryDirectory(prefix="revaer-release-output-") as temporary:
            output = Path(temporary) / "output"
            self._invoke(
                (
                    "--config",
                    "release/semantic-release.toml",
                    "version",
                    "--no-commit",
                    "--no-changelog",
                    "--skip-build",
                ),
                env={"GITHUB_OUTPUT": str(output)},
            )
            values = parse_outputs(output.read_text(encoding="utf-8"))
        if values.get("tag") != plan.tag or values.get("released") != "true":
            raise ToolingError("Release result changed since preview; inspect the published tag")

    def restore_release(self, tag: str) -> None:
        """Use the engine's supported command to recreate missing release notes."""
        self._invoke(
            (
                "--config",
                "release/semantic-release.toml",
                "changelog",
                "--post-to-release-tag",
                tag,
            )
        )
