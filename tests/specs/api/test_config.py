"""Configuration, dashboard counters and the configured filesystem boundary."""

from pathlib import Path

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import array_value


def test_dashboard_snapshot(api: ApiClient) -> None:
    response = api.request(ApiRequest(Method.GET, "/v1/dashboard"))
    assert response.ok
    assert "download_bps" in response.object()
    assert "upload_bps" in response.object()


def test_config_read_and_patch(api: ApiClient) -> None:
    assert api.request(ApiRequest(Method.GET, "/v1/config")).ok
    patched = api.request(ApiRequest(Method.PATCH, "/v1/config", {}))
    assert patched.ok
    assert "revision" in patched.object()


def test_admin_settings_patch(api: ApiClient) -> None:
    response = api.request(ApiRequest(Method.PATCH, "/admin/settings", {}))
    assert response.ok
    assert "revision" in response.object()


def test_filesystem_browse(api: ApiClient, fs_root: Path) -> None:
    response = api.request(ApiRequest(Method.GET, "/v1/fs/browse", query={"path": str(fs_root)}))
    assert response.ok
    assert response.object()["path"]
    array_value(response.object()["entries"])
