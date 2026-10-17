"""Resolve setup selections before any installer changes the host.

Local setup defaults to the complete tool set. CI can select reviewed Cargo
packages and browser engines explicitly, while their versions still come from
the checkout's manifests. Empty selections mean no additional tools of that kind.
"""

import tomllib
from dataclasses import dataclass
from pathlib import Path

from ..errors import ToolingError
from ..external.packages import AptInstallArgs
from ..external.rust import CargoInstallArgs
from ..filesystem import FileSystem

BASE_PACKAGES = ("libtorrent-rasterbar-dev", "pkg-config", "sccache")
DATABASE_PACKAGES = (*BASE_PACKAGES, "libpq-dev", "postgresql-client")
APT_PROFILES: dict[str, tuple[str, ...]] = {
    "": (),
    "base": BASE_PACKAGES,
    "db": DATABASE_PACKAGES,
    "media": ("ca-certificates", "curl", "ffmpeg"),
    "coverage": (
        *DATABASE_PACKAGES,
        "ffmpeg",
        "clang-19",
        "ca-certificates",
        "curl",
        "build-essential",
        "cmake",
        "binutils-dev",
        "libssl-dev",
        "libcurl4-openssl-dev",
        "libelf-dev",
        "libstdc++-12-dev",
        "zlib1g-dev",
        "libdw-dev",
        "libiberty-dev",
        "gnupg",
        "openssl",
        "apache2-utils",
        "lsof",
    ),
}
BROWSERS = ("chromium", "firefox", "webkit")
# Branded Chrome uses the host's normal installation location. Keep it explicit
# so ordinary setup does not replace a developer's installed browser.
INSTALL_BROWSERS = (*BROWSERS, "chrome")


@dataclass(frozen=True)
class NativeSetup:
    toolchain: str
    components: tuple[str, ...]
    cargo: tuple[CargoInstallArgs, ...]
    browsers: tuple[str, ...]
    wasm: bool
    nightly: str | None


def comma_selection(value: str) -> tuple[str, ...]:
    """Keep an explicit empty selection distinct from the omitted default."""
    return tuple(part.strip() for part in value.split(",")) if value.strip() else ()


def apt_selection(profile: str, packages: tuple[str, ...]) -> tuple[str, ...]:
    if profile not in APT_PROFILES:
        raise ToolingError(f"Unknown apt profile: {profile}")
    selected = packages or APT_PROFILES[profile]
    if selected:
        AptInstallArgs(selected).validate()
    return selected


def native_selection(
    fs: FileSystem,
    root: Path,
    cargo_tools: tuple[str, ...] | None,
    browsers: tuple[str, ...] | None,
) -> NativeSetup:
    toolchain = tomllib.loads(fs.read(root / "rust-toolchain.toml"))["toolchain"]
    pins = tomllib.loads(fs.read(root / "tools/versions.toml"))
    selected = tuple(pins["cargo"]) if cargo_tools is None else cargo_tools
    if len(selected) != len(set(selected)) or any(name not in pins["cargo"] for name in selected):
        raise ToolingError(
            "Cargo setup selection must contain unique names from tools/versions.toml"
        )
    engines = BROWSERS if browsers is None else browsers
    if len(engines) != len(set(engines)) or any(name not in INSTALL_BROWSERS for name in engines):
        raise ToolingError(
            "Browser selection must contain unique chromium, firefox, webkit or chrome names"
        )
    packages = tuple(
        CargoInstallArgs(
            name,
            pins["cargo"][name]["version"],
            tuple(pins["cargo"][name].get("features", [])),
            pins["cargo"][name].get("no_default_features", False),
        )
        for name in selected
    )
    return NativeSetup(
        toolchain["channel"],
        tuple(toolchain["components"]),
        packages,
        engines,
        cargo_tools is None or "trunk" in selected,
        pins["rust"]["udeps_toolchain"]
        if cargo_tools is None or "cargo-udeps" in selected
        else None,
    )
