# Media transcoding preflight capacity report integration slice 12

- Status: Accepted
- Date: 2026-05-24
- Context:
  - `WorkspacePolicy::evaluate_capacity` produced structured capacity decisions, but `JobPreflightReport` did not surface that report.
  - Preflight consumers needed to recompute or infer capacity budget outcomes from separate fields.
- Decision:
  - Add `capacity_report: WorkspaceCapacityReport` to `JobPreflightReport`.
  - Populate it inside `build_preflight_report(...)` using the already-computed workspace estimate.
  - Preserve strict capacity enforcement semantics (`ensure_execution_capacity`) while exposing structured decision data.
- Consequences:
  - Positive outcomes: a single preflight artifact now includes plan, steps, timeline, and disk-capacity decision payload.
  - Risks or trade-offs: report payload size grows slightly and must remain synchronized with workspace evaluation semantics.
- Follow-up:
  - Use `capacity_report` directly in API/event surfaces for preflight diagnostics.

## Task Record

- Motivation:
  - Close data-shape gap between workspace policy evaluation and preflight result surfaces.
- Design notes:
  - Imported `WorkspaceCapacityReport` into jobs module and added it to report struct.
  - Built capacity report before final strict-capacity assertion to keep payload deterministic.
- Test coverage summary:
  - Extended `build_preflight_report_returns_summary_and_steps` with capacity-report assertions.
  - Re-ran `cargo test -p revaer-media-runtime` (34 passed).
- Observability updates:
  - Preflight reports now carry explicit workspace-budget acceptance/reason data.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to `revaer-media-runtime` report payload shape.
  - Rollback is a single commit revert of jobs and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
