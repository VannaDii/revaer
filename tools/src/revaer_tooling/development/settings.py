"""Development settings resolved once, before any services start."""

from collections.abc import Mapping
from dataclasses import dataclass

from ..errors import ToolingError


@dataclass(frozen=True)
class DevelopmentSettings:
    startup_timeout: int
    skip_port_check: bool
    api_log: str
    ui_log: str


def load_development_settings(environment: Mapping[str, str]) -> DevelopmentSettings:
    try:
        timeout = int(environment.get("DEV_STARTUP_TIMEOUT") or "180")
    except ValueError as error:
        raise ToolingError("DEV_STARTUP_TIMEOUT must be a positive integer") from error
    if timeout < 1:
        raise ToolingError("DEV_STARTUP_TIMEOUT must be a positive integer")
    skip = environment.get("DEV_SKIP_PORT_CHECK") or "0"
    if skip not in ("0", "1"):
        raise ToolingError("DEV_SKIP_PORT_CHECK must be 0 or 1")
    return DevelopmentSettings(
        timeout,
        skip == "1",
        environment.get("RUST_LOG") or "debug",
        environment.get("RUST_LOG") or "info",
    )
