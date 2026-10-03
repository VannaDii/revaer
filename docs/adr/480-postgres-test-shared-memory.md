# PostgreSQL Test Shared Memory

- Status: Recorded
- Date: 2026-08-15
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The complete validation gate runs database-backed workspace tests concurrently against disposable PostgreSQL databases.
  - Docker's 64 MiB shared-memory default allowed two migration workers to exhaust the service allocation, producing PostgreSQL `53100` failures before test assertions ran.
  - The same default applied to local validation and GitHub-hosted pull-request and Sonar service containers.
- Decision:
  - Reserve 1 GiB of shared memory for every PostgreSQL 16 validation service container.
  - Recreate the named managed local container when its configured allocation is below that floor while preserving its host-backed data directory.
  - Fail workflow policy when a PostgreSQL 16 service omits the matching allocation.
  - Preserve concurrent workspace testing, isolated disposable databases, and all existing coverage and quality thresholds.
- Consequences:
  - Parallel migrations have enough dynamic shared memory without serializing the test corpus or masking scheduler-dependent defects.
  - Existing local managed containers are recreated once to adopt the allocation; external database endpoints are unchanged.
  - The limit is a capacity ceiling and does not reserve 1 GiB eagerly.
- Follow-up:
  - Re-run `just ci` and `just ui-e2e` against the recreated local service.
  - Keep local and GitHub service allocations aligned when validation topology changes.

## Task Record

- Motivation:
  - Make the independently validated bottom stack boundary deterministic under the repository's existing parallel database test load.
- Design notes:
  - The change sizes validation infrastructure only; it does not alter product persistence, migration semantics, runtime concurrency, or production deployment.
- Test coverage summary:
  - `just policy` verifies that all PostgreSQL 16 workflow services retain the 1 GiB allocation.
  - Docker inspection reported `1073741824` bytes after `just db-start` recreated the managed container.
  - `just ci` passed after recreation, including the previously failing application bootstrap migration group and the complete coverage run.
  - `just ui-e2e` passed all 101 API and browser tests.
- Observability updates:
  - Local startup reports when it replaces an underprovisioned managed container.
- Status-doc validation:
  - Updated the scoped DevOps rule, ADR catalogue, and documentation summary.
- Risk & rollback plan:
  - Revert the allocation and guard together only after validation no longer performs parallel migrations or a measured smaller floor is proven reliable.
  - Do not respond to shared-memory exhaustion by reducing test coverage or quality thresholds.
- Dependency rationale:
  - No dependency is added. The implementation uses Docker service options and existing shell tooling.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, `justfile`, `.github/workflows/pr.yml`, `.github/workflows/sonar.yml`, and `scripts/workflow-guardrails.sh`.
  - Drift was found in the undocumented Docker shared-memory assumption and corrected with a mechanical workflow guard.
