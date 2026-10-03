"""Validate cached or staged bytes before publishing a locked fixture."""

import base64
import binascii
import tempfile
from collections.abc import Callable
from pathlib import Path

from ..errors import ToolingError
from ..external.curl import Curl, DownloadArgs
from .model import Source, owned_path, regular_file
from .settings import FixtureSettings


def acquire(
    source: Source, root: Path, settings: FixtureSettings, curl: Curl, emit: Callable[[str], None]
) -> None:
    destination = owned_path(root, source.path)
    if destination.exists():
        regular_file(destination)
        try:
            source.verify(destination)
        except ToolingError as error:
            emit(f"Discarding invalid cached fixture {source.id}: {error}")
            destination.unlink()
        else:
            if not settings.force_download:
                emit(f"Verified cached fixture {source.id}")
                return
    destination.parent.mkdir(parents=True, exist_ok=True)
    # Staging on the destination filesystem makes publication atomic. A failed
    # forced refresh retains the previous valid fixture; it still returns failure.
    with tempfile.TemporaryDirectory(prefix=".acquire-", dir=destination.parent) as name:
        staging = Path(name)
        for index, url in enumerate(source.urls):
            payload = staging / f"payload-{index}"
            try:
                curl.download(
                    DownloadArgs(
                        url,
                        payload,
                        source.encoded_maximum,
                        settings.connect_timeout,
                        settings.deadline,
                    )
                )
                regular_file(payload)
                if payload.stat().st_size > source.encoded_maximum:
                    raise ToolingError(f"Fixture {source.id} exceeds its transfer bound")
                if source.encoding == "base64":
                    encoded = b"".join(payload.read_bytes().split())
                    try:
                        data = base64.b64decode(encoded, validate=True)
                    except binascii.Error as error:
                        raise ToolingError(f"Fixture {source.id} has invalid base64") from error
                    decoded = staging / f"decoded-{index}"
                    decoded.write_bytes(data)
                    payload = decoded
                source.verify(payload)
            except ToolingError as error:
                emit(f"Fixture {source.id} source {index + 1} failed: {error}")
                continue
            payload.chmod(0o644)
            payload.replace(destination)
            emit(f"Downloaded and verified fixture {source.id}")
            return
    raise ToolingError(f"Every locked source failed for fixture {source.id}")
