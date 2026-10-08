"""Source integrity, bounded native HTTPS transfers, and manifest rejection."""

import base64
import json
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.downloads import Downloads, download_server
from fixtures.media import DATA, media_context
from revaer_tooling.context import Context
from revaer_tooling.errors import CommandError, ToolingError
from revaer_tooling.external.curl import Curl, DownloadArgs
from revaer_tooling.media.acquire import acquire
from revaer_tooling.media.catalog import load_catalog
from revaer_tooling.media.settings import load_fixture_settings
from revaer_tooling.process import Completed

__all__ = ["media_context"]


class SourceCurl(Curl):
    """Inject transfer bytes/status while keeping the real validation path."""

    def __init__(self, original: Curl, payloads: tuple[bytes | int, ...]) -> None:
        super().__init__(original.name, original.runner, original.root, original.environment)
        self.payloads = iter(payloads)
        self.calls: list[DownloadArgs] = []

    def download(self, args: DownloadArgs) -> Completed:
        self.calls.append(args)
        value = next(self.payloads)
        if isinstance(value, int):
            raise CommandError("simulated transfer status", value)
        args.output.write_bytes(value)
        return Completed(0, "")


def test_both_reviewed_catalogs_validate_without_fixed_counts(media_context: Context) -> None:
    root = Path(__file__).parents[2]
    settings = load_fixture_settings({})
    current = load_catalog(root, settings)
    media = load_catalog(
        DATA.parent.parent.parent.parent,
        replace(settings, lock=DATA / "lock.json", manifest=DATA / "manifest.json"),
    )
    assert (len(current.sources), len(current.fixtures)) == (22, 30)
    assert (len(media.sources), len(media.fixtures)) == (22, 30)
    assert sum(item.exact_diagnostic for item in media.fixtures) == 1


@pytest.mark.parametrize(
    "field,value",
    (
        ("id", "Invalid"),
        ("path", "../outside"),
        ("path", "test-fixtures/derived/file.mp4"),
        ("encoding", "unknown"),
        ("sha256", "f" * 63),
        ("sha256", "A" * 64),
        ("minimumBytes", True),
        ("minimumBytes", 0),
        ("maximumBytes", -1),
        ("maximumBytes", 1),
        ("urls", []),
        ("urls", ["http://fixtures.invalid/source"]),
        ("urls", ["https://user:secret@fixtures.invalid/source"]),
        ("urls", ["https://fixtures.invalid/source#fragment"]),
        ("urls", ["https://[invalid/source"]),
        ("urls", ["https://fixtures.invalid:invalid/source"]),
        ("urls", ["https://fixtures.invalid:0/source"]),
    ),
)
def test_invalid_locked_source_fails(media_context: Context, field: str, value: object) -> None:
    path = media_context.root / "test-fixtures/lock.json"
    data = json.loads(path.read_text())
    data["sources"][0][field] = value
    path.write_text(json.dumps(data))
    with pytest.raises(ToolingError):
        load_catalog(media_context.root, media_context.settings.fixtures)


@pytest.mark.parametrize(
    "field,value",
    (
        ("id", "INVALID"),
        ("path", "/outside"),
        ("generated", True),
        ("shouldGenerate", True),
        ("shouldDownload", False),
        ("allowProbeDiagnostics", "yes"),
        ("allowProbeDiagnostics", True),
        ("probeDiagnosticContract", "adr578-f1"),
        ("probeDiagnosticContract", None),
        ("probeDiagnosticContract", True),
    ),
)
def test_invalid_manifest_fails(media_context: Context, field: str, value: object) -> None:
    path = media_context.root / "test-fixtures/manifest.json"
    data = json.loads(path.read_text())
    data["fixtures"][0][field] = value
    path.write_text(json.dumps(data))
    with pytest.raises(ToolingError):
        load_catalog(media_context.root, media_context.settings.fixtures)


@pytest.mark.parametrize(
    "document_name,key", (("lock.json", "sources"), ("manifest.json", "fixtures"))
)
@pytest.mark.parametrize(
    "change", ("empty", "duplicate-id", "duplicate-path", "version", "malformed")
)
def test_inventory_shape_failures(
    media_context: Context, document_name: str, key: str, change: str
) -> None:
    path = media_context.root / "test-fixtures" / document_name
    data = json.loads(path.read_text())
    if change == "empty":
        data[key] = []
    elif change.startswith("duplicate-"):
        field = change.removeprefix("duplicate-")
        data[key][1][field] = data[key][0][field]
    elif change == "version":
        data["schemaVersion"] = True
    path.write_text("invalid JSON" if change == "malformed" else json.dumps(data))
    with pytest.raises(ToolingError):
        load_catalog(media_context.root, media_context.settings.fixtures)


@pytest.mark.parametrize(
    "payloads",
    ((b"wrong", b"fixture-data"), (28, b"fixture-data"), (b"fixture-data" * 2, b"fixture-data")),
)
def test_fallback_rejects_bad_payload_before_publication(
    media_context: Context, payloads: tuple[bytes | int, ...]
) -> None:
    context = media_context
    source = replace(
        load_catalog(context.root, context.settings.fixtures).sources[0],
        urls=("https://fixtures.invalid/first", "https://fixtures.invalid/second"),
    )
    path = context.root / source.path
    path.write_bytes(b"corrupt cache")
    messages: list[str] = []
    curl = SourceCurl(context.tools.curl, payloads)
    acquire(source, context.root, context.settings.fixtures, curl, messages.append)
    assert path.read_bytes() == b"fixture-data"
    assert len(curl.calls) == 2
    assert curl.calls[0].deadline == 120
    assert any("Discarding invalid cached" in item for item in messages)
    assert any("source 1 failed" in item for item in messages)
    assert not tuple(path.parent.glob(".acquire-*"))


def test_cache_reuse_and_failed_forced_refresh_keep_valid_source(media_context: Context) -> None:
    context = media_context
    source = load_catalog(context.root, context.settings.fixtures).sources[0]
    curl = SourceCurl(context.tools.curl, (b"bad",))
    acquire(source, context.root, context.settings.fixtures, curl, context.emit)
    assert not curl.calls
    with pytest.raises(ToolingError, match="Every locked source"):
        acquire(
            source,
            context.root,
            replace(context.settings.fixtures, force_download=True),
            curl,
            context.emit,
        )
    assert (context.root / source.path).read_bytes() == b"fixture-data"


@pytest.mark.parametrize("encoded", (base64.b64encode(b"fixture-data"), b"%%%"))
def test_base64_download_validates_decoded_integrity(
    media_context: Context, encoded: bytes
) -> None:
    context = media_context
    source = replace(
        load_catalog(context.root, context.settings.fixtures).sources[0], encoding="base64"
    )
    path = context.root / source.path
    path.unlink()
    curl = SourceCurl(context.tools.curl, (encoded,))
    if encoded == b"%%%":
        with pytest.raises(ToolingError, match="Every locked source"):
            acquire(source, context.root, context.settings.fixtures, curl, context.emit)
        assert not path.exists()
    else:
        acquire(source, context.root, context.settings.fixtures, curl, context.emit)
        assert path.read_bytes() == b"fixture-data"
    assert curl.calls[0].maximum_bytes == 20


@pytest.mark.parametrize("location", ("file", "parent", "hardlink"))
def test_cache_links_are_rejected_before_transfer(media_context: Context, location: str) -> None:
    context = media_context
    source = load_catalog(context.root, context.settings.fixtures).sources[0]
    path = context.root / source.path
    foreign = context.root.parent / "foreign"
    foreign.mkdir()
    if location == "parent":
        path.unlink()
        path.parent.rmdir()
        path.parent.symlink_to(foreign)
    elif location == "file":
        path.unlink()
        path.symlink_to(foreign / "payload")
    else:
        (foreign / "linked").hardlink_to(path)
    curl = SourceCurl(context.tools.curl, ())
    with pytest.raises(ToolingError):
        acquire(source, context.root, context.settings.fixtures, curl, context.emit)
    assert not curl.calls


def test_native_curl_tls_redirect_and_size_failures(media_context: Context) -> None:
    context = media_context
    state = Downloads(
        {
            "/good": (200, b"fixture-data", {}),
            "/redirect": (302, b"", {"Location": "/good"}),
            "/downgrade": (302, b"", {"Location": "http://localhost/unsafe"}),
        }
    )
    tls_root = context.root / "https"
    with download_server(tls_root, state) as (url, _):
        curl = Curl(
            "curl",
            context.tools.curl.runner,
            context.root,
            {**context.tools.curl.environment, "CURL_CA_BUNDLE": str(tls_root / "tls.crt")},
        )
        output = context.root / "download"
        curl.download(DownloadArgs(url + "/redirect", output, 12, 1, 2))
        assert output.read_bytes() == b"fixture-data"
        for name, maximum in (("good", 3), ("downgrade", 12), ("missing", 12)):
            with pytest.raises(CommandError):
                curl.download(DownloadArgs(url + "/" + name, context.root / name, maximum, 1, 2))
        assert state.requests == ["/redirect", "/good", "/good", "/downgrade", "/missing"]


@pytest.mark.parametrize(
    "name,value",
    (
        ("REVAER_FIXTURE_FORCE_DOWNLOAD", "yes"),
        ("REVAER_FIXTURE_FORCE_GENERATE", "2"),
        ("REVAER_FIXTURE_DEADLINE_SECONDS", "0"),
        ("REVAER_FIXTURE_CONNECT_TIMEOUT_SECONDS", "bad"),
    ),
)
def test_invalid_settings_fail(name: str, value: str) -> None:
    with pytest.raises(ToolingError):
        load_fixture_settings({name: value})
