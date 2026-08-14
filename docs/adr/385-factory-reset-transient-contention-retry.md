# Factory Reset Transient Contention Retry

- Status: Accepted
- Date: 2026-08-11
- Context:
  - Factory reset can contend briefly with runtime workers that are polling stored procedures while reset truncates and reseeds configuration and runtime state.
  - Retrying every database failure would conceal migration, procedure, authentication, connectivity, and schema defects.
- Decision:
  - Retry only PostgreSQL SQLSTATE `40P01` (`deadlock_detected`), `40001` (`serialization_failure`), and `55P03` (`lock_not_available`).
  - Bound factory reset to three total attempts with a 100 ms linear delay after the first failure and a 200 ms delay after the second.
  - Log each permitted retry with its attempt, maximum attempt count, SQLSTATE, and delay.
  - Propagate non-allowlisted errors immediately and propagate an allowlisted error after the retry budget is exhausted.
  - Alternatives considered:
    - Retry all database errors: rejected because terminal defects must remain immediately visible.
    - Retry only in UI test setup: rejected because operator-initiated resets can encounter the same contention.
- Consequences:
  - Factory reset can converge through bounded, known PostgreSQL contention without relaxing its failure contract.
  - Persistent contention adds at most 300 ms of retry delay before the original data error is returned.
- Follow-up:
  - If contention remains common, isolate reset from background workers rather than expanding the SQLSTATE allowlist or retry budget.

## Task Record

- Motivation:
  - Restore reliable factory reset behavior under short-lived PostgreSQL contention while preserving fail-closed handling for every other failure.
- Design notes:
  - Classification uses SQLSTATE values from the typed data error instead of parsing error text.
  - Retry eligibility combines the SQLSTATE allowlist with the current attempt, making the total-attempt boundary directly testable.
- Test coverage summary:
  - Added focused unit tests for every allowlisted SQLSTATE, representative rejected SQLSTATEs, missing SQLSTATEs, invalid attempt zero, linear delays, and exhaustion at three total attempts.
  - The focused `revaer-config` run passed all three factory-reset retry tests, and `just lint` passed.
  - The full `just test` run stopped at two inherited media chapter error-code assertion failures before completing the workspace; this change does not alter those media paths.
  - `just ui-e2e` passed all 45 `api-none` tests, then the inherited API process exited without a terminal error in its log before the `api-api-key` project, causing the remaining project and UI coverage gate to fail.
- Observability updates:
  - Added one structured warning per permitted retry; terminal errors retain the existing `config.factory_reset` operation context.
- Status-doc validation:
  - Product status and operator guides are unaffected. `docs/adr/index.md` and `docs/SUMMARY.md` were updated.
- Risk & rollback plan:
  - A persistent lock issue takes up to 300 ms longer to surface. Roll back the retry constants, helper, loop, tests, and this ADR together if the policy causes an operational regression.
- Dependency rationale:
  - No dependency was added; the implementation uses existing Tokio timing and typed data-layer errors.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - No policy drift, contradiction, or relaxation was introduced.
