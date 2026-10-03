"""Bounded standard-library HTTP requests and streaming responses.

HTTP error statuses are response data for API assertions. Connection failures
remain errors. Redirects are deliberately visible: following one would hide an
endpoint's status and could forward a credential to another service.
"""

import json
import tempfile
from collections.abc import Iterator, Mapping
from contextlib import contextmanager
from dataclasses import dataclass, field
from http.client import HTTPException, HTTPMessage, HTTPResponse
from pathlib import Path
from typing import IO, cast
from urllib.error import HTTPError, URLError
from urllib.parse import urljoin, urlsplit
from urllib.request import HTTPRedirectHandler, OpenerDirector, Request

from ..errors import ToolingError
from ..json_data import Json, JsonObject, decode, object_value


class VisibleRedirects(HTTPRedirectHandler):
    def redirect_request(
        self,
        req: Request,
        fp: IO[bytes],
        code: int,
        msg: str,
        headers: HTTPMessage,
        newurl: str,
    ) -> None:
        return None


@dataclass(frozen=True)
class HttpRequest:
    method: str
    url: str = field(repr=False)
    headers: Mapping[str, str] = field(default_factory=dict, repr=False)
    body: Json = field(default=None, repr=False)
    timeout: float = 10


@dataclass(frozen=True)
class HttpResult:
    status: int
    headers: Mapping[str, str]
    text: str = field(repr=False)

    @property
    def ok(self) -> bool:
        return 200 <= self.status < 300

    def json(self) -> Json:
        return decode(self.text)

    def object(self) -> JsonObject:
        return object_value(self.json())


@dataclass(frozen=True)
class DownloadArgs:
    url: str
    destination: Path
    maximum_bytes: int


class Http:
    """The opener (including proxy/TLS policy) is supplied by bootstrap wiring."""

    def __init__(self, opener: OpenerDirector) -> None:
        self.opener = opener

    @contextmanager
    def stream(self, args: HttpRequest) -> Iterator[HTTPResponse]:
        headers = dict(args.headers)
        payload = None
        if args.body is not None:
            payload = json.dumps(args.body).encode()
            headers.setdefault("content-type", "application/json")
        request = Request(args.url, data=payload, headers=headers, method=args.method)
        try:
            try:
                response = self.opener.open(request, timeout=args.timeout)
            except HTTPError as error:
                response = error
            with response:
                yield cast(HTTPResponse, response)
        except (URLError, OSError, HTTPException) as error:
            # URLs may contain API keys in their query. Keep the diagnostic
            # actionable without publishing the URL or an upstream error body.
            raise ToolingError(f"HTTP {args.method} connection or stream failed") from error

    def request(self, args: HttpRequest) -> HttpResult:
        with self.stream(args) as response:
            return HttpResult(
                response.status,
                {key.lower(): value for key, value in response.headers.items()},
                response.read().decode("utf-8"),
            )

    def download(self, args: DownloadArgs) -> None:
        """Stream a bounded public HTTPS download; failed transfers publish nothing."""
        if args.maximum_bytes < 1:
            raise ToolingError("Download byte limit must be positive")
        url = args.url
        for _ in range(6):
            parsed = urlsplit(url)
            if (
                parsed.scheme != "https"
                or not parsed.hostname
                or parsed.username
                or parsed.password
            ):
                raise ToolingError("Tool downloads and redirects must use credential-free HTTPS")
            with self.stream(HttpRequest("GET", url, timeout=60)) as response:
                if response.status in (301, 302, 303, 307, 308):
                    location = response.headers.get("Location")
                    if not location:
                        raise ToolingError("Tool download redirect has no location")
                    url = urljoin(url, location)
                    continue
                if response.status != 200:
                    raise ToolingError(f"Tool download returned HTTP {response.status}")
                args.destination.parent.mkdir(parents=True, exist_ok=True)
                with tempfile.NamedTemporaryFile(
                    dir=args.destination.parent, delete=False
                ) as stream:
                    temporary = Path(stream.name)
                    try:
                        size = 0
                        while chunk := response.read(128 * 1024):
                            size += len(chunk)
                            if size > args.maximum_bytes:
                                raise ToolingError("Tool download exceeded its byte limit")
                            stream.write(chunk)
                        if size == 0:
                            raise ToolingError("Tool download is empty")
                        stream.flush()
                        temporary.replace(args.destination)
                    finally:
                        temporary.unlink(missing_ok=True)
                return
        raise ToolingError("Tool download exceeded its redirect limit")
