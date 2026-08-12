# ADR 472: PR 134 Watcher Lock Lifetime Correction

- Status: Proposed
- Operator approval: Pending
- Date: 2026-08-13
- Context:
  - PR 134 kept the watcher event-buffer mutex guard alive through the function return expression.
  - Strict linting requires the guard to be released explicitly once the bounded state update is complete.
- Decision:
  - Drop the mutex guard before returning success.
  - Treat this as a nonarchitectural lifetime correction that preserves the existing synchronization design.
- Consequences:
  - The critical section ends at the last state access and the intermediate PR passes strict linting.
  - Runtime behavior remains equivalent except that unrelated waiters can resume before the return expression completes.
- Follow-up:
  - Keep watcher tests and the full lint gate green after replay.

## Task Record

- Motivation:
  - Repair the earliest stack boundary containing the lock-lifetime lint failure.
- Design notes:
  - The explicit `drop(state)` follows the final guard use and does not change state mutation or error handling.
  - No synchronization primitive, public interface, or architectural boundary changes.
- Test coverage summary:
  - Ran formatting, lint, focused watcher tests, fixture cleanup, and retained-media verification.
- Observability updates:
  - No logging, tracing, metrics, health, or event-surface changes.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; no user-facing documentation changes are required.
- Risk & rollback plan:
  - Risk is limited to moving unlock timing earlier within the same successful call; no guard-protected access follows the drop.
  - Rollback would restore the lint failure and longer critical section.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or contradiction was found.
