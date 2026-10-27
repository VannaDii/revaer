"""Owned HTTP upstream for real manual-search application scenarios."""

import base64
import hmac
import secrets
import threading
from collections.abc import Iterator
from contextlib import ExitStack, contextmanager, nullcontext
from dataclasses import dataclass, field
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Literal
from urllib.parse import parse_qs, urlsplit
from xml.sax.saxutils import escape

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, string_value

from tests.support.api_assertions import rows

_ITEM_OPEN = b"<item>"
_ITEM_CLOSE = b"</item>"
_SINGLE_TOTAL = b'total="1"'
_INSTANCE_PATH = "/{indexer_instance_public_id}"


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
    response_gate: threading.Event | None = field(default=None, repr=False)
    status_code: int | None = None
    requested: threading.Event = field(default_factory=threading.Event, repr=False)
    delay_seconds: float = 0
    stopping: threading.Event = field(default_factory=threading.Event, repr=False)
    responded: threading.Event = field(default_factory=threading.Event, repr=False)
    release_count: int = 1
    parameters: list[dict[str, list[str]]] = field(default_factory=list)
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
        prefix, _, remainder = feed.partition(_ITEM_OPEN)
        item, _, suffix = remainder.partition(_ITEM_CLOSE)
        if self.scenario == "empty":
            return prefix.replace(_SINGLE_TOTAL, b'total="0"') + suffix
        if self.scenario == "duplicate":
            return (
                prefix.replace(_SINGLE_TOTAL, b'total="2"')
                + (_ITEM_OPEN + item + _ITEM_CLOSE) * 2
                + suffix
            )
        if self.scenario == "malformed":
            return b"<rss>"
        if self.scenario == "oversized":
            return feed.replace(self.details_url.encode(), (self.details_url + "x" * 2048).encode())
        if self.release_count > 1:
            releases = []
            for index in range(self.release_count):
                infohash = f"{(int(self.infohash, 16) + index) % (1 << 160):040x}"
                release = item.replace(self.infohash.encode(), infohash.encode())
                release = release.replace(
                    escape(self.title).encode(), escape(f"{self.title} item {index:03}").encode()
                )
                releases.append(_ITEM_OPEN + release + _ITEM_CLOSE)
            return (
                prefix.replace(_SINGLE_TOTAL, f'total="{self.release_count}"'.encode())
                + b"".join(releases)
                + suffix
            )
        return feed


@contextmanager
def torznab_upstream() -> Iterator[TorznabUpstream]:
    upstream = TorznabUpstream()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            parsed = urlsplit(self.path)
            upstream.proxied_requests.append(bool(parsed.scheme))
            query = parse_qs(parsed.query)
            upstream.parameters.append(
                {
                    key: value
                    for key, value in query.items()
                    if key
                    in {
                        "t",
                        "q",
                        "cat",
                        "imdbid",
                        "tmdbid",
                        "tvdbid",
                        "season",
                        "ep",
                        "offset",
                        "limit",
                    }
                }
            )
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
            if allowed and upstream.status_code is not None:
                status = upstream.status_code
            upstream.requested.set()
            if upstream.response_gate is not None:
                upstream.response_gate.wait()
            if upstream.stopping.wait(upstream.delay_seconds):
                return
            self.send_response(status)
            self.send_header("Content-Type", "application/rss+xml")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            upstream.responded.set()

        def log_message(self, format: str, *args: object) -> None:
            # HTTP request URLs can contain the upstream credential.
            return

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = False
    upstream.endpoint = f"http://127.0.0.1:{server.server_port}/api"
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield upstream
    finally:
        upstream.stopping.set()
        if upstream.response_gate is not None:
            upstream.response_gate.set()
        server.shutdown()
        server.server_close()
        thread.join(timeout=1)
        if thread.is_alive():
            raise RuntimeError("Owned Torznab upstream did not stop")


@contextmanager
def configure_torznab(
    api: ApiClient,
    upstream: TorznabUpstream,
    routing_policy: str | None = None,
    *,
    exclusive: bool = True,
) -> Iterator[str]:
    route = "/v1/indexers/instances"
    created = api.request(
        ApiRequest(
            Method.POST,
            route,
            {
                "indexer_definition_upstream_slug": "torznab",
                "display_name": "Live search " + upstream.infohash[:12],
                "routing_policy_public_id": routing_policy,
            },
        )
    )
    assert created.status == 201, "A Torznab upstream must be configurable through the API"
    identifier = string_value(created.object()["indexer_instance_public_id"])
    path = {"indexer_instance_public_id": identifier}
    secret_id: str | None = None
    try:
        secret = api.request(
            ApiRequest(
                Method.POST,
                "/v1/indexers/secrets",
                {"secret_type": "api_key", "secret_value": upstream.api_key},
            )
        )
        assert secret.status == 201
        secret_id = string_value(secret.object()["secret_public_id"])
        values: tuple[tuple[str, JsonObject], ...] = (
            ("value", {"field_name": "base_url", "value_plain": upstream.endpoint}),
            (
                "secret",
                {"field_name": "api_key", "secret_public_id": secret.object()["secret_public_id"]},
            ),
        )
        for kind, body in values:
            configured = api.request(
                ApiRequest(
                    Method.PATCH,
                    route + "/{indexer_instance_public_id}/fields/" + kind,
                    body,
                    path=path,
                )
            )
            assert configured.status == 204
        enabled = api.request(
            ApiRequest(
                Method.PATCH,
                route + _INSTANCE_PATH,
                {
                    "indexer_instance_public_id": identifier,
                    "is_enabled": True,
                    "enable_rss": False,
                },
                path=path,
            )
        )
        assert enabled.ok
        with isolated_indexer(api, identifier) if exclusive else nullcontext():
            yield identifier
    finally:
        # Stop this source before revoking its credential, and attempt both cleanups.
        with ExitStack() as cleanup:
            if secret_id is not None:
                cleanup.callback(
                    require_status,
                    api,
                    ApiRequest(
                        Method.DELETE, "/v1/indexers/secrets", {"secret_public_id": secret_id}
                    ),
                    204,
                )
            cleanup.callback(
                require_status,
                api,
                ApiRequest(
                    Method.PATCH,
                    route + _INSTANCE_PATH,
                    {"indexer_instance_public_id": identifier, "is_enabled": False},
                    path=path,
                ),
                200,
            )


def require_status(api: ApiClient, request: ApiRequest, status: int) -> None:
    assert api.request(request).status == status


@contextmanager
def isolated_indexer(api: ApiClient, selected: str) -> Iterator[None]:
    route = "/v1/indexers/instances"
    with ExitStack() as cleanup:
        for instance in rows(api, route, "indexer_instances"):
            identifier = string_value(instance["indexer_instance_public_id"])
            if identifier == selected or instance["instance_status"] != "enabled":
                continue
            path = {"indexer_instance_public_id": identifier}
            endpoint = route + _INSTANCE_PATH
            cleanup.callback(
                require_status,
                api,
                ApiRequest(Method.PATCH, endpoint, {**path, "is_enabled": True}, path=path),
                200,
            )
            require_status(
                api,
                ApiRequest(Method.PATCH, endpoint, {**path, "is_enabled": False}, path=path),
                200,
            )
        yield
