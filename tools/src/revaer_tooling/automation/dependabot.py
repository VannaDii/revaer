"""Dependabot ownership and asynchronous refresh decisions from provider records."""

import fnmatch
import re
from datetime import datetime, timedelta

from ..errors import ToolingError
from ..json_data import JsonObject, object_value, string_value

BOT = "dependabot[bot]"
REFRESH_MARKER = "<!-- revaer-dependency-refresh:"
REFRESH_WAIT = timedelta(hours=1)


def commit_sha(value: object) -> str:
    if not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{40}", value) is None:
        raise ToolingError("Dependabot requires a complete commit SHA")
    return value


def timestamp(value: object) -> datetime:
    if not isinstance(value, str):
        raise ToolingError("Dependabot requires a provider timestamp")
    try:
        result = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError as error:
        raise ToolingError("Dependabot returned an invalid timestamp") from error
    if result.tzinfo is None:
        raise ToolingError("Dependabot timestamp requires a timezone")
    return result


def bot_owned(commits: tuple[JsonObject, ...], count: object, head: str) -> bool:
    # GitHub caps this endpoint at 250 commits. An incomplete or ambiguous list
    # cannot authorize overwriting a branch, even when its PR author is a bot.
    if type(count) is not int or count < 1:
        raise ToolingError("Dependabot returned an invalid commit count")
    if len(commits) != count or commit_sha(commits[-1].get("sha")) != head:
        return False
    return all(
        object_value(item.get("author")).get("login") == BOT
        and object_value(item.get("author")).get("type") == "Bot"
        and object_value(object_value(item.get("commit")).get("verification")).get("verified")
        is True
        for item in commits
    )


def reviews_state(reviews: tuple[JsonObject, ...], login: str, head: str) -> tuple[bool, bool]:
    latest: dict[str, JsonObject] = {}
    for review in reviews:
        if review.get("state") not in ("APPROVED", "CHANGES_REQUESTED", "DISMISSED"):
            continue
        user = object_value(review.get("user"))
        latest[string_value(user.get("login"))] = review
    approved = latest.get(login)
    return (
        approved is not None
        and object_value(approved.get("user")).get("type") == "Bot"
        and approved.get("state") == "APPROVED"
        and approved.get("commit_id") == head,
        any(
            item.get("state") == "CHANGES_REQUESTED"
            for actor, item in latest.items()
            if actor != login
        ),
    )


def refresh_action(
    comments: tuple[JsonObject, ...], head: str, head_time: datetime, now: datetime
) -> str:
    requests: list[tuple[datetime, str]] = []
    for comment in comments:
        body = string_value(comment.get("body"))
        if comment.get("author_association") not in ("OWNER", "MEMBER", "COLLABORATOR"):
            continue
        command = body.splitlines()[0].strip()
        if command not in ("@dependabot rebase", "@dependabot recreate"):
            continue
        created = timestamp(comment.get("created_at"))
        # Our marker survives a recreated commit's backdated author timestamp.
        # Unmarked maintainer commands also count while this head is unchanged.
        marker = REFRESH_MARKER + head + ":"
        if (REFRESH_MARKER in body and marker in body) or (
            REFRESH_MARKER not in body and created >= head_time
        ):
            requests.append((created, command))
    if not requests:
        return "rebase"
    created, command = max(requests)
    if now < created + REFRESH_WAIT:
        return "wait"
    if command == "@dependabot recreate":
        raise ToolingError("Dependabot recreation made no progress; maintainer action required")
    return "recreate"


def dependency_files(files: tuple[JsonObject, ...], count: object) -> bool:
    if type(count) is not int or count < 1:
        raise ToolingError("Dependabot returned an invalid changed-file count")
    patterns = (
        "Cargo.toml",
        "Cargo.lock",
        "crates/*/Cargo.toml",
        "pyproject.toml",
        "uv.lock",
        "tools/launcher/pyproject.toml",
        "tools/launcher/uv.lock",
        "Dockerfile",
        "rust-toolchain.toml",
        ".github/workflows/*.yml",
        ".github/workflows/*.yaml",
        ".github/actions/*/action.yml",
        ".github/actions/*/action.yaml",
        "release/package.json",
        "release/package-lock.json",
        "tests/package.json",
        "tests/package-lock.json",
    )
    return len(files) == count and all(
        item.get("status") in ("modified", "added")
        and any(
            fnmatch.fnmatchcase(string_value(item.get("filename")), pattern) for pattern in patterns
        )
        for item in files
    )
