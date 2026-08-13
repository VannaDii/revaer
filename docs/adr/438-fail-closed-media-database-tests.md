# Fail-closed media database tests

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Media integration helpers returned optional databases and callers treated provisioning failures as successful tests.
  - Required database coverage must prove behavior against PostgreSQL rather than silently reducing the executed suite.
- Decision:
  - Make media test database helpers return concrete test databases and propagate provisioning, connection, initialization, and retry-exhaustion errors.
  - Preserve bounded retries for transient PostgreSQL startup states, but fail after the bound instead of reporting success.
- Consequences:
  - Missing or unhealthy test infrastructure fails the affected suite immediately.
  - Local contributors must provide the documented test database before running database-backed tests.
- Follow-up:
  - Keep database-backed test helpers concrete and reject new skip-on-infrastructure-failure patterns in review.

## Task Record

- Motivation:
  - Ensure media database coverage cannot appear green when no database behavior was exercised.
- Design notes:
  - The helper boundary owns infrastructure failure propagation, so every existing caller inherits fail-closed behavior without duplicating policy.
- Test coverage summary:
  - Run the complete repository CI gate and UI E2E gate against an isolated PostgreSQL database.
- Observability updates:
  - Test output now reports the original provisioning or timeout error instead of a skip message.
- Status-doc validation:
  - Reviewed media test conventions and the v0 initializer task record; no user-facing runtime documentation changed.
- Risk & rollback plan:
  - Tests may newly fail on machines without PostgreSQL. Restore the required test service; do not restore successful skips.
- Dependency rationale:
  - No dependencies added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and the canonical Justfile gates. No policy drift found.
