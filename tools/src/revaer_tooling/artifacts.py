"""Bind application release artifacts to the clean commit that produced them.

The manifest is written last. An interrupted preparation can leave files on disk,
but publication rejects them unless every recorded digest still matches.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import TYPE_CHECKING

from .errors import ToolingError

if TYPE_CHECKING:
    from .context import Context

APPLICATION_ARTIFACTS = ("dist/revaer-app", "dist/revaer-app.sha256", "dist/openapi.json")
MANIFEST = "dist/release-artifacts.json"


def digest_file(path: Path) -> str:
    if not path.is_file() or path.stat().st_size == 0:
        raise ToolingError(f"Missing or empty release artifact: {path.name}")
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def record_artifacts(context: Context, source: str) -> None:
    hashes = {name: digest_file(context.root / name) for name in APPLICATION_ARTIFACTS}
    context.fs.write(
        context.root / MANIFEST,
        json.dumps({"sourceCommit": source, "sha256": hashes}, indent=2) + "\n",
    )


def verify_artifacts(context: Context, source: str) -> None:
    try:
        manifest = json.loads(context.fs.read(context.root / MANIFEST))
    except (OSError, ValueError) as error:
        raise ToolingError("Run rv release-artifacts to prepare a release manifest") from error
    if not isinstance(manifest, dict) or manifest.get("sourceCommit") != source:
        raise ToolingError("Release artifacts were not built from the current source commit")
    expected = {name: digest_file(context.root / name) for name in APPLICATION_ARTIFACTS}
    if manifest.get("sha256") != expected:
        raise ToolingError("Release artifacts changed after preparation; rebuild them")
    checksum = context.fs.read(context.root / "dist/revaer-app.sha256")
    if checksum != f"{expected['dist/revaer-app']}  revaer-app\n":
        raise ToolingError("Application release checksum does not match the binary")
