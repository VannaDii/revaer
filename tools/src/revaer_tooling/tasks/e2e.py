"""Own the browser suite's database, services, ordered phases, and evidence.

The outer task keeps resources alive across pytest invocations. Anonymous API,
authenticated API, and UI phases share one disposable database in that order.
Every process is reaped before that database is dropped, including on failure.
"""

import html
import json
import time
import tomllib
import uuid
from collections.abc import Iterator
from contextlib import ExitStack, contextmanager
from dataclasses import dataclass, replace
from pathlib import Path
from urllib.parse import urlsplit

from ..context import Context, TaskResult
from ..e2e.api import ApiClient, ApiRequest, ApiSchema, ApiSession, Method, configure_auth
from ..e2e.coverage import RouteCoverage, required_api_operations, verify_routes
from ..e2e.database import single_init_database, uses_single_init
from ..e2e.shards import verify_shards
from ..errors import ToolingError
from ..external.database import with_database
from ..external.http import HttpRequest
from ..external.python import E2ePytestArgs
from ..external.rust import TrunkServeArgs
from ..external.serving import ApplicationArgs, ServingExecutable, ServingKind
from ..json_data import decode, object_value
from ..process import RunningProcess
from .base import Task
from .build import SyncAssets
from .database import database_connection


@contextmanager
def temporary_database(context: Context) -> Iterator[str]:
    if uses_single_init(context):
        with single_init_database(context) as url:
            yield url
        return
    settings = context.settings.e2e
    # A caller-supplied server is used as supplied. Only the default managed
    # server goes through Docker ownership; neither path resets an existing DB.
    configured = context.settings.database
    supplied = settings.admin_url
    database = replace(
        configured,
        url=with_database(supplied or configured.url, "postgres"),
        managed=supplied is None and not settings.skip_database_start,
        reset=False,
    )
    owner = replace(context, settings=replace(context.settings, database=database))
    with database_connection(owner) as admin_url:
        name = settings.database_prefix + "_" + uuid.uuid4().hex
        url = with_database(admin_url, name)
        context.tools.sqlx.create_database(url)
        try:
            context.tools.sqlx.migrate(url, context.root / "crates/revaer-data/migrations")
            yield url
        finally:
            context.tools.sqlx.drop_database(url)


def wait_for_service(context: Context, process: RunningProcess, url: str, log: Path) -> None:
    settings = context.settings.e2e
    for attempt in range(settings.startup_attempts):
        if process.poll() is not None:
            raise ToolingError(f"E2E service exited before becoming ready; inspect {log}")
        try:
            response = context.tools.http.request(HttpRequest("GET", url, timeout=1))
        except ToolingError as error:
            # Connection refusal is expected while a newly spawned service binds.
            # Preserve the last error as the cause if startup never succeeds.
            if attempt + 1 == settings.startup_attempts:
                raise ToolingError(f"E2E service startup timed out; inspect {log}") from error
        else:
            if response.ok:
                return
        if attempt + 1 < settings.startup_attempts:
            time.sleep(settings.startup_interval_ms / 1000)
    raise ToolingError(f"E2E service startup timed out; inspect {log}")


def defaults(context: Context) -> dict[str, object]:
    return tomllib.loads(context.fs.read(context.root / "tests/e2e.toml"))


def serving_kind(context: Context) -> ServingKind:
    configuration = defaults(context).get("api")
    if not isinstance(configuration, dict):
        raise ToolingError("tests/e2e.toml must specify [api].runner")
    key = "media_runner" if uses_single_init(context) else "runner"
    value = context.settings.e2e.api_runner or configuration.get(key)
    if not isinstance(value, str):
        raise ToolingError(f"tests/e2e.toml must specify [api].{key}")
    return ServingKind(value)


@dataclass(frozen=True)
class RunPaths:
    results: Path
    report: Path
    logs: Path
    runtime: Path
    filesystem: Path

    @staticmethod
    def for_context(context: Context) -> "RunPaths":
        root = context.root
        filesystem = context.settings.e2e.filesystem_root or root
        if not filesystem.is_absolute():
            filesystem = root / filesystem
        return RunPaths(
            root / "tests/test-results",
            root / "tests/playwright-report",
            root / "tests/logs",
            root / "tests/.runtime",
            filesystem.resolve(),
        )


def phase_arguments(context: Context, paths: RunPaths, phase: str) -> E2ePytestArgs:
    name = phase + context.settings.e2e.shard_suffix
    return E2ePytestArgs(
        phase,
        context.settings.e2e,
        paths.results / name,
        paths.report / name,
        context.root / "coverage/e2e" / name,
        include_media=uses_single_init(context),
    )


def prepare_outputs(context: Context, paths: RunPaths, phases: tuple[str, ...]) -> None:
    for directory in (paths.results, paths.report, paths.logs):
        if directory.is_symlink():
            raise ToolingError("E2E output directories must not be symlinks")
        context.fs.mkdir(directory)
    for prefix in ("api-coverage-", "ui-coverage-", "selection-", "javascript-"):
        for path in paths.results.glob(prefix + "*.json"):
            context.fs.remove_owned(path, context.root)
    for phase in phases:
        args = phase_arguments(context, paths, phase)
        for path in (args.results, args.report, args.coverage):
            context.fs.remove_owned(path, context.root)
            context.fs.mkdir(path)
    if context.settings.e2e.browser_coverage:
        sources = [
            path
            for path in context.tools.git.files(include_untracked=True)
            if path.endswith((".js", ".mjs", ".cjs")) and (context.root / path).is_file()
        ]
        context.fs.write(paths.results / "javascript-sources.json", json.dumps(sources) + "\n")


def write_summary(
    context: Context,
    paths: RunPaths,
    status: str,
    outcomes: dict[str, str],
) -> None:
    suffix = context.settings.e2e.shard_suffix
    rows = "".join(
        f'<li><a href="{phase}{suffix}/index.html">{phase}</a>: {outcome}</li>'
        for phase, outcome in outcomes.items()
    )
    context.fs.write(
        paths.report / "index.html",
        (
            "<!doctype html><meta charset=utf-8><title>Revaer E2E</title>"
            f"<h1>Revaer E2E: {status}</h1><ul>" + rows + "</ul>"
            '<p><a href="../test-results/">Test artifacts</a> · '
            '<a href="../logs/">Service logs</a></p>'
            f"<p>Checkout: {html.escape(str(context.root))}</p>"
        ),
    )
    context.fs.write(
        paths.results / f"python-e2e-summary{suffix}.json",
        json.dumps(
            {
                "status": status,
                "phases": outcomes,
                "shard": context.settings.e2e.shard_index,
                "total_shards": context.settings.e2e.shard_total,
                "browser_coverage": context.settings.e2e.browser_coverage,
            },
            indent=2,
        )
        + "\n",
    )


@contextmanager
def running_services(
    context: Context,
    paths: RunPaths,
    executable: ServingExecutable,
    database_url: str,
    ui_required: bool,
    media_root_catalog: Path | None = None,
) -> Iterator[tuple[RunningProcess, ...]]:
    settings = context.settings.e2e
    with ExitStack() as cleanup:
        api_log = paths.logs / "api.log"
        api = context.tools.application.start(
            ApplicationArgs(
                executable,
                database_url,
                paths.filesystem / ".media-workspace",
                api_log,
                media_root_catalog,
            )
        )
        cleanup.callback(api.stop)
        wait_for_service(context, api, settings.api_url + "/health", api_log)
        # Check ownership before any destructive setup API call. A successful
        # health response alone does not identify the process serving the port.
        context.tools.listeners.require_owner(urlsplit(settings.api_url).port or 80, api.pid)
        services = [api]
        if ui_required:
            ui_log = paths.logs / "ui.log"
            port = urlsplit(settings.ui_url).port or 80
            ui = context.tools.trunk.start(TrunkServeArgs(port, settings.api_url, ui_log))
            cleanup.callback(ui.stop)
            wait_for_service(context, ui, settings.ui_url, ui_log)
            context.tools.listeners.require_owner(port, ui.pid)
            services.append(ui)
        yield tuple(services)


def run_phases(
    context: Context,
    paths: RunPaths,
    phases: tuple[str, ...],
    services: tuple[RunningProcess, ...],
    outcomes: dict[str, str],
    initial_session: ApiSession | None = None,
) -> None:
    settings = context.settings.e2e
    schema = ApiSchema(
        object_value(decode(context.fs.read(context.root / "docs/api/openapi.json")))
    )
    session = initial_session
    for phase in phases:
        outcomes[phase] = "failed"
        mode = "none" if phase.startswith("api-none") else "api_key"
        if session is None or session.auth_mode != mode:
            coverage = RouteCoverage(
                paths.results / f"api-coverage-{phase}{settings.shard_suffix}-setup.json",
                context.fs,
            )
            client = ApiClient(context.tools.http, settings.api_url, coverage, schema)
            session = configure_auth(client, mode, str(paths.filesystem))
        context.emit(f"E2E phase: {phase}")
        context.tools.python.e2e(
            phase_arguments(context, paths, phase),
            {
                "REVAER_E2E_API_KEY": session.api_key or "",
                "E2E_FS_ROOT": str(paths.filesystem),
                "REVAER_E2E_RESULTS_ROOT": str(paths.results),
            },
        )
        if any(service.poll() is not None for service in services):
            raise ToolingError(f"An E2E service exited during {phase}; inspect {paths.logs}")
        outcomes[phase] = "passed"


def run_media_phases(
    context: Context,
    paths: RunPaths,
    phases: tuple[str, ...],
    executable: ServingExecutable,
    database_url: str,
    outcomes: dict[str, str],
) -> None:
    """Join each old service before loading a different startup catalog.

    Authentication setup resets public tables, including the catalog. Run it
    against the absent-catalog service, then restart to attest the configured
    catalog. Never fabricate a generation or add runtime catalog reloading.
    The database retains authentication across restarts, so resets use the
    previous session before replacing it with newly issued credentials.
    """
    missing = paths.runtime / "missing-media-roots.json"
    if missing.exists() or missing.is_symlink():
        raise ToolingError("The missing-catalog fixture path must be absent")
    settings = context.settings.e2e
    schema = ApiSchema(
        object_value(decode(context.fs.read(context.root / "docs/api/openapi.json")))
    )
    session: ApiSession | None = None
    for phase in phases:
        outcomes[phase] = "failed"
        selected_paths = replace(paths, logs=paths.logs / phase)
        ui = phase.startswith("ui-")
        coverage = RouteCoverage(
            paths.results / f"api-coverage-{phase}{settings.shard_suffix}-setup.json",
            context.fs,
        )
        if not ui or session is None:
            with running_services(
                context,
                replace(selected_paths, logs=selected_paths.logs / "setup"),
                executable,
                database_url,
                False,
                missing,
            ):
                client = ApiClient(
                    context.tools.http,
                    settings.api_url,
                    coverage,
                    schema,
                    session.headers() if session is not None else None,
                )
                mode = "none" if phase.startswith("api-none") else "api_key"
                session = configure_auth(client, mode, str(paths.filesystem))
        context.tools.listeners.require_free(urlsplit(settings.api_url).port or 80)
        with running_services(
            context,
            selected_paths,
            executable,
            database_url,
            ui,
            missing if phase.endswith("-missing-catalog") else None,
        ) as services:
            if session is None:
                raise ToolingError("Media E2E authentication setup did not return a session")
            if not phase.endswith("-missing-catalog"):
                client = ApiClient(
                    context.tools.http, settings.api_url, coverage, schema, session.headers()
                )
                readiness = client.request(
                    ApiRequest(Method.GET, "/v1/media/root-catalog/readiness")
                ).object()
                if (
                    readiness.get("source_state") != "ready"
                    or readiness.get("attestation_state") != "ready"
                ):
                    raise ToolingError(
                        "Media E2E requires a real active Linux root catalog; inspect "
                        + str(selected_paths.logs)
                    )
            run_phases(context, paths, (phase,), services, outcomes, session)


def run_suite(context: Context, paths: RunPaths) -> None:
    """Run while the calling task holds the checkout's E2E lock.

    Runbook keeps that same lock through artifact copying, so another invocation
    cannot replace results between suite completion and archiving.
    """
    settings = context.settings.e2e
    media = uses_single_init(context)
    phases = settings.phases(media=media)
    api_port = urlsplit(settings.api_url).port or 80
    ui_required = any(phase.startswith("ui-") for phase in phases)
    prepare_outputs(context, paths, phases)
    outcomes = dict.fromkeys(phases, "not run")
    write_summary(context, paths, "running", outcomes)
    status = "failed"
    try:
        kind = serving_kind(context)
        if api_port != 7070:
            raise ToolingError("The application's bootstrap listener uses port 7070")
        context.tools.listeners.require_free(api_port)
        if ui_required:
            context.tools.listeners.require_free(urlsplit(settings.ui_url).port or 80)
        executable = context.tools.cargo.serving_executable(kind)
        if ui_required:
            SyncAssets.run(context)
            context.fs.mkdir(context.root / "crates/revaer-ui/dist-serve/.stage")
        context.fs.mkdir(paths.filesystem)
        with temporary_database(context) as database_url:
            if media:
                run_media_phases(context, paths, phases, executable, database_url, outcomes)
            else:
                with running_services(
                    context, paths, executable, database_url, ui_required
                ) as services:
                    run_phases(context, paths, phases, services, outcomes)
        status = "passed"
    finally:
        write_summary(context, paths, status, outcomes)


class UiE2e(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        paths = RunPaths.for_context(context)
        context.fs.mkdir(paths.runtime)
        with context.fs.lock(paths.runtime / "e2e.lock"):
            run_suite(context, paths)
        return TaskResult("E2E phases passed; report: tests/playwright-report/index.html")


class UiE2eCoverage(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        directory = context.settings.e2e.coverage_directory or context.root / "tests/test-results"
        if not directory.is_absolute():
            directory = context.root / directory
        required = required_api_operations(context.fs.read(context.root / "docs/api/openapi.json"))
        verify_routes(context.fs, directory, "API", required)
        configuration = defaults(context).get("ui")
        routes = configuration.get("required_routes") if isinstance(configuration, dict) else None
        if (
            not isinstance(routes, list)
            or not routes
            or not all(isinstance(route, str) for route in routes)
        ):
            raise ToolingError("tests/e2e.toml must list required UI routes")
        verify_routes(context.fs, directory, "UI", routes)
        return TaskResult("API operations and required UI routes are covered")


class UiE2eShardCoverage(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        directory = context.settings.e2e.coverage_directory or context.root / "tests/test-results"
        if not directory.is_absolute():
            directory = context.root / directory
        verify_shards(
            context.fs,
            directory,
            context.settings.e2e.phases(media=uses_single_init(context)),
            context.options.expected_shards,
        )
        return TaskResult("Every E2E shard completed its expected scenarios and recorded coverage")


def archive_runbook(context: Context, paths: RunPaths, status: str) -> None:
    destination = context.root / "artifacts/runbook"
    if destination.is_symlink():
        raise ToolingError("Runbook output must not be a symlink")
    context.fs.mkdir(destination)
    # Invalidate yesterday's success before any fallible copy. Summary is the
    # commit marker for a complete archive, including a completed failure archive.
    context.fs.remove_owned(destination / "summary.txt", context.root)
    for source in (paths.logs, paths.report, paths.results):
        target = destination / source.name
        context.fs.remove_owned(target, context.root)
        if source.exists():
            context.fs.copy_tree(source, target)
    # The retired runner kept its plaintext session in this file. It is never
    # part of a runbook, even if a prior legacy run left it in test-results.
    context.fs.remove_owned(destination / "test-results/e2e-state.json", context.root)
    context.fs.write(
        destination / "summary.txt",
        f"runbook={status}\nartifacts=artifacts/runbook\n"
        "playwright_report=artifacts/runbook/playwright-report/index.html\n"
        "test_results=artifacts/runbook/test-results\nlogs=artifacts/runbook/logs\n",
    )


class Runbook(Task):
    @staticmethod
    def run(context: Context) -> TaskResult:
        paths = RunPaths.for_context(context)
        context.fs.mkdir(paths.runtime)
        with context.fs.lock(paths.runtime / "e2e.lock"):
            status = "failed"
            try:
                run_suite(context, paths)
                # Runbook verifies its own run, regardless of an aggregation
                # directory a caller may have set for separate CI shard checks.
                settings = replace(context.settings.e2e, coverage_directory=paths.results)
                UiE2eCoverage.run(
                    replace(context, settings=replace(context.settings, e2e=settings))
                )
                status = "ok"
            finally:
                archive_runbook(context, paths, status)
        return TaskResult("Runbook artifacts: artifacts/runbook")
