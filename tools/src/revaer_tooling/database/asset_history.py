"""Fresh, complete provider history for the already-scoped ADR 588 ASSET-1 decision."""

from dataclasses import dataclass

from ..errors import ToolingError
from ..external.git import DiffArgs
from ..external.github import GitHub, GraphqlArgs
from ..external.release import ReleaseRepository
from ..json_data import Json, JsonObject, array_value, decode_unique, object_value, string_value

REPOSITORY: JsonObject = {"id": "R_kgDOQJiaFw", "nameWithOwner": "VannaDii/revaer"}
EXPIRED_EVENTS = frozenset(("ClosedEvent", "MergedEvent", "ReopenedEvent"))
QUERY = """
query($cursor: String) {
  repository(owner: "VannaDii", name: "revaer") {
    id nameWithOwner
    pullRequest(number: 130) {
      id state closed merged closedAt mergedAt updatedAt
      baseRefName baseRefOid headRefName headRefOid
      headRepository { id nameWithOwner }
      timelineItems(first: 100, after: $cursor) {
        totalCount pageInfo { hasNextPage endCursor }
        nodes { __typename ... on Node { id } }
      }
    }
  }
}
"""


def require(condition: bool, reason: str) -> None:
    if not condition:
        raise ToolingError("ASSET-1 " + reason)


def identity(pull: JsonObject, refs: DiffArgs) -> None:
    expected: JsonObject = {
        "id": "PR_kwDOQJiaF8749-ef",
        "state": "OPEN",
        "closed": False,
        "merged": False,
        "closedAt": None,
        "mergedAt": None,
        "baseRefOid": refs.base,
        "headRefOid": refs.head,
        "headRefName": "stack/media3-53-sonar-asset-inputs",
        "headRepository": REPOSITORY,
    }
    for key, value in expected.items():
        require(
            key in pull and type(pull[key]) is type(value) and pull[key] == value,
            "PR identity, refs or open state do not match",
        )
    for key in ("baseRefName", "updatedAt"):
        string_value(pull[key])


@dataclass(frozen=True)
class AssetHistory:
    github: GitHub

    def fetch(self, cursor: str | None) -> JsonObject:
        source = self.github.graphql(
            GraphqlArgs(ReleaseRepository("github.com", "VannaDii/revaer"), QUERY, cursor)
        )
        try:
            page = object_value(decode_unique(source))
        except ToolingError as error:
            raise ToolingError("ASSET-1 provider evidence is malformed") from error
        require("errors" not in page, "provider returned GraphQL errors")
        return page

    def verify(self, refs: DiffArgs) -> JsonObject:
        """Never cache acceptance or filter timeline event types.

        The exception expires permanently on close, merge or reopen. Every page
        must describe the same exact PR/revisions and the same complete timeline.
        A failed retry returns no earlier receipt.
        """
        refs.revisions()
        try:
            return self._verify(refs)
        except KeyError as error:
            raise ToolingError("ASSET-1 provider evidence is malformed or incomplete") from error

    def _verify(self, refs: DiffArgs) -> JsonObject:
        pages: list[Json] = []
        cursors: set[str] = set()
        node_ids: set[str] = set()
        signature: JsonObject | None = None
        expected_count: int | None = None
        cursor = None
        while True:
            page = self.fetch(cursor)
            repository = object_value(object_value(page["data"])["repository"])
            require(
                {key: repository.get(key) for key in REPOSITORY} == REPOSITORY,
                "wrong repository",
            )
            pull = object_value(repository["pullRequest"])
            identity(pull, refs)
            current = {key: value for key, value in pull.items() if key != "timelineItems"}
            if signature is None:
                signature = current
            require(current == signature, "PR identity changed during pagination")
            timeline = object_value(pull["timelineItems"])
            count = timeline["totalCount"]
            if not isinstance(count, int) or isinstance(count, bool) or count < 0:
                raise ToolingError("ASSET-1 invalid timeline count")
            if expected_count is None:
                expected_count = count
            require(count == expected_count, "timeline changed during pagination")
            nodes = array_value(timeline["nodes"])
            require(len(nodes) <= 100, "invalid timeline page")
            for value in nodes:
                node = object_value(value)
                kind, identifier = string_value(node["__typename"]), string_value(node["id"])
                require(
                    kind not in EXPIRED_EVENTS,
                    "exception permanently expired after close, merge or reopen",
                )
                require(identifier not in node_ids, "duplicate timeline event")
                node_ids.add(identifier)
            pages.append(page)
            info = object_value(timeline["pageInfo"])
            more, end = info["hasNextPage"], info["endCursor"]
            require(isinstance(more, bool), "invalid pagination state")
            cursor = None if end is None else string_value(end)
            require(not nodes or cursor is not None, "missing timeline cursor")
            if not more:
                require(len(node_ids) == expected_count, "incomplete timeline")
                return {
                    "pull_request": signature,
                    "timeline_count": expected_count,
                    "pages": pages,
                }
            require(
                bool(nodes)
                and len(node_ids) < expected_count
                and cursor is not None
                and cursor not in cursors,
                "incomplete or repeated timeline page",
            )
            if cursor is not None:
                cursors.add(cursor)
