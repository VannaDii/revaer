"""Verify the controlled upstream before using it with the production application."""

import base64
import xml.etree.ElementTree as ET
from urllib.error import HTTPError
from urllib.parse import urlencode, urlsplit
from urllib.request import ProxyHandler, Request, build_opener, urlopen

import pytest

from tests.support.torznab import torznab_upstream


def test_controlled_torznab_upstream_serves_authenticated_release() -> None:
    with torznab_upstream() as upstream:
        with pytest.raises(HTTPError) as rejected:
            urlopen(upstream.endpoint, timeout=2)
        with rejected.value as error:
            assert error.code == 401
        query = urlencode({"t": "search", "q": upstream.title, "apikey": upstream.api_key})
        with urlopen(upstream.endpoint + "?" + query, timeout=2) as response:
            assert response.status == 200
            root = ET.fromstring(response.read())
        assert root.findtext("./channel/item/title") == upstream.title
        assert root.findtext("./channel/item/link") == upstream.magnet
        assert (upstream.title, True) in upstream.requests


@pytest.mark.parametrize(("scenario", "expected_items"), [("empty", 0), ("duplicate", 2)])
def test_controlled_torznab_response_shapes(scenario: str, expected_items: int) -> None:
    with torznab_upstream() as upstream:
        if scenario == "empty":
            upstream.scenario = "empty"
        else:
            upstream.scenario = "duplicate"
        root = ET.fromstring(upstream.feed())
        assert len(root.findall("./channel/item")) == expected_items
        for item in root.findall("./channel/item"):
            assert item.findtext("title") == upstream.title


def test_controlled_torznab_can_fail_an_authenticated_request() -> None:
    with torznab_upstream() as upstream:
        upstream.scenario = "unauthorized"
        query = urlencode({"t": "search", "q": upstream.title, "apikey": upstream.api_key})
        with pytest.raises(HTTPError) as rejected:
            urlopen(upstream.endpoint + "?" + query, timeout=2)
        with rejected.value as error:
            assert error.code == 401
        assert upstream.requests == [(upstream.title, True)]
        upstream.scenario = "malformed"
        malformed = upstream.feed()
        with pytest.raises(ET.ParseError):
            ET.fromstring(malformed)


@pytest.mark.parametrize("authenticated", [False, True])
def test_controlled_torznab_accepts_absolute_form_proxy_requests(authenticated: bool) -> None:
    with torznab_upstream() as upstream:
        proxy = upstream.endpoint.removesuffix("/api")
        opener = build_opener(ProxyHandler({}))
        query = urlencode({"t": "search", "q": upstream.title, "apikey": upstream.api_key})
        upstream.require_proxy_auth = authenticated
        headers = {}
        if authenticated:
            headers["Proxy-Authorization"] = (
                "Basic "
                + base64.b64encode(("search-proxy:" + upstream.proxy_password).encode()).decode()
            )
        request = Request(upstream.endpoint + "?" + query, headers=headers)
        request.set_proxy(urlsplit(proxy).netloc, "http")
        with opener.open(request, timeout=2) as response:
            assert response.status == 200
            root = ET.fromstring(response.read())
        assert root.findtext("./channel/item/title") == upstream.title
        assert upstream.proxied_requests == [True]
        assert upstream.proxy_authenticated == [True]
        assert upstream.requests == [(upstream.title, True)]
