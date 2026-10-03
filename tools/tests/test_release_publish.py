"""Publish through the real CLIs to a disposable Git remote and local HTTPS API.

The API fixture implements GitHub release/asset responses, including failures and
server digests. It has no route to GitHub, credentials, or a real repository. This
checks orchestration across process boundaries without publishing test releases.
"""

import hashlib
import json
import ssl
import subprocess
import tempfile
import threading
from collections.abc import Iterator
from dataclasses import dataclass, field, replace
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from socketserver import ThreadingMixIn, UnixStreamServer
from urllib.parse import parse_qs, urlsplit

import pytest
from revaer_tooling.artifacts import APPLICATION_ARTIFACTS, MANIFEST, record_artifacts
from revaer_tooling.cli import make_context
from revaer_tooling.context import Context, Options
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.release import parse_outputs
from revaer_tooling.tasks.automation import WorkflowChartVersions
from revaer_tooling.tasks.release import ReleasePreview, ReleasePublish, ReleaseResume


def git(root: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(root), *arguments],
        capture_output=True,
        text=True,
        check=True,
        timeout=20,
    ).stdout.strip()


@dataclass
class ReleaseService:
    """Small remote ledger with observable creation, transfer and digest failures."""

    url: str = ""
    release: dict[str, object] = field(default_factory=dict)
    assets: dict[int, tuple[str, bytes]] = field(default_factory=dict)
    sequence: int = 0
    source: str = ""
    fail_creation: bool = False
    fail_upload: bool = False
    corrupt_digest: bool = False
    requests: list[str] = field(default_factory=list)
    pulls: list[dict[str, object]] = field(default_factory=list)
    pull_query: dict[str, list[str]] = field(default_factory=dict)
    pull_status: int = 200

    def asset_metadata(self, identifier: int) -> dict[str, object]:
        name, content = self.assets[identifier]
        digest = hashlib.sha256(content).hexdigest()
        return {
            "id": identifier,
            "name": name,
            "state": "uploaded",
            "size": len(content),
            "digest": f"sha256:{'0' * 64 if self.corrupt_digest else digest}",
            "url": f"{self.url}/api/v3/repos/example/revaer/releases/assets/{identifier}",
        }

    def metadata(self) -> dict[str, object]:
        return {
            **self.release,
            "assets": [self.asset_metadata(identifier) for identifier in self.assets],
            "upload_url": f"{self.url}/uploads{{?name,label}}",
            "html_url": (
                f"{self.url}/example/revaer/releases/tag/{self.release.get('tag_name', '')}"
            ),
        }


@pytest.fixture
def release_service(tmp_path: Path) -> Iterator[ReleaseService]:
    key, certificate = tmp_path / "tls.key", tmp_path / "tls.crt"
    subprocess.run(
        [
            "openssl",
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-keyout",
            str(key),
            "-out",
            str(certificate),
            "-days",
            "1",
            "-subj",
            "/CN=localhost",
            "-addext",
            "subjectAltName=DNS:localhost,IP:127.0.0.1",
        ],
        check=True,
        capture_output=True,
        timeout=30,
    )
    service = ReleaseService()

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, format: str, *args: object) -> None:
            # Capture access evidence without interleaving logs from server threads.
            service.requests.append(format % args)

        def respond(self, status: int, body: object) -> None:
            content = json.dumps(body).encode()
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(content)))
            try:
                self.end_headers()
                if status != 204:
                    self.wfile.write(content)
            except (BrokenPipeError, ConnectionResetError):
                # gh races its published/draft lookups and cancels the loser.
                service.requests.append(f"Client cancelled {self.path}")

        def do_GET(self) -> None:
            path = urlsplit(self.path).path
            if not path.startswith("/api/v3/repos/example/revaer/"):
                self.respond(404, {"message": "Unexpected repository"})
            elif "/git/ref" in path:
                self.respond(200, {"object": {"type": "commit", "sha": service.source}})
            elif path.endswith("/pulls"):
                service.pull_query = parse_qs(urlsplit(self.path).query)
                self.respond(service.pull_status, service.pulls)
            elif not service.release:
                self.respond(404, {"message": "Not found"})
            elif path.endswith("/assets"):
                self.respond(200, service.metadata()["assets"])
            elif path.endswith("/releases/1") or "/releases/tags/" in path:
                self.respond(200, service.metadata())
            else:
                self.respond(404, {"message": f"Unknown fixture path: {path}"})

        def do_POST(self) -> None:
            path = urlsplit(self.path)
            content = self.rfile.read(int(self.headers.get("Content-Length", "0")))
            if path.path.startswith("/api/v3/repos/") and not path.path.startswith(
                "/api/v3/repos/example/revaer/"
            ):
                self.respond(404, {"message": "Unexpected repository"})
            elif path.path.endswith("/graphql"):
                query = json.loads(content)["query"]
                result: dict[str, object] = (
                    {"ref": {"id": service.source}}
                    if "RepositoryFindRef" in query
                    else {"release": None}
                )
                self.respond(200, {"data": {"repository": result}})
            elif path.path.endswith("/releases"):
                if service.fail_creation:
                    service.fail_creation = False
                    self.respond(500, {"message": "Injected release failure"})
                elif service.release:
                    self.respond(422, {"message": "Release already exists"})
                else:
                    service.release = {"id": 1, **json.loads(content)}
                    self.respond(201, service.metadata())
            elif path.path.endswith("/releases/1"):
                service.release.update(json.loads(content))
                self.respond(200, service.metadata())
            elif path.path == "/uploads":
                name = parse_qs(path.query)["name"][0]
                if service.fail_upload and name == "revaer-app":
                    self.respond(403, {"message": "Injected upload failure"})
                    return
                if any(existing == name for existing, data in service.assets.values()):
                    self.respond(422, {"message": "Asset already exists"})
                    return
                service.sequence += 1
                service.assets[service.sequence] = (name, content)
                self.respond(201, service.asset_metadata(service.sequence))
            else:
                self.respond(404, {"message": f"Unknown fixture path: {path.path}"})

        def do_DELETE(self) -> None:
            identifier = int(urlsplit(self.path).path.rsplit("/", 1)[1])
            if identifier in service.assets:
                del service.assets[identifier]
                self.respond(204, {})
            else:
                self.respond(404, {"message": "Asset missing"})

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    tls = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    tls.load_cert_chain(certificate, key)
    server.socket = tls.wrap_socket(server.socket, server_side=True)
    service.url = f"https://localhost:{server.server_port}"

    class UnixServer(ThreadingMixIn, UnixStreamServer):
        pass

    # macOS Go clients use Keychain roots, not SSL_CERT_FILE. gh explicitly
    # supports an HTTP Unix socket, which keeps this test independent of machine
    # trust settings. PSR still exercises HTTPS with the scoped test certificate.
    with tempfile.TemporaryDirectory(prefix="rv-release-api-") as temporary:
        socket = Path(temporary) / "api.sock"
        local = UnixServer(str(socket), Handler)
        gh_config = tmp_path / "gh-config"
        gh_config.mkdir()
        (gh_config / "config.yml").write_text(f"http_unix_socket: {socket}\n")
        workers = [threading.Thread(target=item.serve_forever) for item in (server, local)]
        for worker in workers:
            worker.start()
        try:
            yield service
        finally:
            for item, worker in zip((server, local), workers, strict=True):
                item.shutdown()
                worker.join(timeout=5)
                item.server_close()


@pytest.fixture
def publication(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, release_service: ReleaseService
) -> Context:
    source = Path(__file__).resolve().parents[2]
    root = tmp_path / "checkout"
    remote = tmp_path / "remote.git"
    root.mkdir()
    git(remote.parent, "init", "--bare", str(remote))
    git(root, "init", "--initial-branch=main")
    for key, value in (
        ("user.name", "Revaer tooling tests"),
        ("user.email", "tests@example.invalid"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        (f"url.{remote.as_uri()}.insteadOf", f"{release_service.url}/example/revaer.git"),
    ):
        git(root, "config", key, value)
    git(root, "remote", "add", "origin", f"{release_service.url}/example/revaer.git")
    (root / "release").mkdir()
    (root / "tools/src/revaer_tooling").mkdir(parents=True)
    (root / "tools/src/revaer_tooling/cli.py").touch()
    (root / ".gitignore").write_text("dist/\nartifacts/\nrelease/next-release.json\n")
    config = (source / "release/semantic-release.toml").read_text()
    (root / "release/semantic-release.toml").write_text(
        config.replace(
            "[semantic_release.remote]\n",
            "[semantic_release.remote]\nignore_token_for_push = true\n"
            f'url = "{release_service.url}/example/revaer.git"\n'
            f'domain = "{release_service.url}"\napi_domain = "{release_service.url}/api/v3"\n',
        )
    )
    git(root, "add", ".")
    git(root, "commit", "-m", "Initial fixture")
    git(root, "tag", "v1.2.3")
    git(root, "push", "origin", "main", "--tags")
    git(root, "commit", "--allow-empty", "-m", "fix: correct release behavior")
    release_service.source = git(root, "rev-parse", "HEAD")
    # The actual CLI clients trust only this test certificate and use a local
    # Enterprise-shaped host. Git transport is rewritten to the owned bare repo.
    for key, value in {
        "GH_HOST": release_service.url.removeprefix("https://"),
        "GH_REPO": "example/revaer",
        "GH_ENTERPRISE_TOKEN": "fixture-only-token",
        "GITHUB_TOKEN": "fixture-only-token",
        "GITHUB_REPOSITORY": "example/revaer",
        "GITHUB_ACTOR": "fixture",
        "SSL_CERT_FILE": str(tmp_path / "tls.crt"),
        "REQUESTS_CA_BUNDLE": str(tmp_path / "tls.crt"),
        "GITHUB_OUTPUT": str(tmp_path / "github-output"),
        "GH_PROMPT_DISABLED": "1",
        "GH_CONFIG_DIR": str(tmp_path / "gh-config"),
        "NO_COLOR": "1",
    }.items():
        monkeypatch.setenv(key, value)
    monkeypatch.delenv("GH_TOKEN", raising=False)
    monkeypatch.delenv("REVAER_ENABLE_HELM_RELEASE_ASSETS", raising=False)
    monkeypatch.chdir(root)
    context = make_context(Options())
    (root / "dist").mkdir()
    (root / "dist/revaer-app").write_bytes(b"application build fixture\n")
    digest = hashlib.sha256((root / "dist/revaer-app").read_bytes()).hexdigest()
    (root / "dist/revaer-app.sha256").write_text(f"{digest}  revaer-app\n")
    (root / "dist/openapi.json").write_text('{"openapi":"3.1.0"}\n')
    (root / "dist/stale-artifact").write_text("must not be uploaded")
    record_artifacts(context, git(root, "rev-parse", "HEAD"))
    return context


def test_chart_resolution_uses_real_gh_query_encoding_and_rejects_ambiguous_results(
    publication: Context,
    release_service: ReleaseService,
) -> None:
    branch = 'review/61&mode="literal"'
    release_service.pulls = [
        {
            "number": 61,
            "state": "open",
            "head": {"ref": branch, "repo": {"full_name": "example/revaer"}},
        }
    ]
    context = replace(
        publication,
        options=Options(chart_version="", app_version=""),
        settings=replace(
            publication.settings,
            workflow=replace(
                publication.settings.workflow, ref_type="branch", ref_name=branch, run_number="7"
            ),
        ),
    )
    result = json.loads(WorkflowChartVersions.run(context).message)
    assert result == {
        "pr_number": "61",
        "chart_version": "0.0.0-dev.pr61.7",
        "app_version": f"pr-61-{release_service.source[:7]}",
    }
    assert release_service.pull_query == {
        "head": [f"example:{branch}"],
        "state": ["open"],
        "per_page": ["2"],
    }
    release_service.pulls *= 2
    with pytest.raises(ToolingError, match="More than one open"):
        WorkflowChartVersions.run(context)
    release_service.pulls.clear()
    with pytest.raises(ToolingError, match="No open pull request"):
        WorkflowChartVersions.run(context)
    release_service.pull_status = 503
    with pytest.raises(ToolingError):
        WorkflowChartVersions.run(context)


def test_publication_uploads_exact_artifacts_and_reports_verified_completion(
    publication: Context,
    release_service: ReleaseService,
) -> None:
    preview = json.loads(ReleasePreview.run(publication).message)
    assert preview["version"] == "1.2.4-dev.1"
    assert "Correct release behavior" in preview["notes"]
    assert not release_service.release
    assert "verified" in ReleasePublish.run(publication).message
    names = {name for name, content in release_service.assets.values()}
    assert names == {Path(name).name for name in (*APPLICATION_ARTIFACTS, MANIFEST)}
    assert "Correct release behavior" in str(release_service.release["body"])
    output = publication.settings.workflow.output
    assert output is not None
    assert parse_outputs(output.read_text())["released"] == "true"
    assert not git(publication.root, "status", "--porcelain")
    remote = publication.root.parent / "remote.git"
    assert git(remote, "rev-parse", "v1.2.4-dev.1^{commit}") == preview["sourceCommit"]
    count = release_service.sequence
    assert "No new release" in ReleasePublish.run(publication).message
    assert release_service.sequence == count
    assert parse_outputs(output.read_text())["released"] == "false"


@pytest.mark.parametrize("failure", ["fail_creation", "fail_upload", "corrupt_digest"])
def test_partial_publication_never_reports_success_and_recovery_reuses_the_tag(
    publication: Context,
    release_service: ReleaseService,
    failure: str,
) -> None:
    setattr(release_service, failure, True)
    with pytest.raises(ToolingError):
        ReleasePublish.run(publication)
    metadata = json.loads((publication.root / "release/next-release.json").read_text())
    assert metadata["released"] is False
    output = publication.settings.workflow.output
    assert output is not None
    assert not output.exists()
    tags = git(publication.root, "tag", "--list")
    bytes_before = {name: (publication.root / name).read_bytes() for name in APPLICATION_ARTIFACTS}
    setattr(release_service, failure, False)
    resumed = replace(publication, options=Options(release_tag="v1.2.4-dev.1"))
    assert "verified" in ReleaseResume.run(resumed).message
    assert git(publication.root, "tag", "--list") == tags
    assert release_service.sequence == 4
    assert not any("DELETE " in request for request in release_service.requests)
    assert bytes_before == {name: (publication.root / name).read_bytes() for name in bytes_before}
    assert parse_outputs(output.read_text())["released"] == "true"
    assert not git(publication.root, "status", "--porcelain")


@pytest.mark.parametrize("failure", ["missing_manifest", "stale_commit", "tamper", "bad_checksum"])
def test_bad_artifacts_fail_before_remote_mutation(
    publication: Context,
    release_service: ReleaseService,
    failure: str,
) -> None:
    root = publication.root
    if failure == "missing_manifest":
        (root / MANIFEST).unlink()
    elif failure == "stale_commit":
        git(root, "commit", "--allow-empty", "-m", "fix: later source")
    elif failure == "tamper":
        (root / "dist/revaer-app").write_bytes(b"unexpected build")
    else:
        (root / "dist/revaer-app.sha256").write_text("invalid checksum\n")
        record_artifacts(publication, git(root, "rev-parse", "HEAD"))
    tags = git(root, "tag", "--list")
    with pytest.raises(ToolingError):
        ReleasePublish.run(publication)
    assert git(root, "tag", "--list") == tags
    assert not release_service.requests


def test_recovery_rejects_a_different_tag_or_source_before_remote_mutation(
    publication: Context,
    release_service: ReleaseService,
) -> None:
    for tag in (None, "v1.2.3", "--help"):
        with pytest.raises(ToolingError):
            ReleaseResume.run(replace(publication, options=Options(release_tag=tag)))
    git(publication.root, "tag", "v1.2.4-dev.1")
    git(publication.root, "commit", "--allow-empty", "-m", "Unclassified follow-up")
    with pytest.raises(ToolingError, match="original source commit"):
        ReleaseResume.run(replace(publication, options=Options(release_tag="v1.2.4-dev.1")))
    assert not release_service.requests


@pytest.mark.parametrize("conflict", ["changed_asset", "wrong_tag", "draft_release"])
def test_recovery_inspects_conflicts_before_mutating_the_release(
    publication: Context,
    release_service: ReleaseService,
    conflict: str,
) -> None:
    ReleasePublish.run(publication)
    if conflict == "changed_asset":
        identifier = next(iter(release_service.assets))
        name, content = release_service.assets[identifier]
        release_service.assets[identifier] = (name, content + b"remote modification")
    elif conflict == "wrong_tag":
        release_service.release["tag_name"] = "v9.9.9"
    else:
        release_service.release["draft"] = True
    before = len(release_service.requests)
    assets = dict(release_service.assets)
    with pytest.raises(ToolingError):
        ReleaseResume.run(replace(publication, options=Options(release_tag="v1.2.4-dev.1")))
    assert release_service.assets == assets
    assert not any(
        request.startswith(('"POST ', '"DELETE ', '"PATCH '))
        for request in release_service.requests[before:]
    )


@pytest.mark.parametrize("annotated", [False, True])
def test_stable_tag_publication_preserves_detached_workflow_and_title(
    publication: Context,
    release_service: ReleaseService,
    annotated: bool,
) -> None:
    root = publication.root
    arguments = ("-a", "-m", "Stable fixture") if annotated else ()
    git(root, "tag", *arguments, "v1.2.4")
    git(root, "push", "origin", "refs/tags/v1.2.4")
    git(root, "checkout", "--detach", "v1.2.4")
    refs = git(root, "show-ref")
    context = replace(publication, options=Options(release_tag="v1.2.4"))
    assert "verified" in ReleasePublish.run(context).message
    assert release_service.release["name"] == "Revaer v1.2.4"
    assert release_service.release.get("prerelease", False) is False
    assert git(root, "show-ref") == refs
    assert not git(root, "branch", "--show-current")
    before = release_service.sequence
    assert "verified" in ReleaseResume.run(context).message
    assert release_service.sequence == before


def test_stable_tag_requires_remote_source_match_before_publication(
    publication: Context,
    release_service: ReleaseService,
) -> None:
    root = publication.root
    remote = root.parent / "remote.git"
    git(root, "tag", "v1.2.4")
    git(root, "push", "origin", "refs/tags/v1.2.4")
    # Simulate a remote tag conflict without changing this checkout's source.
    git(remote, "update-ref", "refs/tags/v1.2.4", "v1.2.3")
    with pytest.raises(ToolingError, match="Remote release tag"):
        ReleasePublish.run(replace(publication, options=Options(release_tag="v1.2.4")))
    assert not release_service.requests
    assert not release_service.assets


@pytest.mark.parametrize("tag", ["v1.2.4-dev.1", "v01.2.4", "../outside", "--help"])
def test_explicit_stable_tag_does_not_accept_a_prerelease_or_malformed_name(
    publication: Context,
    release_service: ReleaseService,
    tag: str,
) -> None:
    with pytest.raises(ToolingError, match="Stable publication"):
        ReleasePublish.run(replace(publication, options=Options(release_tag=tag)))
    assert not release_service.requests


@pytest.mark.parametrize(
    ("key", "value"),
    [("GH_REPO", "different/repository"), ("GH_HOST", "different.example.invalid")],
)
def test_conflicting_github_destination_fails_before_publication(
    publication: Context,
    release_service: ReleaseService,
    key: str,
    value: str,
) -> None:
    from revaer_tooling.external.github import GitHub

    original = publication.tools.github
    github = GitHub(
        original.name, original.runner, original.root, {**original.environment, key: value}
    )
    context = replace(publication, tools=replace(publication.tools, github=github))
    before = git(context.root, "show-ref")
    with pytest.raises(ToolingError, match=f"{key} conflicts"):
        ReleasePublish.run(context)
    assert git(context.root, "show-ref") == before
    assert not release_service.requests
    assert not release_service.release
