"""Scheduling prerequisites for RSS tests in the coordinator-owned database."""

import os
import re
import tempfile
from pathlib import Path
from uuid import UUID

from revaer_tooling.e2e.api import ApiClient, ApiRequest, Method
from revaer_tooling.external.database import LibpqConnection, Psql, with_database
from revaer_tooling.json_data import JsonObject, array_value, decode, object_value
from revaer_tooling.process import Invocation, ProcessRunner, stderr_message

from tests.conftest import E2eRuntime


def make_due(runtime: E2eRuntime, identifier: str, *, due: bool = True) -> None:
    name = os.environ.get("REVAER_E2E_DATABASE_NAME", "")
    assert re.fullmatch(r"revaer_test_\d+_\d+", name, flags=re.ASCII), (
        "RSS requires an owned E2E database"
    )
    admin = runtime.settings.admin_url
    assert admin is not None
    identifier = str(UUID(identifier))
    offset = "- interval '1 second'" if due else "+ interval '10 minutes'"
    sql = f"""
DO $$ BEGIN
    IF current_database() <> '{name}' OR NOT EXISTS (
        SELECT 1 FROM pg_database d JOIN pg_roles r ON r.oid = d.datdba
        WHERE d.datname = current_database() AND r.rolname = current_database() || '_owner'
    ) THEN RAISE EXCEPTION 'RSS fixture requires the owned disposable database'; END IF;
END $$;
DO $$ BEGIN
    FOR attempt IN 1..500 LOOP
        IF NOT EXISTS (SELECT 1 FROM job_schedule
            WHERE job_key = 'rss_poll' AND locked_until > clock_timestamp()) THEN RETURN; END IF;
        PERFORM pg_sleep(0.01);
    END LOOP;
    RAISE EXCEPTION 'The owned RSS job did not settle';
END $$;
UPDATE indexer_rss_subscription SET next_poll_at = clock_timestamp() {offset}
WHERE is_enabled AND indexer_instance_id = (
    SELECT indexer_instance_id FROM indexer_instance
    WHERE indexer_instance_public_id = '{identifier}'::uuid
);
UPDATE job_schedule SET next_run_at = clock_timestamp() - interval '1 second'
WHERE job_key = 'rss_poll';
"""
    tool = Psql("psql", ProcessRunner(stderr_message), runtime.root, os.environ)
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".sql", dir=runtime.root / ".test-tmp"
    ) as source:
        source.write(sql)
        source.flush()
        tool.seed(with_database(admin, name), Path(source.name))


RSS_ROUTE = "/v1/indexers/instances/{indexer_instance_public_id}/rss"


def discovery_state(runtime: E2eRuntime, identifier: str) -> JsonObject:
    """Observe side effects only in the coordinator-owned disposable database."""
    name = os.environ.get("REVAER_E2E_DATABASE_NAME", "")
    assert re.fullmatch(r"revaer_test_\d+_\d+", name, flags=re.ASCII)
    admin = runtime.settings.admin_url
    assert admin is not None
    identifier = str(UUID(identifier))
    sql = f"""
DO $$ BEGIN
    IF current_database() <> '{name}' OR NOT EXISTS (
        SELECT 1 FROM pg_database d JOIN pg_roles r ON r.oid = d.datdba
        WHERE d.datname = current_database() AND r.rolname = current_database() || '_owner'
    ) THEN RAISE EXCEPTION 'RSS observation requires the owned disposable database'; END IF;
END $$;
SELECT json_build_object(
    'search_requests', (SELECT count(*) FROM search_request),
    'canonical_results', (SELECT count(*) FROM canonical_torrent),
    'rss_requests', count(*) FILTER (WHERE log.request_type = 'rss'),
    'unexpected_requests', count(*) FILTER (
        WHERE log.request_type <> 'rss' OR log.search_request_id IS NOT NULL)
)
FROM outbound_request_log log JOIN indexer_instance inst USING (indexer_instance_id)
WHERE inst.indexer_instance_public_id = '{identifier}'::uuid;
"""
    connection = LibpqConnection.from_url(with_database(admin, name))
    result = ProcessRunner(stderr_message).run(
        Invocation(
            (
                "psql",
                "--no-psqlrc",
                "--no-password",
                "--quiet",
                "--tuples-only",
                "--no-align",
                "--set=ON_ERROR_STOP=1",
                "--single-transaction",
                "--dbname",
                connection.uri,
                "--command",
                sql,
            ),
            runtime.root,
            {**os.environ, **connection.environment()},
            capture=True,
        )
    )
    return object_value(decode(result.stdout.strip()))


def enable_rss(api: ApiClient, identifier: str) -> None:
    path = {"indexer_instance_public_id": identifier}
    assert api.request(
        ApiRequest(
            Method.PATCH,
            "/v1/indexers/instances/{indexer_instance_public_id}",
            {**path, "enable_rss": True},
            path=path,
        )
    ).ok
    assert api.request(
        ApiRequest(
            Method.PUT,
            RSS_ROUTE,
            {"is_enabled": True, "interval_seconds": 300},
            path=path,
        )
    ).ok


def status(api: ApiClient, identifier: str) -> JsonObject:
    response = api.request(
        ApiRequest(
            Method.GET,
            RSS_ROUTE,
            path={"indexer_instance_public_id": identifier},
        )
    )
    assert response.ok
    return response.object()


def items(api: ApiClient, identifier: str) -> list[JsonObject]:
    response = api.request(
        ApiRequest(
            Method.GET,
            RSS_ROUTE + "/items",
            path={"indexer_instance_public_id": identifier},
        )
    )
    assert response.ok
    return [object_value(value) for value in array_value(response.object()["items"])]
