# Factory reset transient contention retry

- Status: Accepted
- Date: 2026-08-01
- Context:
  - `just ui-e2e` exposed a factory-reset failure during API project setup: the reset endpoint returned HTTP 500 while runtime workers were concurrently polling stored procedures.
  - Factory reset truncates and reseeds broad configuration and runtime state, so brief PostgreSQL deadlock, serialization, or lock-timeout contention can happen while background workers hold ordinary read or write locks.
  - Retrying every database failure would hide real migration, procedure, auth, or schema defects. The reset path needs a narrow transient-contention retry instead.
- Decision:
  - Retry factory reset only for PostgreSQL SQLSTATE `40P01` (`deadlock_detected`), `40001` (`serialization_failure`), and `55P03` (`lock_not_available`).
  - Bound reset retries to three total attempts with a short linear delay.
  - Log each retry with the attempt, maximum attempt count, SQLSTATE, and delay.
  - Keep all non-transient reset errors fail-fast.
  - Alternatives considered:
    - Retry all reset failures: rejected because it can mask real defects and slow failures.
    - Move retry into the Playwright helper only: rejected because operator-initiated reset can encounter the same transient database contention.
- Consequences:
  - Positive outcomes:
    - Factory reset can converge through short-lived worker contention without weakening authentication or test coverage.
    - Unexpected reset failures still surface immediately.
  - Risks or trade-offs:
    - A genuine persistent lock problem now takes up to the bounded retry budget before failing.
- Follow-up:
  - If contention remains common, isolate e2e projects by database or suspend background workers during setup instead of broadening the retry allowlist.

## Task Record

- Motivation:
  - Make the required UI e2e gate reliable without bypassing coverage or relaxing reset authentication.
- Design notes:
  - Implemented retry in the configuration facade so API and non-HTTP callers share the same behavior.
  - Kept retry classification on SQLSTATE rather than error text.
- Test coverage summary:
  - Added unit coverage proving the retry SQLSTATE allowlist rejects non-transient errors.
  - Added unit coverage for the retry delay calculation.
- Observability updates:
  - Added a warning log for each transient-contention reset retry.
- Status-doc validation:
  - Updated `docs/adr/index.md` and `docs/SUMMARY.md`.
- Risk and rollback plan:
  - Roll back the retry helper and constants if reset retry masks an unexpected production issue.
  - Do not replace this with e2e-only coverage suppression.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Reviewed existing factory-reset ADR history and the `just ui-e2e` failure artifact.
  - No policy relaxation or stale instruction contradiction was introduced.
