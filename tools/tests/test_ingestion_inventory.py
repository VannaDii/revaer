"""Verify frozen ingestion bodies from native catalogs and reject snapshot drift."""

from copy import deepcopy
from dataclasses import replace
from pathlib import Path

import pytest
from revaer_tooling.cli import make_context
from revaer_tooling.context import Options
from revaer_tooling.database.contract import BUILD_INPUTS, Contract, PostgresPin, assignments
from revaer_tooling.database.final_sql import approved_legacy_deltas
from revaer_tooling.database.ingestion.inventory import (
    HELPERS,
    IngestionInventory,
    routines,
    verify,
)
from revaer_tooling.errors import ToolingError
from revaer_tooling.json_data import array_value, string_value
from test_database_contract import contract_root
from test_database_final_sql import CANDIDATE
from test_database_postgres import receipt

__all__ = ["contract_root"]


def test_native_inventory_binds_parser_and_rejects_unapproved_changes(contract_root: Path) -> None:
    context = make_context(Options())
    contract = replace(
        Contract.load(contract_root, context.fs),
        postgres=PostgresPin.load(assignments(context.root, context.fs, BUILD_INPUTS)),
    )
    runtime = context.tools.proof_databases
    checks: list[bool] = []
    with runtime.open(contract, user="postgres", database="final_proof") as db:
        db.sql(
            "CREATE ROLE proof_owner LOGIN; CREATE ROLE proof_runtime LOGIN; "
            "ALTER DATABASE final_proof OWNER TO proof_owner"
        )
        db.docker.create_database(db.container, "postgres", "reference_proof")
        for database in ("reference_proof", "final_proof"):
            role = "postgres" if database == "reference_proof" else "proof_owner"
            db.sql(
                "CREATE SCHEMA revaer_config; CREATE SCHEMA revaer_runtime",
                role=role,
                database=database,
            )
            source = (
                CANDIDATE if database == "reference_proof" else approved_legacy_deltas(CANDIDATE)
            )
            db.sql(source.decode(), role=role, database=database)
            for name in HELPERS:
                if name != "search_result_ingest_v1":
                    db.sql(
                        f"CREATE FUNCTION public.{name}() RETURNS integer "
                        "LANGUAGE sql AS 'SELECT 1'",
                        role=role,
                        database=database,
                    )
            db.sql(
                "CREATE TABLE public.canonical_torrent (id integer); "
                "CREATE TRIGGER proof_trigger BEFORE INSERT ON public.canonical_torrent "
                "FOR EACH ROW EXECUTE FUNCTION public.observe()",
                role=role,
                database=database,
            )
        inventory = IngestionInventory(db, "proof_runtime", lambda _, passed: checks.append(passed))
        result = inventory.collect()
        assert checks == [True]
        assert result.evidence.signature == "search_result_ingest_v1()"
        assert (
            result.evidence.source
            == routines(result.reference)["search_result_ingest_v1"]["source"]
        )
        assert len(array_value(result.final["triggers"])) == 1
        assert (contract.output / "final_proof-inventory.json").stat().st_mode & 0o777 == 0o600

        for mutation in (
            "version",
            "missing",
            "duplicate",
            "body",
            "signature",
            "reference-guc",
            "final-guc",
            "delta",
        ):
            reference, final = deepcopy(result.reference), deepcopy(result.final)
            before, after = routines(reference), routines(final)
            if mutation == "version":
                final["version"] = "160013"
            elif mutation == "missing":
                array_value(final["routines"]).pop()
            elif mutation == "duplicate":
                array_value(final["routines"]).append(after["normalize_title_v1"])
            elif mutation == "body":
                after["normalize_title_v1"]["source"] = "SELECT 2"
            elif mutation == "signature":
                after["normalize_title_v1"]["signature"] = "normalize_title_v1(integer)"
            elif mutation == "reference-guc":
                before["search_result_ingest_v1"]["settings"] = []
            elif mutation == "final-guc":
                after["search_result_ingest_v1"]["settings"] = [
                    "plpgsql.variable_conflict=use_column"
                ]
            else:
                row = before["search_result_ingest_v1"]
                row["source"] = string_value(row["source"]).replace(
                    "tmp_policy_rules", "unreviewed"
                )
            with pytest.raises(ToolingError):
                verify(reference, final, "proof_runtime")

        # Live source drift must prevent collection from emitting a positive check.
        db.sql(
            "CREATE OR REPLACE FUNCTION public.normalize_title_v1() RETURNS integer "
            "LANGUAGE sql AS 'SELECT 2'",
            role="proof_owner",
        )
        checks.clear()
        with pytest.raises(ToolingError, match="body or signature"):
            inventory.collect()
        assert checks == []
    assert receipt(contract)["storage_removed"] is True
