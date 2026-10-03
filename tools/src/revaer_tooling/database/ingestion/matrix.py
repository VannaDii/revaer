"""Compose the original cold/helper-first/warm cases without claiming full D3."""

import json
from collections.abc import Callable

from ...errors import ToolingError
from ...json_data import Json, JsonObject
from .corrections import ApprovedCorrections, exact
from .isolated import IsolatedIngestion, Variant
from .sessions import cases, session


class IngestionMatrix:
    def __init__(
        self,
        runner: IsolatedIngestion,
        helper_sql: str,
        check: Callable[[str, bool], None],
    ) -> None:
        if not helper_sql.strip():
            raise ToolingError("Ingestion matrix requires its helper-first fixture")
        self.runner, self.helper_sql, self.check = runner, helper_sql, check
        self.corrections = ApprovedCorrections(runner.evidence.signature)

    def run(self) -> list[Json]:
        results: list[Json] = []
        completed = False
        failure: str | None = None
        output = self.runner.connection.output / "ingestion-matrix.json"
        self.runner.connection.fs.remove_owned(output, self.runner.connection.output)
        try:
            for case in cases():
                query = session(
                    case.changes,
                    repeat=case.repeat,
                    helper_sql=self.helper_sql if case.helpers_first else None,
                )
                reference = self.runner.run(
                    case.name, query, Variant.REFERENCE, helpers_first=case.helpers_first
                )
                final = self.runner.run(
                    case.name, query, Variant.FINAL, helpers_first=case.helpers_first
                )
                equivalent = exact(reference, final)
                approved = self.corrections.match(case.name, reference, final)
                admissible = equivalent or approved is not None
                accepted = admissible and final.get("states") == list(case.expected)
                results.append(
                    {
                        "name": case.name,
                        "expected": list(case.expected),
                        "helpers_first": case.helpers_first,
                        "reference": reference,
                        "final": final,
                        "equivalent": equivalent,
                        "approved_delta": approved,
                        "accepted": accepted,
                    }
                )
                self.check(f"ingestion {case.name} parity or exact approved correction", admissible)
                self.check(f"ingestion {case.name} required outcome", accepted)
                if not accepted:
                    failure = (
                        f"shared reference/final failure at {case.name}; "
                        "not a demonstrated D3-only regression"
                        if equivalent
                        else f"reference/final behavior differs at {case.name}; "
                        "further semantic review required"
                    )
                    raise ToolingError(failure + "; retained ingestion matrix evidence")
            completed = True
        except BaseException as error:
            if failure is None:
                failure = f"{type(error).__name__}: {error}"
            raise
        finally:
            report: JsonObject = {
                "matrix_completed": completed,
                "matrix_passed": completed
                and all(isinstance(row, dict) and row.get("accepted") is True for row in results),
                # The original proof explicitly leaves the full D3 scope open.
                # This phase receipt cannot be mistaken for that qualification.
                "complete": False,
                "passed": False,
                "scope": "cold-helper-first-warm matrix only",
                "stop_reason": failure or "complete D3 scope remains unproven",
                "cases": results,
            }
            self.runner.connection.fs.write(output, json.dumps(report, indent=2) + "\n", 0o600)
        return results
