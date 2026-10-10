"""Local Sonar protocol fixtures and an explicitly simulated scanner process."""

import copy
import json
import sys
import threading
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass, field, replace
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlsplit

from revaer_tooling.json_data import JsonObject, array_value
from revaer_tooling.process import Completed, Invocation, ProcessRunner, RunningProcess


def result_records() -> dict[str, JsonObject]:
    """Representative responses use every field required by the existing gate."""
    return {
        "ce-task": {
            "task": {
                "id": "owned-task",
                "componentKey": "VannaDii_Revaer",
                "status": "SUCCESS",
                "analysisId": "owned-analysis",
            }
        },
        "measures": {
            "component": {
                "measures": [
                    {"metric": "coverage", "value": "94.1"},
                    {"metric": "line_coverage", "value": "94.9"},
                    {"metric": "lines_to_cover", "value": "1000"},
                    {"metric": "uncovered_lines", "value": "0"},
                ]
            }
        },
        "quality-gate": {"projectStatus": {"status": "OK", "ignoredConditions": False}},
        "issues": {"total": 0},
        "all-issues": {"total": 0, "issues": []},
        "hotspots": {"paging": {"total": 0}},
    }


class ScannerFixtureRunner:
    """Run a real child that models scanner I/O, without pretending to analyze."""

    def __init__(self) -> None:
        self.invocations: list[Invocation] = []
        self.messages: list[str] = []
        self.processes = ProcessRunner(self.messages.append)

    def start(self, invocation: Invocation) -> RunningProcess:
        self.invocations.append(invocation)
        fixture = Path(__file__).with_name("scanner_process.py")
        return self.processes.start(
            replace(invocation, argv=(sys.executable, str(fixture), *invocation.argv[1:]))
        )

    def run(self, invocation: Invocation) -> Completed:
        return self.start(invocation).wait()


@dataclass
class ApiState:
    records: dict[str, JsonObject] = field(default_factory=result_records)
    statuses: list[int] = field(default_factory=list)
    requests: list[tuple[str, dict[str, list[str]], str]] = field(default_factory=list)
    pending_tasks: int = 0


@contextmanager
def sonar_server(state: ApiState) -> Iterator[str]:
    endpoints = {
        "/api/ce/task": "ce-task",
        "/api/measures/component": "measures",
        "/api/qualitygates/project_status": "quality-gate",
        "/api/issues/search": "issues",
        "/api/hotspots/search": "hotspots",
    }

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:
            request = urlsplit(self.path)
            state.requests.append(
                (request.path, parse_qs(request.query), self.headers.get("Authorization", ""))
            )
            status = state.statuses.pop(0) if state.statuses else 200
            body = copy.deepcopy(state.records[endpoints[request.path]])
            if request.path == "/api/issues/search" and parse_qs(request.query).get("ps") == [
                "500"
            ]:
                body = copy.deepcopy(state.records["all-issues"])
                page = int(parse_qs(request.query).get("p", ["1"])[0])
                body["issues"] = array_value(body["issues"])[(page - 1) * 500 : page * 500]
            if request.path == "/api/ce/task" and state.pending_tasks:
                state.pending_tasks -= 1
                body = {
                    "task": {
                        "id": "owned-task",
                        "componentKey": "VannaDii_Revaer",
                        "status": "PENDING",
                    }
                }
            encoded = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(encoded)))
            self.end_headers()
            self.wfile.write(encoded)

        def log_message(self, format: str, *args: object) -> None:
            """Requests are retained in state; suppress the fixture server's stderr."""

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    worker = threading.Thread(target=server.serve_forever, daemon=True)
    worker.start()
    try:
        yield f"http://127.0.0.1:{server.server_port}/api"
    finally:
        server.shutdown()
        server.server_close()
        worker.join(timeout=5)
        if worker.is_alive():
            raise RuntimeError("Sonar fixture server did not stop")
