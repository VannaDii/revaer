# ADR 474: PR 82 Cancellation Clone Boundary Correction

- Status: Accepted
- Operator approval: Approved 2026-08-15
- Date: 2026-08-14
- Context:
  - PR 82 cloned the discovery cancellation token twice even though the intermediate binding had no later use.
  - Rust 1.96 strict linting rejects the redundant clone.
- Decision:
  - Clone the runtime's cancellation token independently for the task guard and returned task handle.
  - Treat this as a nonarchitectural ownership correction preserving the existing cancellation design.
- Consequences:
  - The discovery task retains the two owned cancellation-token references required by its guard and external handle without a redundant intermediate clone.
  - Task shutdown behavior remains unchanged.
- Follow-up:
  - Keep discovery runtime tests and the full lint gate green after replay.

## Task Record

- Motivation:
  - Restore independent lintability at the first stack boundary containing the redundant clone.
- Design notes:
  - Both clones come from the runtime field, and each resulting reference has a distinct consumer.
  - No runtime collaborator, lifecycle contract, public interface, or scanner criterion changes.
- Test coverage summary:
  - Run formatting, lint, discovery runtime tests, fixture cleanup, and retained-media verification.
- Observability updates:
  - No logging, tracing, metrics, health, or event-surface changes.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; no user-facing documentation changes are required.
- Risk & rollback plan:
  - Risk is limited to token ownership; compilation and discovery shutdown tests validate the lifecycle.
  - Rollback would restore the redundant clone and strict lint failure.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or contradiction was found.
