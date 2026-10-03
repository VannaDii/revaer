"""Typed request inputs, route evidence, and OpenAPI response validation.

The client uses the selected checkout's OpenAPI document directly. There is no
generated TypeScript client or second copy of its response schemas to go stale.
"""

import re
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from enum import StrEnum
from urllib.parse import quote, urlencode

from jsonschema import Draft202012Validator

from ..errors import ToolingError
from ..external.http import Http, HttpRequest, HttpResult
from ..json_data import Json, JsonObject, object_value, string_value
from .coverage import RouteCoverage

type QueryValue = str | int | float | bool


class Method(StrEnum):
    GET = "GET"
    POST = "POST"
    PUT = "PUT"
    PATCH = "PATCH"
    DELETE = "DELETE"


@dataclass(frozen=True)
class ApiRequest:
    method: Method
    route: str
    body: Json = field(default=None, repr=False)
    path: Mapping[str, str] = field(default_factory=dict)
    query: Mapping[str, QueryValue] = field(default_factory=dict, repr=False)
    headers: Mapping[str, str] = field(default_factory=dict, repr=False)

    def target(self, base_url: str) -> str:
        if not self.route.startswith("/") or self.route.startswith("//"):
            raise ToolingError("API routes must be absolute paths within the selected service")
        placeholders = set(re.findall(r"\{([^{}]+)\}", self.route))
        if placeholders != set(self.path):
            raise ToolingError(f"Path parameters do not match {self.method} {self.route}")
        path = re.sub(
            r"\{([^{}]+)\}", lambda match: quote(self.path[match[1]], safe=""), self.route
        )
        query = urlencode(
            {
                key: str(value).lower() if isinstance(value, bool) else value
                for key, value in self.query.items()
            }
        )
        return base_url.rstrip("/") + path + (f"?{query}" if query else "")


class ApiSchema:
    def __init__(self, document: JsonObject) -> None:
        self.document = document
        self.paths = object_value(document.get("paths"))
        self.validator = Draft202012Validator(document)

    def validate(self, request: ApiRequest, response: HttpResult) -> None:
        methods = self.paths.get(request.route)
        operation = methods.get(request.method.lower()) if isinstance(methods, dict) else None
        if not isinstance(operation, dict):
            raise ToolingError(f"Undocumented API operation {request.method} {request.route}")
        responses = object_value(operation.get("responses"))
        definition = responses.get(str(response.status)) or responses.get("default")
        if definition is None:
            raise ToolingError(
                f"Undocumented {response.status} response for {request.method} {request.route}"
            )
        content = object_value(definition).get("content")
        if content is None:
            return
        media_types = object_value(content)
        media_type = response.headers.get("content-type", "").split(";", 1)[0]
        # RFC 9457 problem responses use application/problem+json even when the
        # OpenAPI generator describes the payload under application/json.
        entry = media_types.get(media_type)
        if entry is None and media_type.endswith("+json"):
            entry = media_types.get("application/json")
        if entry is None:
            raise ToolingError(f"Undocumented content type for {request.method} {request.route}")
        if media_type == "application/json" or media_type.endswith("+json"):
            schema = object_value(entry).get("schema")
            if schema is not None:
                failures = list(
                    self.validator.evolve(schema=object_value(schema)).iter_errors(response.json())
                )
                if failures:
                    # Do not include failing instance values: setup/secrets
                    # endpoints legitimately contain newly generated credentials.
                    failure = failures[0]
                    location = "/".join(str(part) for part in failure.absolute_path)
                    raise ToolingError(
                        f"OpenAPI response mismatch for {request.method} {request.route} "
                        f"at /{location}: {failure.validator}"
                    )


class ApiClient:
    def __init__(
        self,
        http: Http,
        base_url: str,
        coverage: RouteCoverage,
        schema: ApiSchema,
        headers: Mapping[str, str] | None = None,
    ) -> None:
        self.http = http
        self.base_url = base_url
        self.coverage = coverage
        self.schema = schema
        self.headers = dict(headers or {})

    def request(self, args: ApiRequest) -> HttpResult:
        self.coverage.record(f"{args.method} {args.route}")
        result = self.http.request(
            HttpRequest(
                args.method, args.target(self.base_url), {**self.headers, **args.headers}, args.body
            )
        )
        self.schema.validate(args, result)
        return result

    def events(self, route: str, trigger: Callable[[], None] | None = None) -> None:
        """Assert SSE headers and, when triggered, a nonempty bounded first chunk."""
        self.coverage.record(f"GET {route}")
        with self.http.stream(
            HttpRequest(
                "GET",
                ApiRequest(Method.GET, route).target(self.base_url),
                {**self.headers, "accept": "text/event-stream"},
                timeout=3,
            )
        ) as response:
            if response.status != 200 or "text/event-stream" not in response.headers.get(
                "content-type", ""
            ):
                raise ToolingError(f"Expected a successful SSE response from {route}")
            if trigger is not None:
                trigger()
                if not response.read(1):
                    raise ToolingError(f"SSE stream {route} ended before delivering data")


@dataclass(frozen=True)
class ApiSession:
    auth_mode: str
    api_key: str | None = field(default=None, repr=False)

    def headers(self) -> dict[str, str]:
        return {"x-revaer-api-key": self.api_key} if self.api_key else {}


def configure_auth(client: ApiClient, mode: str, filesystem_root: str) -> ApiSession:
    """The runner calls this only against its newly created disposable database."""
    health = client.request(ApiRequest(Method.GET, "/health"))
    if not health.ok:
        raise ToolingError("Initial health check failed")
    reset = client.request(
        ApiRequest(Method.POST, "/admin/factory-reset", {"confirm": "factory reset"})
    )
    if reset.status != 204:
        raise ToolingError(f"Factory reset failed with {reset.status}")
    if client.request(ApiRequest(Method.GET, "/health")).object().get("mode") != "setup":
        raise ToolingError("Factory reset did not enter setup mode")
    start = client.request(ApiRequest(Method.POST, "/admin/setup/start", {}))
    token = string_value(start.object().get("token"))
    snapshot = client.request(ApiRequest(Method.GET, "/.well-known/revaer.json")).object()
    profile = object_value(snapshot.get("app_profile"))
    filesystem = object_value(snapshot.get("fs_policy"))
    complete = client.request(
        ApiRequest(
            Method.POST,
            "/admin/setup/complete",
            {
                "app_profile": {**profile, "auth_mode": mode},
                "fs_policy": {**filesystem, "allow_paths": [filesystem_root]},
            },
            headers={"x-revaer-setup-token": token},
        )
    )
    if not complete.ok:
        raise ToolingError(f"Setup completion failed with {complete.status}")
    key = complete.object().get("api_key")
    return ApiSession(mode, string_value(key) if mode == "api_key" else None)
