# Media transcoding workspace capacity report slice 12

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Workspace preflight exposed pass/fail checks but not a structured decision report for callers that need explainability before hard failure.
  - Slice 12 emphasizes deterministic disk-impact and reserve behavior with auditable outcomes.
- Decision:
  - Add `WorkspaceCapacityReport` and `WorkspaceRejectionReason`.
  - Add `WorkspacePolicy::evaluate_capacity(...)` that returns a deterministic report for accept/reject, available budget, and rejection reason.
  - Keep existing `ensure_capacity(...)` error-returning API unchanged.
- Consequences:
  - Positive outcomes: orchestration and API layers can surface preflight capacity outcomes as structured data while preserving existing strict checks.
  - Risks or trade-offs: introduces an additional API surface that must stay consistent with `ensure_capacity` semantics.
- Follow-up:
  - Integrate report usage into higher-level media preflight APIs/events once those surfaces are wired.

## Task Record

- Motivation:
  - Improve preflight explainability and deterministic policy introspection for workspace capacity checks.
- Design notes:
  - `evaluate_capacity` mirrors `ensure_capacity` decision ordering and returns machine-readable reason enums.
  - Included `available_after_reserve_bytes` and `required_workspace_bytes` for direct UI/API consumption.
- Test coverage summary:
  - Added `evaluate_capacity_reports_acceptance_and_budget`.
  - Added `evaluate_capacity_reports_rejection_reason`.
  - Re-ran `cargo test -p revaer-media-runtime` (31 passed).
- Observability updates:
  - None in this runtime library increment.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no further docs drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to `revaer-media-runtime` workspace policy helpers.
  - Rollback is a single commit revert of workspace and ADR/index changes.
- Dependency rationale:
  - No new dependencies added.
