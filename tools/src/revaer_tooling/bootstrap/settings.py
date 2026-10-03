"""Kcov distribution settings, read only at the CLI wiring boundary."""

from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from urllib.parse import urlsplit

from ..errors import ToolingError


@dataclass(frozen=True)
class KcovSettings:
    install_root: Path | None
    archives_url: str


def load_kcov_settings(environment: Mapping[str, str]) -> KcovSettings:
    location = environment.get("REVAER_KCOV_ARCHIVES_URL") or (
        "https://github.com/SimonKagstrom/kcov/archive"
    )
    url = urlsplit(location)
    if (
        url.scheme != "https"
        or not url.hostname
        or url.username is not None
        or url.password is not None
        or url.query
        or url.fragment
    ):
        raise ToolingError("REVAER_KCOV_ARCHIVES_URL must be an HTTPS distribution directory")
    return KcovSettings(
        Path(environment["REVAER_KCOV_INSTALL_ROOT"])
        if environment.get("REVAER_KCOV_INSTALL_ROOT")
        else None,
        location.rstrip("/"),
    )
