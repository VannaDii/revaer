"""Owned HTTP upstream for real manual-search application scenarios."""

import base64
import hmac
import secrets
import threading
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Literal
from urllib.parse import parse_qs, urlsplit
from xml.sax.saxutils import escape


@dataclass
class TorznabUpstream:
    api_key: str = field(default_factory=lambda: secrets.token_urlsafe(32), repr=False)
    proxy_password: str = field(default_factory=lambda: secrets.token_urlsafe(32), repr=False)
    require_proxy_auth: bool = False
    proxy_authenticated: list[bool] = field(default_factory=list)
    infohash: str = field(default_factory=lambda: secrets.token_hex(20))
    scenario: Literal[
        "matching", "empty", "unauthorized", "malformed", "oversized", "duplicate"
    ] = "matching"
    private_download: bool = False
    endpoint: str = ""
    requests: list[tuple[str, bool]] = field(default_factory=list)
    proxied_requests: list[bool] = field(default_factory=list)

    @property
    def title(self) -> str:
        return "Live Torznab release " + self.infohash[:12]

    @property
    def details_url(self) -> str:
        return "https://tracker.example.test/releases/" + self.infohash

    @property
    def magnet(self) -> str:
        return "magnet:?xt=urn:btih:" + self.infohash

    @property
    def download_url(self) -> str:
        return self.endpoint + "?apikey=" + self.api_key + "&id=" + self.infohash

    def feed(self) -> bytes:
        feed = f"""<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
<channel><title>Controlled Torznab upstream</title>
<description>Live manual search</description><link>{escape(self.endpoint)}</link>
<torznab:response offset="0" total="1"/>
<item><title>{escape(self.title)}</title>
<guid isPermaLink="false">{self.infohash}</guid><link>{escape(self.magnet)}</link>
<comments>{self.details_url}</comments><size>1048576</size>
<pubDate>Sat, 24 Oct 2026 00:00:00 GMT</pubDate>
<enclosure url="{escape(self.magnet)}" length="1048576" type="application/x-bittorrent"/>
<torznab:attr name="infohash" value="{self.infohash}"/>
<torznab:attr name="magneturl" value="{escape(self.magnet)}"/>
<torznab:attr name="size" value="1048576"/>
<torznab:attr name="category" value="2000"/>
<torznab:attr name="seeders" value="17"/>
<torznab:attr name="peers" value="19"/>
</item></channel></rss>""".encode()
        if self.private_download:
            feed = feed.replace(escape(self.magnet).encode(), escape(self.download_url).encode())
            start = feed.index(b'<torznab:attr name="magneturl"')
            end = feed.index(b"/>", start) + 2
            feed = feed[:start] + feed[end:]
        prefix, _, remainder = feed.partition(b"<item>")
        item, _, suffix = remainder.partition(b"</item>")
        if self.scenario == "empty":
            return prefix.replace(b'total="1"', b'total="0"') + suffix
        if self.scenario == "duplicate":
            return (
                prefix.replace(b'total="1"', b'total="2"')
                + (b"<item>" + item + b"</item>") * 2
                + suffix
            )
        if self.scenario == "malformed":
            return b"<rss>"
        if self.scenario == "oversized":
            return feed.replace(self.details_url.encode(), (self.details_url + "x" * 2048).encode())
        return feed


@contextmanager
def torznab_upstream() -> Iterator[TorznabUpstream]:
    upstream = TorznabUpstream()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            parsed = urlsplit(self.path)
            upstream.proxied_requests.append(bool(parsed.scheme))
            query = parse_qs(parsed.query)
            authenticated = hmac.compare_digest(query.get("apikey", [""])[0], upstream.api_key)
            upstream.requests.append((query.get("q", [""])[0], authenticated))
            expected_proxy_auth = (
                "Basic "
                + base64.b64encode(("search-proxy:" + upstream.proxy_password).encode()).decode()
            )
            proxy_authenticated = not upstream.require_proxy_auth or hmac.compare_digest(
                self.headers.get("Proxy-Authorization", ""), expected_proxy_auth
            )
            upstream.proxy_authenticated.append(proxy_authenticated)
            allowed = authenticated and proxy_authenticated and upstream.scenario != "unauthorized"
            body = upstream.feed() if allowed else b"Unauthorized"
            if allowed:
                status = 200
            elif not proxy_authenticated:
                status = 407
            else:
                status = 401
            self.send_response(status)
            self.send_header("Content-Type", "application/rss+xml")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format: str, *args: object) -> None:
            # HTTP request URLs can contain the upstream credential.
            return

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    upstream.endpoint = f"http://127.0.0.1:{server.server_port}/api"
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield upstream
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=1)
        if thread.is_alive():
            raise RuntimeError("Owned Torznab upstream did not stop")
