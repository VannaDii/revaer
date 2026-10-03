"""Native mutation proofs for every catalog class and constrained-owner boundary."""

import itertools
import json
from collections.abc import Iterator
from dataclasses import dataclass, replace
from pathlib import Path

import pytest
from revaer_tooling.cli import COMMANDS, make_context
from revaer_tooling.context import Options
from revaer_tooling.database.postgres import Connection
from revaer_tooling.database.pristine.query import Query
from revaer_tooling.database.pristine.snapshot import Snapshot, records, validate_row
from revaer_tooling.database.pristine.spec import Specification
from revaer_tooling.database.pristine.workspace import Workspace, provision
from revaer_tooling.errors import ToolingError
from revaer_tooling.external.postgres import QueryArgs
from revaer_tooling.filesystem import FileSystem
from revaer_tooling.json_data import JsonObject, array_value, decode_unique, object_value
from revaer_tooling.tasks.pristine import PristineGenerate, PristineValidate

ROOT = Path(__file__).parents[2]
SPEC = ROOT / "tools/src/revaer_tooling/database/pristine/spec.json"


def cases() -> dict[str, tuple[str, str]]:
    path = Path(__file__).parent / "fixtures/pristine/cases.json"
    result = {}
    for catalog, value in object_value(decode_unique(path.read_text())).items():
        pair = array_value(value)
        if len(pair) != 2 or not all(isinstance(item, str) for item in pair):
            raise ValueError("Invalid catalog mutation fixture")
        setup, mutation = pair
        if not isinstance(setup, str) or not isinstance(mutation, str) or not mutation:
            raise ValueError("Invalid catalog mutation SQL")
        result[catalog] = (setup, mutation)
    return result


CASES = cases()


@dataclass(frozen=True)
class CatalogServer:
    connection: Connection
    spec: Specification
    counter: Iterator[int]

    def fresh(self) -> Snapshot:
        database, owner = provision(self.connection, f"{next(self.counter):x}")
        return Snapshot(self.connection, database, owner, self.spec)

    def execute(self, snapshot: Snapshot, source: str) -> None:
        if source:
            self.connection.sql(source + ";", database=snapshot.database)


@pytest.fixture(scope="module")
def catalogs(tmp_path_factory: pytest.TempPathFactory) -> Iterator[CatalogServer]:
    context = make_context(Options())
    root = tmp_path_factory.mktemp("pristine-catalogs")
    context.fs.write_bytes(
        root / ".github/build-inputs.env", (ROOT / ".github/build-inputs.env").read_bytes()
    )
    workspace = Workspace.load(root, context.fs)
    spec = Specification.load(context.fs, SPEC)
    assert CASES.keys() == spec.columns.keys()
    native = context.tools.proof_databases
    native.docker.verify_image(workspace.postgres)
    with native.open(
        workspace, user="postgres", database="postgres", logical_wal=True
    ) as connection:
        yield CatalogServer(connection, spec, itertools.count(1))


def catalog_rows(source: bytes, catalog: str) -> list[bytes]:
    return [line for line in source.splitlines() if line.startswith(catalog.encode() + b"\t")]


@pytest.mark.parametrize("catalog", CASES)
def test_every_native_catalog_mutation_is_visible_or_denied(
    catalogs: CatalogServer,
    catalog: str,
) -> None:
    snapshot = catalogs.fresh()
    setup, mutation = CASES[catalog]
    catalogs.execute(snapshot, setup)
    before = snapshot.read()
    try:
        if catalog == "pg_subscription":
            # This deliberately disconnected fixture has one expected diagnostic.
            # Assert it exactly; normal catalog reads retain fail-on-warning behavior.
            result = catalogs.connection.docker.query(
                catalogs.connection.container,
                QueryArgs(
                    "postgres", snapshot.database, "\\set VERBOSITY default\n" + mutation + ";"
                ),
            )
            assert (result.code, result.stdout, result.stderr) == (
                0,
                "",
                "WARNING:  subscription was created, but is not connected\n"
                "HINT:  To initiate replication, you must manually create the replication slot, "
                "enable the subscription, and refresh the subscription.\n",
            )
        else:
            catalogs.execute(snapshot, mutation)
        if catalog in ("pg_user_mapping", "pg_subscription"):
            with pytest.raises(ToolingError, match="SQLSTATE=42501"):
                snapshot.read()
        else:
            assert catalog_rows(snapshot.read(), catalog) != catalog_rows(before, catalog)
    finally:
        if catalog == "pg_subscription":
            catalogs.execute(snapshot, "DROP SUBSCRIPTION IF EXISTS fixture_subscription")


def test_native_identity_statistics_and_temporary_relations_normalize(
    catalogs: CatalogServer,
) -> None:
    snapshot, other = catalogs.fresh(), catalogs.fresh()
    baseline = snapshot.read()
    assert baseline == other.read() == (ROOT / "config/postgres-pristine-16.14.tsv").read_bytes()
    assert baseline.decode("utf-8").encode("utf-8") == baseline
    assert baseline.endswith(b"\n") and b"\r" not in baseline
    assert baseline.splitlines() == sorted(baseline.splitlines())
    assert sum(line.startswith(b"#catalog\t") for line in baseline.splitlines()) == 27
    assert snapshot.database.encode() not in baseline and snapshot.owner.encode() not in baseline
    catalogs.execute(
        snapshot,
        "UPDATE pg_class SET relpages=relpages+100, reltuples=reltuples+50, "
        "relallvisible=relallvisible+5 WHERE oid='pg_catalog.pg_class'::regclass",
    )
    assert snapshot.read() == baseline
    assert (
        snapshot.read("CREATE TEMPORARY TABLE fixture_temp (id integer, payload text);") == baseline
    )
    with pytest.raises(ToolingError, match="owner identity"):
        Snapshot(catalogs.connection, snapshot.database, "postgres", catalogs.spec).read()


def test_reallocated_object_and_toast_identities_remain_stable(catalogs: CatalogServer) -> None:
    snapshot = catalogs.fresh()
    ddl = (
        "CREATE TABLE public.fixture_toast (id integer, payload text); "
        "CREATE FUNCTION public.fixture_function(value integer DEFAULT 7) "
        "RETURNS integer LANGUAGE SQL RETURN value + 1;"
    )
    catalogs.connection.sql(ddl, database=snapshot.database, role=snapshot.owner)
    initial = snapshot.read()
    catalogs.connection.sql(
        "DROP TABLE public.fixture_toast; DROP FUNCTION public.fixture_function(integer); " + ddl,
        database=snapshot.database,
        role=snapshot.owner,
    )
    assert snapshot.read() == initial


SECURITY_CASES = (
    ("pg_proc", "ALTER FUNCTION public.fixture_function(integer) SET search_path=public"),
    ("pg_proc", "ALTER FUNCTION public.fixture_function(integer) LEAKPROOF"),
    ("pg_proc", "REVOKE ALL ON FUNCTION public.fixture_function(integer) FROM PUBLIC"),
    (
        "pg_proc",
        "CREATE OR REPLACE FUNCTION public.fixture_function(value integer DEFAULT 9) "
        "RETURNS integer LANGUAGE SQL RETURN value + 2",
    ),
    ("pg_class", "GRANT SELECT(id) ON public.fixture_table TO PUBLIC"),
    ("pg_class", "ALTER TABLE public.fixture_table ALTER COLUMN id SET NOT NULL"),
    ("pg_class", "ALTER TABLE public.fixture_table FORCE ROW LEVEL SECURITY"),
    (
        "pg_ts_config",
        "ALTER TEXT SEARCH CONFIGURATION public.fixture_config DROP MAPPING FOR asciiword",
    ),
)


@pytest.mark.parametrize("catalog,mutation", SECURITY_CASES)
def test_native_security_and_definition_mutations_change_evidence(
    catalogs: CatalogServer,
    catalog: str,
    mutation: str,
) -> None:
    snapshot = catalogs.fresh()
    catalogs.execute(
        snapshot,
        "CREATE TABLE public.fixture_table (id integer); "
        "CREATE FUNCTION public.fixture_function(value integer DEFAULT 7) "
        "RETURNS integer LANGUAGE SQL RETURN value+1; "
        "CREATE TEXT SEARCH CONFIGURATION public.fixture_config (COPY=pg_catalog.simple)",
    )
    before = snapshot.read()
    catalogs.execute(snapshot, mutation)
    assert catalog_rows(snapshot.read(), catalog) != catalog_rows(before, catalog)


def test_every_native_column_is_accounted_for_and_drift_fails(catalogs: CatalogServer) -> None:
    snapshot = catalogs.fresh()
    query = Query(catalogs.spec)
    for catalog, columns in snapshot.inventory.items():
        expected = sum(name != "oid" and name not in catalogs.spec.excluded for name, _ in columns)
        assert len(query.projection(catalog, columns)) == expected
        for changed in ((*columns, ("unexpected", "text")), columns[1:]):
            with pytest.raises(ToolingError, match="inventory drift"):
                query.projection(catalog, changed)
    for column, kind in (
        ("nspname", "unsupported"),
        ("unexpected", "pg_node_tree"),
        ("unexpected", "oid"),
    ):
        with pytest.raises(ToolingError, match="Unaccounted catalog"):
            query.expression(column, kind)


@pytest.mark.parametrize(
    "value",
    (
        '{"field":null,"\\u0066ield":true}',
        '{"nested":{"x":1,"x":2}}',
        '{"value":NaN}',
        '{"value":Infinity}',
        '{"value":1e400}',
        "[]",
        "invalid",
    ),
)
def test_ambiguous_or_invalid_catalog_transport_fails(value: str) -> None:
    with pytest.raises(ToolingError):
        records(value)


def test_unique_json_preserves_finite_numbers() -> None:
    assert decode_unique('{"number":1.25e2,"zero":0.0}') == {"number": 125.0, "zero": 0.0}


@pytest.mark.parametrize(
    "row",
    (
        {"value": "<unresolved_reference>"},
        {"probin": "/tmp/library"},
        {"probin": 1},
        {"value": "pg_temp_42"},
        {"value": "pg_toast_24"},
    ),
)
def test_unresolved_or_physical_catalog_identity_is_rejected(row: JsonObject) -> None:
    with pytest.raises(ToolingError):
        validate_row(row)


def test_specification_rejects_unknown_missing_and_duplicate_columns(tmp_path: Path) -> None:
    document = json.loads(SPEC.read_text())
    path = tmp_path / "spec.json"
    for changed in (
        {**document, "unknown": {}},
        {**document, "columns": {}},
        {**document, "columns": {**document["columns"], "pg_namespace": ["oid", "oid"]}},
    ):
        path.write_text(json.dumps(changed))
        with pytest.raises(ToolingError):
            Specification.load(FileSystem(), path)


def test_commands_validate_exact_bytes_and_invalidate_stale_success_on_failure(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    context = replace(make_context(Options()), root=tmp_path)
    context.fs.write_bytes(
        tmp_path / ".github/build-inputs.env", (ROOT / ".github/build-inputs.env").read_bytes()
    )
    expected = tmp_path / "config/postgres-pristine-16.14.tsv"
    baseline = (ROOT / "config/postgres-pristine-16.14.tsv").read_bytes()
    context.fs.write_bytes(expected, baseline)
    assert COMMANDS["db-pristine-catalog-generate"] is PristineGenerate.run
    assert COMMANDS["db-pristine-catalog-validate"] is PristineValidate.run
    assert "validated: 8456 lines" in COMMANDS["db-pristine-catalog-validate"](context).message
    output = Workspace.load(tmp_path, context.fs).output
    generated, evidence = output / expected.name, output / "provenance.json"
    assert generated.read_bytes() == expected.read_bytes() == baseline
    assert json.loads(evidence.read_text())["catalog_count"] == 27
    assert generated.stat().st_mode & 0o777 == evidence.stat().st_mode & 0o777 == 0o600

    expected.write_bytes(baseline + b"unexpected\n")
    with pytest.raises(ToolingError, match="Committed pristine snapshot differs"):
        PristineValidate.run(context)
    assert expected.read_bytes() == baseline + b"unexpected\n"
    assert generated.read_bytes() == baseline and evidence.is_file()

    def fail_read(snapshot: Snapshot, prefix: str = "") -> bytes:
        raise ToolingError("Interrupted catalog read")

    monkeypatch.setattr(Snapshot, "read", fail_read)
    with pytest.raises(ToolingError, match="Interrupted catalog read"):
        PristineGenerate.run(context)
    assert not generated.exists() and not evidence.exists()
    receipts = [json.loads(path.read_text()) for path in output.glob("postgres-*.json")]
    assert len(receipts) == 3
    assert sum(item["completed"] is True for item in receipts) == 2
    assert all(item["storage_removed"] is True for item in receipts)
