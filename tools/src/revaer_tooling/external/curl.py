"""Native curl owns HTTPS redirects, certificate trust, retries and deadlines."""

from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..media.model import https_url
from ..process import Completed
from .base import ExternalTool


@dataclass(frozen=True)
class DownloadArgs:
    url: str
    output: Path
    maximum_bytes: int
    connect_timeout: int = 15
    deadline: int = 120


class Curl(ExternalTool):
    def download(self, args: DownloadArgs) -> Completed:
        https_url(args.url)
        if min(args.maximum_bytes, args.connect_timeout, args.deadline) < 1:
            raise ToolingError("Curl download limits must be positive")
        if not args.output.is_absolute() or args.output.is_symlink() or args.output.exists():
            raise ToolingError("Curl requires a new absolute staging file")
        # --disable must be first: personal curlrc files must not add another
        # transfer or override the locked fixture's transport restrictions.
        # Curl's retry deadline is checked before each attempt. An attempt may
        # consume another max-time; the process timeout bounds that final attempt.
        return self._invoke(
            (
                "--disable",
                "--fail",
                "--location",
                "--show-error",
                "--silent",
                "--globoff",
                "--proto",
                "=https",
                "--proto-redir",
                "=https",
                "--connect-timeout",
                str(args.connect_timeout),
                "--max-time",
                str(args.deadline),
                "--retry-max-time",
                str(args.deadline),
                "--max-filesize",
                str(args.maximum_bytes),
                "--retry",
                "3",
                "--retry-delay",
                "2",
                "--output",
                str(args.output),
                "--url",
                args.url,
            ),
            capture=True,
            timeout=2 * args.deadline + 5,
        )
