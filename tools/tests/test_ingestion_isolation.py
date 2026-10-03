"""Native clone isolation and cleanup for successful and failed ingestion cases."""

from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.ingestion.evidence import IngestionEvidence
from revaer_tooling.database.ingestion.inventory import TABLES
from revaer_tooling.database.ingestion.isolated import IsolatedIngestion, Variant
from revaer_tooling.database.ingestion.sessions import ingestion_call, session
from revaer_tooling.errors import ToolingError
from test_database_contract import contract_root
from test_database_postgres import receipt

__all__ = ["contract_root"]

SEED = (
    "INSERT INTO canonical_torrent(canonical_torrent_id) VALUES (1); "
    "INSERT INTO canonical_torrent_source(canonical_torrent_source_id) VALUES (2);"
)


def test_native_isolation_retains_evidence_and_removes_only_owned_clones(
    contract_root: Path,
) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    with runtime.open(contract, user="postgres", database="final_proof") as db:
        db.sql(
            "CREATE ROLE proof_owner LOGIN; CREATE ROLE proof_runtime LOGIN; "
            "ALTER DATABASE final_proof OWNER TO proof_owner;"
        )
        for table in TABLES:
            columns = (
                f"{table}_id integer PRIMARY KEY, {table}_public_id uuid DEFAULT gen_random_uuid()"
                if table in ("canonical_torrent", "canonical_torrent_source")
                else "id integer"
            )
            db.sql(f"CREATE TABLE public.{table} ({columns})", role="proof_owner")
        db.sql(
            """
            CREATE FUNCTION public.fixture_result() RETURNS TABLE (
                canonical_torrent_public_id uuid, canonical_torrent_source_public_id uuid,
                observed_at timestamptz)
            LANGUAGE sql SECURITY DEFINER SET search_path = pg_catalog AS $$
                SELECT a.canonical_torrent_public_id, b.canonical_torrent_source_public_id,
                    transaction_timestamp()
                FROM public.canonical_torrent a CROSS JOIN public.canonical_torrent_source b;
            $$;
        """,
            role="proof_owner",
        )
        db.sql(
            "CREATE DATABASE reference_proof TEMPLATE final_proof OWNER proof_owner",
            database="postgres",
        )
        evidence = IngestionEvidence(
            "search_result_ingest_v1(boolean)", "\nBEGIN\nEND;\n", "proof_runtime"
        )
        runner = IsolatedIngestion(db, "proof_owner", "proof_runtime", SEED, evidence)
        query = session({}, repeat=True).replace(
            ingestion_call({}), "SELECT row_to_json(r) FROM public.fixture_result() r;"
        )
        reference = runner.run("warm-case", query, Variant.REFERENCE)
        final = runner.run("warm-case", query, Variant.FINAL)
        assert reference == final
        assert final["states"] == ["00000", "00000"]
        assert db.sql("SELECT count(*) FROM canonical_torrent") == "0"
        assert (
            db.sql("SELECT count(*) FROM pg_database WHERE datname LIKE 'ingestion_%_proof'") == "0"
        )
        prefix = contract.output / "warm-case-final"
        for suffix in (".sql", ".stdout", ".stderr", "-tables.json"):
            assert (prefix.parent / (prefix.name + suffix)).stat().st_mode & 0o777 == 0o600

        # Malformed evidence, SQL transport failure, and seed failure must all
        # leave the source untouched and remove the clone they actually created.
        for bad_query in (
            "SELECT 'unrecognized'",
            "SELECT missing_function()",
            "DO $$ BEGIN RAISE WARNING 'unexpected'; END $$;",
        ):
            with pytest.raises(ToolingError):
                runner.run("warm-case", bad_query, Variant.FINAL)
            assert (
                db.sql("SELECT count(*) FROM pg_database WHERE datname = 'ingestion_final_proof'")
                == "0"
            )
            assert not (contract.output / "warm-case-final-tables.json").exists()
        broken = IsolatedIngestion(
            db, "proof_owner", "proof_runtime", "SELECT missing_seed()", evidence
        )
        with pytest.raises(ToolingError, match="query failed"):
            broken.run("bad-seed", query, Variant.FINAL)
        assert (
            db.sql("SELECT count(*) FROM pg_database WHERE datname = 'ingestion_final_proof'")
            == "0"
        )

        db.sql("CREATE DATABASE ingestion_final_proof", database="postgres")
        with pytest.raises(ToolingError, match="query failed"):
            runner.run("collision", query, Variant.FINAL)
        assert (
            db.sql("SELECT count(*) FROM pg_database WHERE datname = 'ingestion_final_proof'")
            == "1"
        )
        db.sql("DROP DATABASE ingestion_final_proof", database="postgres")
        with pytest.raises(ToolingError, match="artifact name"):
            runner.run("../escape", query, Variant.FINAL)
        assert db.sql("SELECT count(*) FROM canonical_torrent") == "0"
    assert receipt(contract)["storage_removed"] is True
