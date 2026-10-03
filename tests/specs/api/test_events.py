"""Streaming routes deliver SSE headers and bounded data after a real change."""

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method


def test_log_stream(api: ApiClient) -> None:
    api.events("/v1/logs/stream")


@pytest.mark.parametrize("route", ["/v1/events", "/v1/events/stream", "/v1/torrents/events"])
def test_event_stream_after_config_change(api: ApiClient, route: str) -> None:
    def trigger() -> None:
        assert api.request(ApiRequest(Method.PATCH, "/v1/config", {})).ok

    api.events(route, trigger)
