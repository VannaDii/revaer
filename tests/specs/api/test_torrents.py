"""The complete torrent lifecycle is one retryable, shardable scenario.

The former serial group shared IDs across twelve tests. Keeping the dependent
operations together prevents a shard or retry from using another test's state.
Authoring files belong to a temporary directory within the allowed filesystem.
"""

import tempfile
import uuid
from functools import partial
from pathlib import Path

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.json_data import JsonObject, array_value, object_value

from tests.support.polling import eventually

MAGNET = "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567"
TRACKER = "https://tracker.example/announce"


def listed(api: ApiClient, prefix: str, identifier: str) -> bool:
    response = api.request(ApiRequest(Method.GET, prefix + "/torrents"))
    assert response.ok
    return any(
        object_value(entry)["id"] == identifier
        for entry in array_value(response.object()["torrents"])
    )


def test_torrent_lifecycle(api: ApiClient, fs_root: Path) -> None:
    torrent_id, admin_id = str(uuid.uuid4()), str(uuid.uuid4())
    for prefix, identifier in (("/v1", torrent_id), ("/admin", admin_id)):
        response = api.request(
            ApiRequest(
                Method.POST,
                prefix + "/torrents",
                {"id": identifier, "magnet": MAGNET, "trackers": [TRACKER]},
            )
        )
        assert response.status == 202
    for prefix in ("/v1", "/admin"):
        eventually(partial(listed, api, prefix, torrent_id), prefix + " torrent listing")
        detail = api.request(
            ApiRequest(Method.GET, prefix + "/torrents/{id}", path={"id": torrent_id})
        )
        assert detail.ok and detail.object()["id"] == torrent_id
        for collection, name in (("categories", "category"), ("tags", "tag")):
            key = f"e2e-{prefix[1:]}-{name}"
            route = f"{prefix}/torrents/{collection}"
            assert api.request(ApiRequest(Method.PUT, route + "/{name}", {}, path={"name": key})).ok
            response = api.request(ApiRequest(Method.GET, route))
            assert response.ok
            assert any(object_value(entry)["name"] == key for entry in array_value(response.json()))

    with tempfile.TemporaryDirectory(prefix="e2e-author-", dir=fs_root) as directory:
        (Path(directory) / "seed.txt").write_text("revaer e2e")
        for prefix in ("/v1", "/admin"):
            author = api.request(
                ApiRequest(Method.POST, prefix + "/torrents/create", {"root_path": directory})
            )
            assert author.ok
            if prefix == "/v1":
                assert author.object()["metainfo"] and author.object()["magnet_uri"]
    assert (
        api.request(ApiRequest(Method.POST, "/v1/torrents/create", {"root_path": ""})).status == 400
    )

    route = "/v1/torrents/{id}"
    path = {"id": torrent_id}
    operations: tuple[tuple[Method, str, JsonObject], ...] = (
        (
            Method.POST,
            "/select",
            {"include": ["*.mkv"], "exclude": [], "skip_fluff": False, "priorities": []},
        ),
        (Method.PATCH, "/options", {"auto_managed": True}),
        (Method.POST, "/action", {"type": "pause"}),
        (
            Method.PATCH,
            "/trackers",
            {"trackers": ["https://tracker.example/alt"], "replace": False},
        ),
        (Method.DELETE, "/trackers", {"trackers": [TRACKER]}),
        (Method.PATCH, "/web_seeds", {"web_seeds": ["https://seed.example/file"], "replace": True}),
    )
    for method, suffix, body in operations:
        assert api.request(ApiRequest(method, route + suffix, body, path=path)).status == 202
        if method == Method.PATCH and suffix == "/trackers":
            assert api.request(ApiRequest(Method.GET, route + suffix, path=path)).ok
    invalid: tuple[tuple[Method, str, JsonObject], ...] = (
        (Method.PATCH, "/options", {}),
        (Method.PATCH, "/trackers", {"trackers": [], "replace": False}),
        (Method.DELETE, "/trackers", {"trackers": []}),
        (Method.PATCH, "/web_seeds", {"web_seeds": [], "replace": False}),
    )
    for method, suffix, body in invalid:
        assert api.request(ApiRequest(method, route + suffix, body, path=path)).status == 400
    for prefix in ("/v1", "/admin"):
        assert api.request(ApiRequest(Method.GET, prefix + "/torrents/{id}/peers", path=path)).ok
    assert (
        api.request(ApiRequest(Method.DELETE, "/admin/torrents/{id}", path={"id": admin_id})).status
        == 204
    )
