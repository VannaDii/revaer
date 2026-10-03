"""Real HTTP boundaries for route encoding, schemas, streaming and diagnostics."""

import json
import threading
from collections.abc import Iterator
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.request import ProxyHandler, build_opener

import pytest
from revaer_tooling.e2e.api import ApiClient, ApiRequest, ApiSchema, Method
from revaer_tooling.e2e.coverage import RouteCoverage, required_api_operations, verify_routes
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.http import Http, HttpRequest, HttpResult, VisibleRedirects
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.json_data import array_value, decode, object_value, string_value


@pytest.fixture
def endpoint() -> Iterator[tuple[str, Http]]:
    triggered = threading.Event()

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            if self.path == "/redirect":
                self.send_response(302)
                self.send_header("location", "/must-not-follow")
                self.end_headers()
                return
            if self.path == "/events":
                self.send_response(200)
                self.send_header("content-type", "text/event-stream")
                self.end_headers()
                self.wfile.flush()
                if triggered.wait(3):
                    self.wfile.write(b"data: changed\n\n")
                    self.wfile.flush()
                return
            self.send_response(200)
            self.send_header("content-type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps({"path": self.path}).encode())

        def do_POST(self) -> None:
            triggered.set()
            self.send_response(204)
            self.end_headers()

        def log_message(self, format: str, *args: object) -> None:
            # Tests make assertions on responses instead of printing access logs.
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever)
    thread.start()
    try:
        yield (
            f"http://127.0.0.1:{server.server_port}",
            Http(build_opener(ProxyHandler({}), VisibleRedirects())),
        )
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def schema() -> ApiSchema:
    return ApiSchema(
        {
            "openapi": "3.1.0",
            "paths": {
                "/value/{id}": {
                    "get": {
                        "responses": {
                            "200": {
                                "content": {
                                    "application/json": {
                                        "schema": {"$ref": "#/components/schemas/Value"}
                                    }
                                }
                            }
                        }
                    }
                }
            },
            "components": {
                "schemas": {
                    "Value": {
                        "type": "object",
                        "required": ["path"],
                        "properties": {"path": {"type": "string"}},
                    }
                }
            },
        }
    )


def test_literal_parameters_and_actual_response_are_recorded(
    endpoint: tuple[str, Http], tmp_path: Path
) -> None:
    base, http = endpoint
    evidence = RouteCoverage(tmp_path / "api-coverage-worker.json", FileSystem())
    client = ApiClient(http, base, evidence, schema())
    request = ApiRequest(
        Method.GET,
        "/value/{id}",
        path={"id": "space/slash?"},
        query={"q": "a&b", "enabled": True},
    )
    result = client.request(request)
    assert result.ok
    assert result.object() == {"path": "/value/space%2Fslash%3F?q=a%26b&enabled=true"}
    assert json.loads(evidence.path.read_text()) == ["GET /value/{id}"]


@pytest.mark.parametrize(
    "arguments",
    [
        ApiRequest(Method.GET, "https://unrelated.invalid"),
        ApiRequest(Method.GET, "//unrelated.invalid"),
        ApiRequest(Method.GET, "/value/{id}"),
        ApiRequest(Method.GET, "/value", path={"id": "unexpected"}),
    ],
)
def test_route_identity_rejects_wrong_host_or_parameters(arguments: ApiRequest) -> None:
    with pytest.raises(ToolingError):
        arguments.target("http://127.0.0.1:7070")


def test_response_refs_are_validated_without_leaking_instance_values() -> None:
    document = schema()
    request = ApiRequest(Method.GET, "/value/{id}", path={"id": "x"})
    with pytest.raises(ToolingError, match="OpenAPI response mismatch") as failure:
        document.validate(
            request,
            HttpResult(
                200, {"content-type": "application/json"}, '{"path": ["private-generated-value"]}'
            ),
        )
    assert "private-generated-value" not in str(failure.value)
    with pytest.raises(ToolingError, match="Undocumented 503"):
        document.validate(request, HttpResult(503, {}, ""))
    with pytest.raises(ToolingError, match="content type"):
        document.validate(request, HttpResult(200, {"content-type": "text/html"}, ""))


@pytest.mark.parametrize(
    "arguments",
    [ApiRequest(Method.GET, "/missing"), ApiRequest(Method.POST, "/value/{id}", path={"id": "x"})],
)
def test_undocumented_operations_report_the_method_and_route(arguments: ApiRequest) -> None:
    with pytest.raises(ToolingError, match="Undocumented API operation") as failure:
        schema().validate(arguments, HttpResult(200, {"content-type": "application/json"}, "{}"))
    assert f"{arguments.method} {arguments.route}" in str(failure.value)


def test_http_redirect_is_observable(endpoint: tuple[str, Http]) -> None:
    base, http = endpoint
    result = http.request(HttpRequest("GET", base + "/redirect"))
    assert result.status == 302
    assert result.headers["location"] == "/must-not-follow"


def test_stream_waits_for_triggered_data_and_closes(
    endpoint: tuple[str, Http], tmp_path: Path
) -> None:
    base, http = endpoint
    evidence = RouteCoverage(tmp_path / "api-coverage-stream.json", FileSystem())
    client = ApiClient(http, base, evidence, schema())

    def trigger() -> None:
        assert http.request(HttpRequest("POST", base + "/trigger")).status == 204

    client.events("/events", trigger)
    assert json.loads(evidence.path.read_text()) == ["GET /events"]
    with pytest.raises(ToolingError, match="SSE response"):
        client.events("/not-a-stream")


def test_refused_connection_does_not_expose_query_credentials() -> None:
    http = Http(build_opener(ProxyHandler({}), VisibleRedirects()))
    with pytest.raises(ToolingError, match="connection") as failure:
        http.request(HttpRequest("GET", "http://127.0.0.1:1/?apikey=private-value", timeout=0.1))
    assert "private-value" not in str(failure.value)


def test_route_coverage_unions_workers_and_rejects_missing_or_invalid_evidence(
    tmp_path: Path,
) -> None:
    filesystem = FileSystem()
    required = required_api_operations(json.dumps(schema().document))
    assert required == {"GET /value/{id}"}
    with pytest.raises(ToolingError, match="not produced"):
        verify_routes(filesystem, tmp_path, "API", required)
    first = RouteCoverage(tmp_path / "api-coverage-one.json", filesystem)
    first.record("GET /other")
    with pytest.raises(ToolingError, match="missing 1"):
        verify_routes(filesystem, tmp_path, "API", required)
    second = RouteCoverage(tmp_path / "api-coverage-two.json", filesystem)
    second.record("GET /value/{id}")
    verify_routes(filesystem, tmp_path, "API", required)
    first.path.write_text("{}")
    with pytest.raises(ToolingError, match="array"):
        verify_routes(filesystem, tmp_path, "API", required)
    with pytest.raises(ToolingError, match="no API operations"):
        required_api_operations('{"paths": {}}')


def test_json_accessors_keep_wrong_response_shapes_explicit() -> None:
    assert string_value("key") == "key"
    assert array_value([1]) == [1]
    assert object_value({"x": None}) == {"x": None}
    for parse, value in ((array_value, {}), (object_value, []), (string_value, "")):
        with pytest.raises(ToolingError, match="Expected"):
            parse(value)
    with pytest.raises(ToolingError, match="valid JSON"):
        decode("invalid")
