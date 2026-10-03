"""Install SonarSource's official archive with the media stack's trust checks.

uv owns Python. This native scanner has its own upstream ZIP distribution; the
reviewed version, archive digests, and signing fingerprint live in versions.toml.
Reinstallation always verifies the cached archive and recreates its extracted
tree, so a modified cache executable is never accepted by version text alone.
"""

import hashlib
import re
import stat
import tempfile
import tomllib
import zipfile
from pathlib import Path, PurePosixPath

from ..context import Context, HostIdentity
from ..errors import ToolingError
from ..external.charts import SignatureArgs
from ..external.http import DownloadArgs
from ..external.sonar import SonarScanner
from .settings import SonarSettings

FLAVORS = {
    ("linux", "x86_64"): "linux-x64",
    ("linux", "aarch64"): "linux-aarch64",
    ("linux", "arm64"): "linux-aarch64",
    ("darwin", "x86_64"): "macosx-x64",
    ("darwin", "arm64"): "macosx-aarch64",
}


def scanner_command(root: Path, settings: SonarSettings, host: HostIdentity) -> str:
    """Bootstrap discovery prefers this checkout's installed native scanner pin."""
    manifest = root / "tools/versions.toml"
    pins = (
        tomllib.loads(manifest.read_text(encoding="utf-8")).get("sonar", {})
        if manifest.is_file()
        else {}
    )
    version = pins.get("version")
    flavor = settings.flavor_override or FLAVORS.get((host.system, host.machine))
    if (
        isinstance(version, str)
        and re.fullmatch(r"\d+\.\d+\.\d+\.\d+", version)
        and flavor in FLAVORS.values()
    ):
        base = settings.install_root or host.home / ".local/revaer/sonar-scanner"
        candidate = (
            base / version / str(flavor) / f"sonar-scanner-{version}-{flavor}/bin/sonar-scanner"
        )
        if candidate.is_file():
            return str(candidate)
    return "sonar-scanner"


def extract_archive(archive: Path, destination: Path, directory: str) -> Path:
    """Extract only regular entries inside the verified archive's named root."""
    try:
        with zipfile.ZipFile(archive) as source:
            entries = source.infolist()
            if not entries or sum(entry.file_size for entry in entries) > 1024**3:
                raise ToolingError("Scanner archive is empty or exceeds its expanded byte limit")
            names: set[str] = set()
            for entry in entries:
                path = PurePosixPath(entry.filename)
                mode = entry.external_attr >> 16
                kind = stat.S_IFMT(mode)
                if (
                    not path.parts
                    or path.parts[0] != directory
                    or path.is_absolute()
                    or ".." in path.parts
                    or "\\" in entry.filename
                    or path.as_posix() != entry.filename.rstrip("/")
                    or path.as_posix() in names
                    or kind not in (0, stat.S_IFREG, stat.S_IFDIR)
                    or (kind == stat.S_IFDIR and not entry.is_dir())
                ):
                    raise ToolingError("Scanner archive contains an unsafe or duplicate entry")
                names.add(path.as_posix())
                target = destination / path
                if entry.is_dir():
                    target.mkdir(parents=True, exist_ok=True)
                else:
                    target.parent.mkdir(parents=True, exist_ok=True)
                    with source.open(entry) as incoming, target.open("xb") as outgoing:
                        while chunk := incoming.read(128 * 1024):
                            outgoing.write(chunk)
                    target.chmod(mode & 0o777 or 0o644)
    except zipfile.BadZipFile as error:
        raise ToolingError("Scanner archive is not a valid ZIP") from error
    executable = destination / directory / "bin/sonar-scanner"
    if not executable.is_file() or not executable.stat().st_mode & 0o111:
        raise ToolingError("Scanner archive does not contain the expected executable")
    return executable


def install_scanner(context: Context) -> Path:
    settings = context.settings.sonar
    pins = tomllib.loads(context.fs.read(context.root / "tools/versions.toml"))["sonar"]
    version, fingerprint = pins["version"], pins["fingerprint"]
    if not isinstance(version, str) or re.fullmatch(r"\d+\.\d+\.\d+\.\d+", version) is None:
        raise ToolingError("Sonar scanner pin must be an exact four-part version")
    if settings.version_override and settings.version_override != version:
        raise ToolingError("SONAR_SCANNER_VERSION must match tools/versions.toml")
    flavor = settings.flavor_override or FLAVORS.get((context.host.system, context.host.machine))
    if flavor is None or flavor not in FLAVORS.values():
        raise ToolingError("Unsupported Sonar scanner platform")
    checksum = pins["archive_sha256"].get(flavor)
    if not isinstance(checksum, str) or re.fullmatch(r"[a-f0-9]{64}", checksum) is None:
        raise ToolingError("Sonar scanner archive needs a pinned SHA-256")
    if not isinstance(fingerprint, str) or re.fullmatch(r"[A-F0-9]{40}", fingerprint) is None:
        raise ToolingError("Sonar scanner needs a pinned primary signing fingerprint")
    key = context.root / "config/sonarsource-public-key.asc"
    if key.is_symlink() or not key.is_file() or key.stat().st_size == 0:
        raise ToolingError("Committed SonarSource public key is missing or empty")
    base = settings.install_root or context.host.home / ".local/revaer/sonar-scanner"
    if not base.is_absolute() or base.is_symlink():
        raise ToolingError("Sonar scanner installation root must be an absolute directory")
    root = base / version / flavor
    if root.is_symlink() or root.resolve() != base.resolve() / version / flavor:
        raise ToolingError("Sonar scanner installation directory must not follow symlinks")
    context.fs.mkdir(root)
    directory = f"sonar-scanner-{version}-{flavor}"
    archive = root / f"sonar-scanner-cli-{version}-{flavor}.zip"
    signature = archive.with_suffix(".zip.asc")
    with context.fs.lock(root / ".install.lock"):
        for path, limit in ((archive, 512 * 1024**2), (signature, 1024**2)):
            if path.is_symlink():
                raise ToolingError("Sonar scanner cache files must not be symlinks")
            if not path.exists() or path.stat().st_size == 0:
                context.tools.http.download(
                    DownloadArgs(settings.binaries_url.rstrip("/") + "/" + path.name, path, limit)
                )
        with archive.open("rb") as stream:
            if hashlib.file_digest(stream, "sha256").hexdigest() != checksum:
                raise ToolingError(
                    "Sonar scanner archive SHA-256 mismatch; cached evidence retained"
                )
        # Public-key verification starts no agent. A short private home also
        # avoids GnuPG's Unix socket path limit on macOS runner temp paths.
        with tempfile.TemporaryDirectory(prefix="rv-sonar-key-", dir="/tmp") as home:
            context.tools.gpg.verify_signature(
                SignatureArgs(Path(home), key, fingerprint, archive, signature)
            )
        with tempfile.TemporaryDirectory(prefix=".extract-", dir=root) as temporary:
            executable = extract_archive(archive, Path(temporary), directory)
            template = context.tools.sonar_scanner
            SonarScanner(
                str(executable), template.runner, context.root, template.environment
            ).verify(version)
            context.fs.remove_owned(root / directory, root)
            (Path(temporary) / directory).replace(root / directory)
    return root / directory / "bin"
