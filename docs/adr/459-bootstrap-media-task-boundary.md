# Bootstrap media task boundary integration correction

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Not required; this extracts existing statements without changing runtime ownership, lifecycle, or policy.
- Context:
  - The workspace-retention deliverable adds one media runtime and its existing dependencies to bootstrap.
  - That addition increases `run_bootstrap_services` from below the enforced Clippy limit to 101 lines.
  - A later runtime-shutdown deliverable already establishes a dedicated media task group, but PR #170 must pass independently before that later behavior exists.
- Decision:
  - Group the existing discovery, job, and retention handles in a private `MediaRuntimeTasks` value.
  - Extract their existing construction and shutdown statements into private helpers at the retention boundary.
  - Preserve construction order, collaborators, abort behavior, join handling, and warning behavior exactly.
- Consequences:
  - PR #170 satisfies the enforced function-size lint without suppressions.
  - Later lifecycle patches can evolve one explicit media task boundary.
  - Runtime behavior and architecture remain unchanged.
- Follow-up:
  - Run formatting, checks, lint, and focused bootstrap tests at PR #170.
  - Replay later shutdown patches and verify the final leaf remains behaviorally identical.

## Task Record

- Motivation:
  - Every PR in the formal stack must pass its own quality gates.
- Design notes:
  - The helper boundary contains only code already present in the same function; it introduces no new dependency or ownership decision.
- Test coverage summary:
  - Run `just fmt-check`, `just check`, `just lint`, and bootstrap tests at this boundary, followed by full leaf verification.
- Observability updates:
  - None; existing discovery join warnings and runtime join warnings are preserved.
- Status-doc validation:
  - The ADR index and documentation summary are updated; product status is unchanged.
- Risk & rollback plan:
  - Risk is accidental task-order drift during extraction. Focused tests and the full leaf gates cover startup and shutdown behavior.
  - Inline the helpers to roll back, which restores the known lint failure.
- Dependency rationale:
  - No dependency is added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and the bootstrap requirements in `MEDIA_TRANSCODING.md`.
  - No policy drift or criteria relaxation was introduced.
