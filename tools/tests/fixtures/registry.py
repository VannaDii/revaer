"""A real, loopback-only Distribution registry with TLS and password authentication.

Every resource belongs to this context manager. Certificate trust is passed only
to the client invocation; no machine trust store or Docker daemon policy changes.
"""

import base64
import json
import secrets
import ssl
import subprocess
import time
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from urllib.error import URLError
from urllib.request import HTTPSHandler, Request, build_opener

from revaer_tooling.external.charts import RegistryCredentials
from revaer_tooling.external.http import VisibleRedirects

# Official registry:3 multi-platform digest resolved on 2026-09-15.
REGISTRY_IMAGE = (
    "registry:3@sha256:1be55279f18a2fe1a74edf2664cac61c1bea305b7b4642dab412e7affdcb3e33"
)


def execute(*arguments: str, input_text: str | None = None) -> str:
    return subprocess.run(
        arguments, input=input_text, capture_output=True, text=True, check=True, timeout=60
    ).stdout.strip()


@dataclass(frozen=True)
class LocalRegistry:
    credentials: RegistryCredentials

    def fetch(self, path: str) -> bytes:
        credentials = self.credentials
        authorization = base64.b64encode(
            f"{credentials.username}:{credentials.password}".encode()
        ).decode()
        trust = ssl.create_default_context(cafile=credentials.ca_file)
        client = build_opener(VisibleRedirects(), HTTPSHandler(context=trust))
        request = Request(
            f"https://{credentials.host}{path}",
            headers={
                "Authorization": "Basic " + authorization,
                "Accept": "application/vnd.oci.image.manifest.v1+json",
            },
        )
        with client.open(request, timeout=5) as response:
            return bytes(response.read())


@contextmanager
def registry(root: Path) -> Iterator[LocalRegistry]:
    root.mkdir(mode=0o700)
    certificate, private = root / "tls.crt", root / "tls.key"
    execute(
        "openssl",
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-noenc",
        "-days",
        "1",
        "-subj",
        "/CN=localhost",
        "-addext",
        "subjectAltName=DNS:localhost,IP:127.0.0.1",
        "-keyout",
        str(private),
        "-out",
        str(certificate),
    )
    password = secrets.token_hex(24)
    authentication = root / "htpasswd"
    authentication.write_text(
        execute("htpasswd", "-niB", "rv-tests", input_text=password + "\n") + "\n"
    )
    authentication.chmod(0o600)
    identifier = execute(
        "docker",
        "create",
        "--read-only",
        "--publish",
        "127.0.0.1::5000",
        "--tmpfs",
        "/var/lib/registry:rw,size=64m,mode=1777",
        "--tmpfs",
        "/tmp:rw,size=16m,mode=1777",
        "--volume",
        f"{root}:/run/rv-tests:ro",
        "--env",
        "OTEL_TRACES_EXPORTER=none",
        "--env",
        "REGISTRY_HTTP_TLS_CERTIFICATE=/run/rv-tests/tls.crt",
        "--env",
        "REGISTRY_HTTP_TLS_KEY=/run/rv-tests/tls.key",
        "--env",
        "REGISTRY_AUTH=htpasswd",
        "--env",
        "REGISTRY_AUTH_HTPASSWD_REALM=rv-tests",
        "--env",
        "REGISTRY_AUTH_HTPASSWD_PATH=/run/rv-tests/htpasswd",
        REGISTRY_IMAGE,
    )
    try:
        execute("docker", "start", identifier)
        bindings = json.loads(execute("docker", "inspect", identifier))[0]["NetworkSettings"][
            "Ports"
        ]["5000/tcp"]
        assert len(bindings) == 1 and bindings[0]["HostIp"] == "127.0.0.1"
        service = LocalRegistry(
            RegistryCredentials(
                "localhost:" + bindings[0]["HostPort"], "rv-tests", password, certificate
            )
        )
        deadline = time.monotonic() + 30
        while True:
            try:
                assert json.loads(service.fetch("/v2/")) == {}
                break
            except URLError:
                if time.monotonic() >= deadline:
                    raise
                time.sleep(0.1)
        yield service
    finally:
        try:
            (root / "registry.log").write_text(execute("docker", "logs", identifier))
        finally:
            execute("docker", "rm", "--force", "--volumes", identifier)
