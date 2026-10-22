"""Resolve injected local credentials without inventing a development password."""

from collections.abc import Mapping
from urllib.parse import quote

from ..errors import ToolingError


def require_database_url(url: str | None) -> str:
    """Require a configured connection only when an operation needs a database."""
    if url is None:
        raise ToolingError(
            "Copy .env.template to .env and set REVAER_LOCAL_DB_PASSWORD, "
            "or supply an explicit database URL"
        )
    return url


def local_database_url(environment: Mapping[str, str], database: str) -> str | None:
    """Preserve local endpoint overrides and encode credentials without trimming."""
    user = environment.get("REVAER_LOCAL_DB_USER") or "revaer"
    password = environment.get("REVAER_LOCAL_DB_PASSWORD")
    host = environment.get("REVAER_LOCAL_DB_HOST") or "localhost"
    try:
        port = int(environment.get("REVAER_LOCAL_DB_PORT") or "5432")
    except ValueError as error:
        raise ToolingError("REVAER_LOCAL_DB_PORT must be an integer") from error
    if not 1 <= port <= 65535:
        raise ToolingError("REVAER_LOCAL_DB_PORT must be between 1 and 65535")
    if not password:
        return None
    if ":" in host and not host.startswith("["):
        host = f"[{host}]"
    return (
        f"postgres://{quote(user, safe='')}:{quote(password, safe='')}"
        f"@{host}:{port}/{quote(database, safe='')}"
    )
