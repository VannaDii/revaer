"""Provider failures cannot revive or broaden the existing binary exception."""

import json
import sys
from pathlib import Path

import pytest
from revaer_tooling.database.asset_history import AssetHistory
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.git import DiffArgs
from revaer_tooling.external.github import GitHub, GraphqlArgs
from revaer_tooling.external.release import ReleaseRepository
from revaer_tooling.json_data import Json, JsonObject, object_value
from revaer_tooling.process import ProcessRunner
from test_external import RecordingRunner

REFS = DiffArgs("1" * 40, "2" * 40)


def page(
    ids: tuple[int, ...] = (),
    *,
    count: int | None = None,
    more: bool = False,
    cursor: str | None = None,
    refs: DiffArgs = REFS,
) -> JsonObject:
    nodes: list[Json] = [{"id": f"event-{index}", "__typename": "IssueComment"} for index in ids]
    repository: JsonObject = {"id": "R_kgDOQJiaFw", "nameWithOwner": "VannaDii/revaer"}
    return {
        "data": {
            "repository": {
                **repository,
                "pullRequest": {
                    "id": "PR_kwDOQJiaF8749-ef",
                    "state": "OPEN",
                    "closed": False,
                    "merged": False,
                    "closedAt": None,
                    "mergedAt": None,
                    "updatedAt": "2026-09-12T00:00:00Z",
                    "baseRefName": "stack/prerequisite",
                    "baseRefOid": refs.base,
                    "headRefName": "stack/media3-53-sonar-asset-inputs",
                    "headRefOid": refs.head,
                    "headRepository": repository,
                    "timelineItems": {
                        "totalCount": len(nodes) if count is None else count,
                        "nodes": nodes,
                        "pageInfo": {"hasNextPage": more, "endCursor": cursor},
                    },
                },
            }
        }
    }


def pull(document: JsonObject) -> JsonObject:
    return object_value(object_value(object_value(document["data"])["repository"])["pullRequest"])


class Provider(GitHub):
    def __init__(self, pages: list[JsonObject | str]) -> None:
        super().__init__(sys.executable, ProcessRunner(lambda message: None), Path.cwd(), {})
        self.pages = [json.dumps(item) if isinstance(item, dict) else item for item in pages]
        self.requests: list[GraphqlArgs] = []

    def graphql(self, args: GraphqlArgs) -> str:
        self.requests.append(args)
        if not self.pages:
            raise ToolingError("Provider unavailable")
        return self.pages.pop(0)


def test_fresh_complete_history_and_opaque_pagination() -> None:
    provider = Provider([page()])
    history = AssetHistory(provider)
    assert history.verify(REFS)["timeline_count"] == 0
    with pytest.raises(ToolingError, match="unavailable"):
        history.verify(REFS)
    provider = Provider(
        [
            page(tuple(range(100)), count=101, more=True, cursor="opaque; first page"),
            page((100,), count=101, cursor="end"),
        ]
    )
    assert AssetHistory(provider).verify(REFS)["timeline_count"] == 101
    assert [request.cursor for request in provider.requests] == [None, "opaque; first page"]
    assert all("itemTypes" not in request.query for request in provider.requests)
    assert all(request.repository.name == "VannaDii/revaer" for request in provider.requests)


@pytest.mark.parametrize(
    "field,value",
    (
        ("id", "other"),
        ("state", "CLOSED"),
        ("closed", True),
        ("closed", 0),
        ("merged", True),
        ("closedAt", "2026-09-12"),
        ("mergedAt", "2026-09-12"),
        ("baseRefOid", "3" * 40),
        ("headRefOid", "3" * 40),
        ("headRefName", "other"),
        ("headRepository", {"id": "fork", "nameWithOwner": "VannaDii/revaer"}),
        ("baseRefName", ""),
        ("updatedAt", None),
    ),
)
def test_wrong_or_missing_identity_cannot_qualify(field: str, value: Json) -> None:
    document = page()
    pull(document)[field] = value
    with pytest.raises(ToolingError):
        AssetHistory(Provider([document])).verify(REFS)
    document = page()
    del pull(document)[field]
    with pytest.raises(ToolingError):
        AssetHistory(Provider([document])).verify(REFS)


@pytest.mark.parametrize(
    "raw",
    (
        "invalid",
        "[]",
        '{"data":null,"data":null}',
        '{"errors":[]}',
        json.dumps(page()).replace('"closed": false', '"closed": true, "\\u0063losed": false'),
    ),
)
def test_ambiguous_or_partial_provider_response_fails(raw: str) -> None:
    with pytest.raises(ToolingError):
        AssetHistory(Provider([raw])).verify(REFS)


@pytest.mark.parametrize(
    "field,value",
    (
        ("totalCount", -1),
        ("totalCount", True),
        ("totalCount", 26),
        ("nodes", None),
        ("nodes", [None]),
        ("pageInfo", {"hasNextPage": "false", "endCursor": None}),
        ("pageInfo", {"hasNextPage": True, "endCursor": None}),
        ("pageInfo", {"hasNextPage": False, "endCursor": ""}),
    ),
)
def test_invalid_or_incomplete_timeline_fails(field: str, value: Json) -> None:
    document = page()
    object_value(pull(document)["timelineItems"])[field] = value
    with pytest.raises(ToolingError):
        AssetHistory(Provider([document])).verify(REFS)


@pytest.mark.parametrize("event", ("ClosedEvent", "MergedEvent", "ReopenedEvent"))
def test_close_merge_and_reopen_permanently_expire_even_on_the_last_page(event: str) -> None:
    first = page(tuple(range(100)), count=101, more=True, cursor="first")
    last = page((100,), count=101, cursor="last")
    object_value(pull(last)["timelineItems"])["nodes"] = [{"id": "event-100", "__typename": event}]
    with pytest.raises(ToolingError, match="permanently expired"):
        AssetHistory(Provider([first, last])).verify(REFS)


@pytest.mark.parametrize(
    "change", ("identity", "count", "duplicate", "repeat", "cursor", "missing")
)
def test_partial_or_changed_later_page_never_completes(change: str) -> None:
    first = page(tuple(range(100)), count=101, more=True, cursor="first")
    last = page((100,), count=101, cursor="last")
    timeline = object_value(pull(last)["timelineItems"])
    match change:
        case "identity":
            pull(last)["updatedAt"] = "changed"
        case "count":
            timeline["totalCount"] = 102
        case "duplicate":
            timeline["nodes"] = [{"id": "event-99", "__typename": "IssueComment"}]
        case "repeat":
            timeline["pageInfo"] = {"hasNextPage": True, "endCursor": "first"}
        case "cursor":
            timeline["pageInfo"] = {"hasNextPage": False, "endCursor": None}
        case "missing":
            timeline["nodes"] = [{"__typename": "IssueComment"}]
    with pytest.raises(ToolingError):
        AssetHistory(Provider([first, last])).verify(REFS)


def test_wrong_repository_oversized_page_and_failed_later_fetch() -> None:
    document = page()
    object_value(object_value(document["data"])["repository"])["id"] = "other"
    with pytest.raises(ToolingError, match="wrong repository"):
        AssetHistory(Provider([document])).verify(REFS)
    with pytest.raises(ToolingError, match="invalid timeline page"):
        AssetHistory(Provider([page(tuple(range(101)), cursor="end")])).verify(REFS)
    first = page((0,), count=2, more=True, cursor="first")
    with pytest.raises(ToolingError, match="unavailable"):
        AssetHistory(Provider([first])).verify(REFS)


def test_native_graphql_adapter_keeps_cursor_literal_and_provider_explicit(tmp_path: Path) -> None:
    runner = RecordingRunner('{"data":null,"data":null}')
    github = GitHub(sys.executable, runner, tmp_path, {})
    args = GraphqlArgs(
        ReleaseRepository("github.com", "VannaDii/revaer"),
        "query { viewer { id } }",
        "opaque; $(touch unwanted)",
    )
    assert github.graphql(args) == runner.stdout
    call = runner.calls[0]
    assert call.argv[1:5] == ("api", "--hostname", "github.com", "graphql")
    assert call.argv[-2:] == ("--raw-field", "cursor=" + str(args.cursor))
    assert call.env["GH_HOST"] == "github.com" and call.env["GH_REPO"] == "VannaDii/revaer"
    assert call.capture and call.timeout == 60
    assert not (tmp_path / "unwanted").exists()
    with pytest.raises(ToolingError):
        github.graphql(GraphqlArgs(args.repository, "", args.cursor))
    with pytest.raises(ToolingError):
        github.graphql(GraphqlArgs(args.repository, args.query, ""))
