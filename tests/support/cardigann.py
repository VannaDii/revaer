"""Controlled public HTML and normal Cardigann import/configuration boundaries."""

import secrets
import threading
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field
from html import escape
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Literal

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import string_value

from tests.support.api_assertions import rows
from tests.support.torznab import isolated_indexer, require_status


@dataclass
class HtmlUpstream:
    layout: Literal["table", "list"] = "table"
    identity: str = field(default_factory=lambda: secrets.token_hex(20))
    origin: str = ""
    requests: list[str] = field(default_factory=list)
    empty: bool = False
    invalid_size: bool = False
    status_code: int = 200

    @property
    def search_path(self) -> str:
        return "/releases.php" if self.layout == "table" else "/browse/list"

    @property
    def query_key(self) -> str:
        return "term" if self.layout == "table" else "needle"

    @property
    def title(self) -> str:
        return "Cardigann café & + / " + self.identity[:12]

    def definition(self) -> str:
        definition = f"""id: html-{self.identity}
name: Controlled public HTML
description: Public HTML acceptance
language: en-US
type: public
encoding: UTF-8
links:
  - {self.origin}/
settings: []
caps:
  categorymappings:
    - {{id: 42, cat: Movies}}
  modes:
    search: [q]
search:
  paths:
    - path: releases.php
      method: get
  inputs:
    term: "{{{{ .Keywords }}}}"
    order: newest
  rows:
    selector: "table.releases > tbody > tr.release"
  fields:
    title:
      selector: a.title
    details:
      selector: a.title
      attribute: href
    magnet:
      selector: a.magnet
      attribute: href
    category:
      selector: td.category
    size:
      selector: td.bytes
    seeders:
      selector: td.seeders
    leechers:
      selector: td.leechers
    date:
      selector: time
      attribute: datetime
"""
        if self.layout == "list":
            definition = (
                definition.replace("id: 42", "id: film")
                .replace("path: releases.php", "path: browse/list")
                .replace("term:", "needle:")
                .replace("table.releases > tbody > tr.release", "ol#catalog > li.result")
                .replace("a.title", "a.release-name")
                .replace("td.category", "span.category")
                .replace("selector: td.bytes", "selector: span.stats\n      attribute: data-bytes")
                .replace("td.seeders", "span.seeders")
                .replace("td.leechers", "span.leechers")
            )
        return definition


@contextmanager
def html_upstream(layout: Literal["table", "list"] = "table") -> Iterator[HtmlUpstream]:
    upstream = HtmlUpstream(layout=layout)

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            upstream.requests.append(self.path)
            row = f"""<tr class="release"><td>
<a class="title" href="/details/{upstream.identity}">{escape(upstream.title)}</a>
<a class="magnet" href="magnet:?xt=urn:btih:{upstream.identity}">magnet</a></td>
<td class="category">42</td><td class="bytes">1048576</td><td class="seeders">17</td>
<td class="leechers">2</td><td><time datetime="2026-10-24T00:00:00Z">recent</time></td></tr>"""
            body_text = (
                "<!doctype html><html><body><table class=releases><tbody>"
                + row * 2
                + "</tbody></table></body></html>"
            )
            if upstream.layout == "list":
                row = f"""<li class="result"><a class="release-name"
href="/details/{upstream.identity}">{escape(upstream.title)}</a>
<a class="magnet" href="magnet:?xt=urn:btih:{upstream.identity}">magnet</a>
<span class="category">film</span><span class="stats" data-bytes="1048576"></span>
<span class="seeders">17</span><span class="leechers">2</span>
<time datetime="2026-10-24T00:00:00Z">recent</time></li>"""
                body_text = "<!doctype html><ol id=catalog>" + row * 2 + "</ol>"
            if upstream.empty:
                body_text = "<!doctype html><html><body>No releases</body></html>"
            if upstream.invalid_size:
                body_text = body_text.replace("1048576", "not-a-size")
            body = body_text.encode()
            self.send_response(upstream.status_code)
            self.send_header("Content-Type", "text/html; charset=utf-8")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, format: str, *args: object) -> None:
            return

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = False
    upstream.origin = f"http://127.0.0.1:{server.server_port}"
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield upstream
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=1)
        assert not thread.is_alive(), "Owned HTML upstream must stop"


@contextmanager
def configure_cardigann(api: ApiClient, upstream: HtmlUpstream) -> Iterator[tuple[str, str]]:
    profiles = rows(api, "/v1/indexers/search-profiles", "search_profiles")
    imported = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/definitions/import/cardigann",
            {"yaml_payload": upstream.definition()},
        )
    )
    assert imported.status == 201
    created = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/instances",
            {
                "indexer_definition_upstream_slug": "html-" + upstream.identity,
                "display_name": "HTML " + upstream.identity[:12],
            },
        )
    )
    assert created.status == 201
    identifier = string_value(created.object()["indexer_instance_public_id"])
    path = {"indexer_instance_public_id": identifier}
    require_status(
        api,
        ApiRequest(
            Method.PATCH,
            "/v1/indexers/instances/{indexer_instance_public_id}",
            {**path, "is_enabled": True, "enable_rss": False},
            path=path,
        ),
        200,
    )
    profile = api.request(
        ApiRequest(
            Method.POST,
            "/v1/indexers/search-profiles",
            {
                "display_name": "HTML profile " + upstream.identity[:12],
                "is_default": True,
                "page_size": 20,
                "default_media_domain_key": "movies",
            },
        )
    )
    assert profile.status == 201
    profile_id = string_value(profile.object()["search_profile_public_id"])
    try:
        with isolated_indexer(api, identifier):
            yield identifier, profile_id
    finally:
        require_status(
            api,
            ApiRequest(
                Method.PATCH,
                "/v1/indexers/instances/{indexer_instance_public_id}",
                {**path, "is_enabled": False},
                path=path,
            ),
            200,
        )
        for previous in profiles:
            if previous["is_default"]:
                previous_path = {
                    "search_profile_public_id": string_value(previous["search_profile_public_id"])
                }
                require_status(
                    api,
                    ApiRequest(
                        Method.POST,
                        "/v1/indexers/search-profiles/{search_profile_public_id}/default",
                        {},
                        path=previous_path,
                    ),
                    204,
                )
