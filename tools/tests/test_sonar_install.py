"""Real TLS, hashes, GPG signatures, and archive extraction for scanner setup.

The small executable only supplies version text. These tests do not run a Sonar
analysis or claim that an analysis scanner was downloaded from SonarSource.
"""

import hashlib
import stat
import subprocess
import sys
import tempfile
import zipfile
from collections.abc import Iterator
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.downloads import Downloads, download_server
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.charts import SignatureArgs
from revaer_tooling.external.http import DownloadArgs
from revaer_tooling.sonar.install import extract_archive, install_scanner, scanner_command

VERSION = "8.1.0.6389"
DIRECTORY = f"sonar-scanner-{VERSION}-linux-x64"
ARCHIVE = f"sonar-scanner-cli-{VERSION}-linux-x64.zip"


def gpg(home: Path, *arguments: str) -> str:
    return subprocess.run(
        ["gpg", "--homedir", str(home), "--batch", *arguments],
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    ).stdout


@pytest.fixture
def installer_context(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> Iterator[tuple[Context, Path, str]]:
    (tmp_path / ".git").mkdir()
    (tmp_path / "tools/src/revaer_tooling").mkdir(parents=True)
    (tmp_path / "tools/src/revaer_tooling/cli.py").touch()
    (tmp_path / "config").mkdir()
    monkeypatch.chdir(tmp_path)
    context = make_context(Options())
    settings = replace(
        context.settings.sonar,
        install_root=tmp_path / "installed",
        flavor_override="linux-x64",
        version_override=None,
    )
    context = replace(context, settings=replace(context.settings, sonar=settings))
    archive = tmp_path / ARCHIVE
    with zipfile.ZipFile(archive, "w") as output:
        info = zipfile.ZipInfo(DIRECTORY + "/bin/sonar-scanner")
        info.external_attr = (stat.S_IFREG | 0o755) << 16
        output.writestr(info, f"#!{sys.executable}\nprint('SonarScanner CLI {VERSION}')\n")
    with tempfile.TemporaryDirectory(prefix="rv-signer-", dir="/tmp") as temporary:
        home = Path(temporary)
        try:
            fingerprints = []
            for name in ("Primary", "Unrelated"):
                identity = f"{name} fixture <{name.lower()}@example.invalid>"
                gpg(
                    home,
                    "--pinentry-mode",
                    "loopback",
                    "--passphrase",
                    "",
                    "--quick-generate-key",
                    identity,
                    "ed25519",
                    "sign",
                    "1d",
                )
                fingerprints.append(
                    next(
                        line.split(":")[9]
                        for line in gpg(
                            home, "--with-colons", "--fingerprint", identity
                        ).splitlines()
                        if line.startswith("fpr:")
                    )
                )
            (tmp_path / "config/sonarsource-public-key.asc").write_text(
                gpg(home, "--armor", "--export")
            )
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            (tmp_path / "tools/versions.toml").write_text(
                f'[sonar]\nversion="{VERSION}"\nfingerprint="{fingerprints[0]}"\n[sonar.archive_sha256]\nlinux-x64="{digest}"\n'
            )
            gpg(home, "--armor", "--local-user", fingerprints[0], "--detach-sign", str(archive))
            gpg(
                home,
                "--armor",
                "--local-user",
                fingerprints[1],
                "--output",
                str(tmp_path / "unrelated.asc"),
                "--detach-sign",
                str(archive),
            )
            yield context, archive, fingerprints[0]
        finally:
            context.tools.gpgconf.stop_agent(home)


def test_verified_install_rechecks_cache_and_repairs_modified_extraction(
    installer_context: tuple[Context, Path, str],
) -> None:
    context, archive, _ = installer_context
    signature = archive.with_suffix(".zip.asc")
    state = Downloads(
        {
            "/" + ARCHIVE: (200, archive.read_bytes(), {}),
            "/" + signature.name: (200, signature.read_bytes(), {}),
        }
    )
    with download_server(context.root / "https", state) as (url, http):
        context = replace(
            context,
            settings=replace(
                context.settings, sonar=replace(context.settings.sonar, binaries_url=url)
            ),
            tools=replace(context.tools, http=http),
        )
        installed = install_scanner(context)
        executable = installed / "sonar-scanner"
        original = executable.read_bytes()
        assert scanner_command(context.root, context.settings.sonar, context.host) == str(
            executable
        )
        executable.write_text("modified cache executable")
        assert install_scanner(context) == installed
        assert executable.read_bytes() == original
        assert state.requests == ["/" + ARCHIVE, "/" + signature.name]
        cached = installed.parent.parent / ARCHIVE
        cached.write_bytes(cached.read_bytes() + b"tampered")
        with pytest.raises(ToolingError, match="SHA-256 mismatch"):
            install_scanner(context)
        assert executable.read_bytes() == original


def test_an_unrelated_valid_signature_cannot_satisfy_the_pinned_primary_key(
    installer_context: tuple[Context, Path, str],
) -> None:
    context, archive, fingerprint = installer_context
    with tempfile.TemporaryDirectory(prefix="rv-verify-", dir="/tmp") as temporary:
        args = SignatureArgs(
            Path(temporary),
            context.root / "config/sonarsource-public-key.asc",
            fingerprint,
            archive,
            context.root / "unrelated.asc",
        )
        with pytest.raises(ToolingError, match="pinned primary key"):
            context.tools.gpg.verify_signature(args)
        args.signature.write_text("not a signature")
        with pytest.raises(ToolingError):
            context.tools.gpg.verify_signature(args)


@pytest.mark.parametrize(
    "field,value",
    (
        ("version_override", "../../escape"),
        ("flavor_override", "../../escape"),
        ("install_root", Path("relative")),
    ),
)
def test_invalid_install_overrides_fail_before_download(
    installer_context: tuple[Context, Path, str], field: str, value: str | Path
) -> None:
    context, _, _ = installer_context
    settings = context.settings.sonar
    if field == "version_override":
        settings = replace(settings, version_override=str(value))
    elif field == "flavor_override":
        settings = replace(settings, flavor_override=str(value))
    else:
        settings = replace(settings, install_root=Path(value))
    with pytest.raises(ToolingError):
        install_scanner(replace(context, settings=replace(context.settings, sonar=settings)))
    assert not (context.root / "installed").exists()


@pytest.mark.parametrize(
    "name,mode",
    (
        ("../escape", stat.S_IFREG),
        ("/absolute", stat.S_IFREG),
        (DIRECTORY + "/../escape", stat.S_IFREG),
        (DIRECTORY + "/bin/link", stat.S_IFLNK),
        (DIRECTORY + "/bin/pipe", stat.S_IFIFO),
    ),
)
def test_archive_paths_and_special_files_cannot_escape_staging(
    tmp_path: Path, name: str, mode: int
) -> None:
    archive = tmp_path / "fixture.zip"
    with zipfile.ZipFile(archive, "w") as output:
        info = zipfile.ZipInfo(name)
        info.external_attr = (mode | 0o755) << 16
        output.writestr(info, b"payload")
    with pytest.raises(ToolingError, match="unsafe"):
        extract_archive(archive, tmp_path / "staged", DIRECTORY)
    assert not (tmp_path / "escape").exists()


def test_https_redirects_and_download_failures_preserve_previous_file(tmp_path: Path) -> None:
    state = Downloads(
        {
            "/file": (200, b"verified bytes", {}),
            "/redirect": (302, b"", {"Location": "/file"}),
            "/loop": (302, b"", {"Location": "/loop"}),
            "/insecure": (302, b"", {"Location": "http://localhost/file"}),
            "/empty": (200, b"", {}),
        }
    )
    destination = tmp_path / "download"
    with download_server(tmp_path / "https", state) as (url, http):
        http.download(DownloadArgs(url + "/redirect", destination, 100))
        assert destination.read_bytes() == b"verified bytes"
        for path, limit in (
            ("/file", 3),
            ("/empty", 100),
            ("/missing", 100),
            ("/insecure", 100),
            ("/loop", 100),
            ("/file", 0),
        ):
            with pytest.raises(ToolingError):
                http.download(DownloadArgs(url + path, destination, limit))
            assert destination.read_bytes() == b"verified bytes"
