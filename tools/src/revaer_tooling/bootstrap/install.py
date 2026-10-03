"""Build the existing media-stack kcov pin with its upstream CMake interface.

The source archive is hash verified before extraction. An installation receipt
checks every cached installed file; changed output is rebuilt in a fresh staging
directory. Publication occurs only after CMake and the exact version probe pass.
"""

import hashlib
import json
import re
import tarfile
import tempfile
import tomllib
from pathlib import Path, PurePosixPath

from ..context import Context, HostIdentity
from ..errors import ToolingError
from ..external.bash import Kcov
from ..external.cmake import CmakeInstallArgs
from ..external.http import DownloadArgs
from .settings import KcovSettings


def kcov_command(root: Path, settings: KcovSettings, host: HostIdentity) -> str:
    """CLI discovery prefers the current checkout's installed source revision."""
    manifest = root / "tools/versions.toml"
    pins = tomllib.loads(manifest.read_text()).get("kcov", {}) if manifest.is_file() else {}
    commit = pins.get("commit", "")
    if isinstance(commit, str) and re.fullmatch(r"[a-f0-9]{40}", commit):
        base = settings.install_root or host.home / ".local/revaer/kcov"
        command = base / commit / "installed/bin/kcov"
        if command.is_file():
            return str(command)
    return "kcov"


def extract_source(archive: Path, destination: Path, directory: str) -> Path:
    """Retain upstream's internal test symlink; reject paths escaping its tree."""
    root = destination / directory
    try:
        with tarfile.open(archive, "r:gz") as source:
            entries = source.getmembers()
            if not entries or sum(entry.size for entry in entries) > 64 * 1024**2:
                raise ToolingError("Kcov source archive is empty or exceeds its expanded limit")
            names: set[str] = set()
            for entry in entries:
                path = PurePosixPath(entry.name)
                if (
                    not path.parts
                    or path.parts[0] != directory
                    or path.is_absolute()
                    or ".." in path.parts
                    or "\\" in entry.name
                    or path.as_posix() != entry.name.rstrip("/")
                    or path.as_posix() in names
                    or not (entry.isfile() or entry.isdir() or entry.issym())
                ):
                    raise ToolingError("Kcov source archive contains an unsafe or duplicate entry")
                names.add(path.as_posix())
                if entry.issym() and not (
                    destination / path.parent / entry.linkname
                ).resolve().is_relative_to(root.resolve()):
                    raise ToolingError("Kcov source symlink escapes its source tree")
            # Python's documented data filter also checks resolved link targets
            # during extraction and rejects devices and unsafe filesystem writes.
            source.extractall(destination, members=entries, filter="data")
    except (tarfile.TarError, EOFError) as error:
        raise ToolingError("Kcov source archive is invalid or unsafe") from error
    if not (root / "CMakeLists.txt").is_file():
        raise ToolingError("Kcov source archive has no CMake project")
    return root


def installed_files(directory: Path) -> dict[str, str]:
    """Record permission bits and bytes, so chmod changes also invalidate a cache."""
    files: dict[str, str] = {}
    for path in sorted(directory.rglob("*")):
        if path.is_symlink():
            raise ToolingError("Installed kcov files must not be symlinks")
        if path.is_file():
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
                files[path.relative_to(directory).as_posix()] = (
                    f"{path.stat().st_mode & 0o777:03o}:" + digest
                )
    return files


def install_kcov(context: Context) -> Path:
    if context.host.system != "linux":
        raise ToolingError(
            "The pinned kcov source installer targets Linux CI; "
            "use native kcov 43 (for example Homebrew) for macOS bootstrap coverage"
        )
    settings = context.settings.kcov
    pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))["kcov"]
    commit, checksum, version = pins["commit"], pins["sha256"], pins["version"]
    if (
        not isinstance(commit, str)
        or re.fullmatch(r"[a-f0-9]{40}", commit) is None
        or not isinstance(checksum, str)
        or re.fullmatch(r"[a-f0-9]{64}", checksum) is None
        or version != "43"
    ):
        raise ToolingError("Kcov requires the exact source revision, SHA-256, and version 43")
    base = settings.install_root or context.host.home / ".local/revaer/kcov"
    root = base / commit
    if not base.is_absolute() or base.is_symlink() or root.resolve() != base.resolve() / commit:
        raise ToolingError("Kcov installation must use an absolute directory without symlinks")
    context.fs.mkdir(root)
    archive, target, receipt = root / "source.tar.gz", root / "installed", root / "receipt.json"
    adapter = context.tools.kcov
    executable = target / "bin/kcov"
    with context.fs.lock(root / ".install.lock"):
        if any(path.is_symlink() for path in (archive, target, receipt)):
            raise ToolingError("Kcov installation entries must not be symlinks")
        if not archive.exists():
            context.tools.http.download(
                DownloadArgs(
                    settings.archives_url + "/" + commit + ".tar.gz", archive, 16 * 1024**2
                )
            )
        with archive.open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != checksum:
                raise ToolingError("Kcov source archive SHA-256 mismatch; cached evidence retained")
        identity = {"commit": commit, "sha256": checksum, "files": installed_files(target)}
        if (
            executable.is_file()
            and receipt.is_file()
            and json.loads(context.fs.read(receipt)) == identity
        ):
            Kcov(
                str(executable),
                adapter.runner,
                context.root,
                adapter.environment,
                adapter.python,
                adapter.uv,
            ).verify(version)
            return executable
        with tempfile.TemporaryDirectory(prefix="build-", dir=root) as temporary:
            staging = Path(temporary)
            source = extract_source(archive, staging / "source", "kcov-" + commit)
            prefix = staging / "installed"
            context.tools.cmake.install(
                CmakeInstallArgs(source, staging / "build", prefix, root / "logs")
            )
            Kcov(
                str(prefix / "bin/kcov"),
                adapter.runner,
                context.root,
                adapter.environment,
                adapter.python,
                adapter.uv,
            ).verify(version)
            files = installed_files(prefix)
            previous = root / "previous"
            context.fs.remove_owned(previous, root)
            if target.exists():
                target.rename(previous)
            try:
                prefix.rename(target)
            except OSError:
                if previous.exists():
                    previous.rename(target)
                raise
            context.fs.write(
                receipt,
                json.dumps({"commit": commit, "sha256": checksum, "files": files}) + "\n",
                0o600,
            )
            context.fs.remove_owned(previous, root)
    return executable
