"""Typed scanner invocation and bounded, authenticated Sonar API requests."""

import base64
import re
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal
from urllib.parse import urlencode

from ..errors import ToolingError
from ..json_data import JsonObject
from ..process import Completed, Invocation
from ..sonar.settings import SonarSettings
from .base import ExternalTool, ToolVersion
from .http import Http, HttpRequest

type Endpoint = Literal[
    "ce/task",
    "measures/component",
    "qualitygates/project_status",
    "issues/search",
    "hotspots/search",
]


@dataclass(frozen=True)
class ScanArgs:
    log: Path
    token: str = field(repr=False)


class SonarScanner(ExternalTool):
    def verify(self, required: str | None = None) -> ToolVersion:
        result = self._invoke(("--version",), capture=True, timeout=30)
        match = re.search(r"SonarScanner CLI (\d+\.\d+\.\d+\.\d+)", result.stdout + result.stderr)
        if match is None:
            raise ToolingError("Cannot determine SonarScanner CLI version")
        version = match.group(1)
        if required is not None and version != required:
            raise ToolingError(
                f"SonarScanner CLI {version} found; {required} required; "
                "run rv setup --sonar-scanner"
            )
        return ToolVersion(self.locate(), version)

    def scan(self, args: ScanArgs) -> Completed:
        # No argument escape hatch: reviewed scanner criteria live exclusively
        # in sonar-project.properties. The token remains in the environment.
        if not args.token:
            raise ToolingError("SONAR_TOKEN is required for the authoritative scanner")
        for name in (
            "SONAR_SCANNER_JSON_PARAMS",
            "SONARQUBE_SCANNER_PARAMS",
            "SONAR_SCANNER_PARAMS",
        ):
            if self.environment.get(name):
                raise ToolingError(f"{name} cannot override sonar-project.properties")
        for name in (
            "SONAR_SCANNER_OPTS",
            "SONAR_SCANNER_JAVA_OPTS",
            "JAVA_TOOL_OPTIONS",
            "JDK_JAVA_OPTIONS",
        ):
            if re.search(
                r"-D\s*(?:sonar\.|project\.settings|project\.home)", self.environment.get(name, "")
            ):
                raise ToolingError(f"{name} cannot override sonar-project.properties")
        return self.runner.run(
            Invocation(
                (str(self.locate()),),
                self.root,
                {**self.environment, "SONAR_TOKEN": args.token},
                log_path=args.log,
            )
        )


class SonarApi:
    def __init__(
        self,
        http: Http,
        settings: SonarSettings,
        pause: Callable[[float], None],
        emit: Callable[[str], None],
    ) -> None:
        self.http, self.settings, self.pause, self.emit = http, settings, pause, emit

    def get(self, endpoint: Endpoint, parameters: Mapping[str, str]) -> JsonObject:
        if not self.settings.auth_token:
            raise ToolingError(
                "SONAR_AUTH_TOKEN or SONAR_TOKEN is required for result verification"
            )
        credential = base64.b64encode((self.settings.auth_token + ":").encode()).decode("ascii")
        request = HttpRequest(
            "GET",
            f"{self.settings.api_base}/{endpoint}?{urlencode(parameters)}",
            {"Authorization": "Basic " + credential},
            timeout=30,
        )
        for attempt in range(1, self.settings.api_attempts + 1):
            try:
                response = self.http.request(request)
            except ToolingError:
                if attempt == self.settings.api_attempts:
                    raise
                self.emit(
                    f"Sonar API {endpoint} connection failed; "
                    f"retrying {attempt}/{self.settings.api_attempts}"
                )
                self.pause(self.settings.api_delay)
                continue
            if response.ok:
                return response.object()
            if response.status != 429 and not 500 <= response.status <= 599:
                raise ToolingError(f"Sonar API {endpoint} returned HTTP {response.status}")
            if attempt == self.settings.api_attempts:
                raise ToolingError(
                    f"Sonar API {endpoint} returned HTTP {response.status} after {attempt} attempts"
                )
            self.emit(
                f"Sonar API {endpoint} returned HTTP {response.status}; "
                f"retrying {attempt}/{self.settings.api_attempts}"
            )
            self.pause(self.settings.api_delay)
        # Settings are validated at construction. Keep this invariant explicit if
        # a caller supplies a settings object directly rather than through CLI.
        raise ToolingError("Sonar API retry count must be positive")
