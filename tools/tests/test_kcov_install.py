"""Build a tiny CMake fixture through the real pinned-source installer path.

The fixture's C++ executable supplies version text only. It verifies CMake,
cache integrity, TLS downloads and atomic installation, not upstream kcov's
instrumentation. test_bootstrap.py measures the real bootstrap with actual kcov.
"""

import hashlib
import io
import json
import tarfile
from collections.abc import Iterator
from dataclasses import replace
from pathlib import Path

import pytest
from fixtures.downloads import Downloads, download_server
from revaer_tooling.bootstrap.install import extract_source, install_kcov, kcov_command
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError

COMMIT = "a" * 40
DIRECTORY = "kcov-" + COMMIT


def source_archive(*, broken: bool = False) -> bytes:
    buffer = io.BytesIO()
    files = {
        "CMakeLists.txt": (
            "cmake_minimum_required(VERSION 3.12)\nproject(fixture LANGUAGES CXX)\n"
            "add_executable(kcov main.cpp)\ninstall(TARGETS kcov DESTINATION bin)\n"
        ),
        "main.cpp": (
            "intentionally invalid C++\n"
            if broken
            else '#include <iostream>\nint main() { std::cout << "kcov 43\\n"; return 0; }\n'
        ),
    }
    with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
        for name, content in files.items():
            data = content.encode()
            entry = tarfile.TarInfo(DIRECTORY + "/" + name)
            entry.size = len(data)
            archive.addfile(entry, io.BytesIO(data))
    return buffer.getvalue()


def pins(context: Context, archive: bytes) -> None:
    context.fs.write(
        context.root / "tools/versions.toml",
        f'[kcov]\nversion="43"\ncommit="{COMMIT}"\n'
        f'sha256="{hashlib.sha256(archive).hexdigest()}"\n',
    )


@pytest.fixture
def installer(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> Iterator[tuple[Context, Downloads]]:
    (tmp_path / ".git").mkdir()
    package = tmp_path / "tools/src/revaer_tooling"
    package.mkdir(parents=True)
    (package / "cli.py").touch()
    monkeypatch.chdir(tmp_path)
    context = make_context(Options())
    archive = source_archive()
    pins(context, archive)
    state = Downloads({"/" + COMMIT + ".tar.gz": (200, archive, {})})
    with download_server(tmp_path / "https", state) as (url, http):
        yield (
            replace(
                context,
                host=replace(context.host, system="linux"),
                tools=replace(context.tools, http=http),
                settings=replace(
                    context.settings,
                    kcov=replace(
                        context.settings.kcov,
                        install_root=tmp_path / "native tools",
                        archives_url=url,
                    ),
                ),
            ),
            state,
        )


def test_source_build_rechecks_cache_and_repairs_changed_executable(
    installer: tuple[Context, Downloads],
) -> None:
    context, downloads = installer
    executable = install_kcov(context)
    original = executable.read_bytes()
    root = executable.parents[2]
    receipt = json.loads((root / "receipt.json").read_text())
    assert receipt["commit"] == COMMIT
    assert "bin/kcov" in receipt["files"]
    assert kcov_command(context.root, context.settings.kcov, context.host) == str(executable)
    assert (root / "logs/configure.log").stat().st_size > 0
    modified = (root / "logs/build.log").stat().st_mtime_ns
    assert install_kcov(context) == executable
    assert (root / "logs/build.log").stat().st_mtime_ns == modified
    executable.chmod(0o600)
    install_kcov(context)
    assert executable.read_bytes() == original
    assert executable.stat().st_mode & 0o111
    executable.write_bytes(b"modified executable")
    install_kcov(context)
    assert executable.read_bytes() == original
    assert len(downloads.requests) == 1
    (root / "source.tar.gz").write_bytes(b"modified cache")
    with pytest.raises(ToolingError, match="SHA-256"):
        install_kcov(context)
    assert executable.read_bytes() == original


def test_failed_native_build_preserves_the_previous_installation(
    installer: tuple[Context, Downloads],
) -> None:
    context, _ = installer
    executable = install_kcov(context)
    previous = executable.read_bytes()
    archive = source_archive(broken=True)
    pins(context, archive)
    (executable.parents[2] / "source.tar.gz").write_bytes(archive)
    with pytest.raises(ToolingError, match="cmake exited"):
        install_kcov(context)
    assert executable.read_bytes() == previous
    assert "error:" in (executable.parents[2] / "logs/build.log").read_text()


@pytest.mark.parametrize("case", ("traversal", "symlink", "device", "duplicate"))
def test_source_archive_cannot_escape_staging(tmp_path: Path, case: str) -> None:
    path = tmp_path / "source.tar.gz"
    with tarfile.open(path, "w:gz") as archive:
        entry = tarfile.TarInfo(DIRECTORY + ("/../escape" if case == "traversal" else "/item"))
        if case == "symlink":
            entry.type, entry.linkname = tarfile.SYMTYPE, "../../escape"
        elif case == "device":
            entry.type = tarfile.CHRTYPE
        archive.addfile(entry)
        if case == "duplicate":
            archive.addfile(entry)
    with pytest.raises(ToolingError, match=r"unsafe|escapes"):
        extract_source(path, tmp_path / "staging", DIRECTORY)
    assert not (tmp_path / "escape").exists()


def test_source_installer_rejects_an_unsupported_host_before_downloading(
    installer: tuple[Context, Downloads],
) -> None:
    context, downloads = installer
    with pytest.raises(ToolingError, match="Linux CI"):
        install_kcov(replace(context, host=replace(context.host, system="darwin")))
    assert downloads.requests == []
