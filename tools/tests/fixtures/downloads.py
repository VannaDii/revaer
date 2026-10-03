"""Disposable HTTPS downloads with client-scoped certificate trust."""

import ssl
import subprocess
import threading
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.request import HTTPSHandler, build_opener

from revaer_tooling.external.http import Http, VisibleRedirects


@dataclass
class Downloads:
    routes: dict[str, tuple[int, bytes, dict[str, str]]]
    requests: list[str] = field(default_factory=list)


@contextmanager
def download_server(root: Path, state: Downloads) -> Iterator[tuple[str, Http]]:
    root.mkdir(mode=0o700)
    certificate, private = root / "tls.crt", root / "tls.key"
    subprocess.run(
        [
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
        ],
        check=True,
        capture_output=True,
        timeout=30,
    )

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            state.requests.append(self.path)
            status, body, headers = state.routes.get(self.path, (404, b"", {}))
            self.send_response(status)
            for key, value in headers.items():
                self.send_header(key, value)
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format: str, *args: object) -> None:
            """Record requests in state, without an unrelated stderr stream."""

    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(certificate, private)
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.socket = tls.wrap_socket(server.socket, server_side=True)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    trust = ssl.create_default_context(cafile=certificate)
    client = Http(build_opener(VisibleRedirects(), HTTPSHandler(context=trust)))
    try:
        yield f"https://localhost:{server.server_port}", client
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)
        if worker.is_alive():
            raise RuntimeError("Download fixture server did not stop")
