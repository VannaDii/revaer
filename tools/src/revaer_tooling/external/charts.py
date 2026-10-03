"""Typed Helm, GPG and OCI metadata operations."""

from dataclasses import dataclass, field
from pathlib import Path

from ..errors import ToolingError
from ..process import Completed
from .base import ExternalTool


@dataclass(frozen=True)
class PackageArgs:
    chart: Path
    destination: Path
    version: str
    app_version: str
    signing_key: str | None = None
    keyring: Path | None = None


@dataclass(frozen=True)
class RegistryCredentials:
    host: str
    username: str
    password: str = field(repr=False)
    ca_file: Path | None = None


@dataclass(frozen=True)
class SignatureArgs:
    home: Path
    public_key: Path
    fingerprint: str
    archive: Path
    signature: Path


class Helm(ExternalTool):
    version_args = ("version", "--short")

    def lint(self, chart: Path, values: Path) -> Completed:
        return self._invoke(("lint", str(chart), "--strict", "--values", str(values)))

    def package(self, args: PackageArgs) -> Completed:
        command = [
            "package",
            str(args.chart),
            "--destination",
            str(args.destination),
            "--version",
            args.version,
            "--app-version",
            args.app_version,
        ]
        if args.signing_key is not None and args.keyring is not None:
            command.extend(("--sign", "--key", args.signing_key, "--keyring", str(args.keyring)))
        return self._invoke(tuple(command))

    def verify_chart(self, chart: Path, keyring: Path) -> Completed:
        return self._invoke(("verify", str(chart), "--keyring", str(keyring)))

    def show_chart(self, chart: Path) -> Completed:
        return self._invoke(("show", "chart", str(chart)), capture=True)

    def login(self, credentials: RegistryCredentials, configuration: Path) -> Completed:
        return self._invoke(
            (
                "registry",
                "login",
                credentials.host,
                "--username",
                credentials.username,
                "--password-stdin",
                "--registry-config",
                str(configuration),
                *(("--ca-file", str(credentials.ca_file)) if credentials.ca_file else ()),
            ),
            input_text=credentials.password + "\n",
        )

    def push(
        self, chart: Path, repository: str, configuration: Path, ca_file: Path | None = None
    ) -> Completed:
        return self._invoke(
            (
                "push",
                str(chart),
                repository,
                "--registry-config",
                str(configuration),
                *(("--ca-file", str(ca_file)) if ca_file else ()),
            )
        )


class Oras(ExternalTool):
    version_args = ("version",)

    def login(self, credentials: RegistryCredentials, configuration: Path) -> Completed:
        return self._invoke(
            (
                "login",
                # ORAS otherwise auto-selects plaintext for localhost, even
                # with a CA file. Explicit false keeps every registry on TLS.
                "--plain-http=false",
                credentials.host,
                "--username",
                credentials.username,
                "--password-stdin",
                "--registry-config",
                str(configuration),
                *(("--ca-file", str(credentials.ca_file)) if credentials.ca_file else ()),
            ),
            input_text=credentials.password + "\n",
        )

    def push_metadata(
        self, ref: str, metadata: Path, configuration: Path, ca_file: Path | None = None
    ) -> Completed:
        return self._invoke(
            (
                "push",
                "--plain-http=false",
                ref,
                "--registry-config",
                str(configuration),
                *(("--ca-file", str(ca_file)) if ca_file else ()),
                "--config",
                "/dev/null:application/vnd.cncf.artifacthub.config.v1+yaml",
                f"{metadata.name}:application/vnd.cncf.artifacthub.repository-metadata.layer.v1.yaml",
            ),
            cwd=metadata.parent,
        )


class Gpg(ExternalTool):
    def verify_signature(self, args: SignatureArgs) -> None:
        """Verify the primary signing identity in a caller-owned, isolated home."""
        common: tuple[str, ...] = ("--homedir", str(args.home), "--batch", "--no-autostart")
        self._invoke((*common, "--import", str(args.public_key)), capture=True)
        identities = self._invoke(
            (*common, "--with-colons", "--fingerprint", args.fingerprint), capture=True
        ).stdout
        fingerprints = {
            fields[9]
            for line in identities.splitlines()
            if len(fields := line.split(":")) > 9 and fields[0] == "fpr"
        }
        if args.fingerprint not in fingerprints:
            raise ToolingError("Imported public key does not match the pinned fingerprint")
        # The committed fingerprint is this operation's trust anchor. Tell GPG
        # that explicitly in the isolated home; this never changes user trust.
        common = (*common, "--trusted-key", args.fingerprint)
        result = self._invoke(
            (*common, "--status-fd", "1", "--verify", str(args.signature), str(args.archive)),
            capture=True,
        )
        if not any(
            fields[:2] == ["[GNUPG:]", "VALIDSIG"] and fields[-1] == args.fingerprint
            for line in result.stdout.splitlines()
            if (fields := line.split())
        ):
            raise ToolingError("Archive signature is not valid for the pinned primary key")

    def import_key(self, home: Path, key: str) -> Completed:
        return self._invoke(
            ("--homedir", str(home), "--batch", "--yes", "--import"), input_text=key, capture=True
        )

    def export(self, home: Path, destination: Path, secret: bool = False) -> Completed:
        return self._invoke(
            (
                "--homedir",
                str(home),
                "--batch",
                "--yes",
                "--output",
                str(destination),
                "--export-secret-keys" if secret else "--export",
            )
        )

    def identities(self, home: Path) -> str:
        return self._invoke(
            (
                "--homedir",
                str(home),
                "--batch",
                "--list-secret-keys",
                "--with-colons",
                "--fingerprint",
            ),
            capture=True,
        ).stdout


class GpgConf(ExternalTool):
    def stop_agent(self, home: Path) -> Completed:
        """Stop only the agent associated with the temporary signing home."""
        return self._invoke(("--homedir", str(home), "--kill", "gpg-agent"))
