# Media transcoding preflight evaluation accessors slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - `JobPreflightEvaluation` had `is_ready()` but callers still needed explicit `match` blocks to access typed success/failure payloads.
  - This repeats branching logic across call sites.
- Decision:
  - Add `JobPreflightEvaluation::as_ready()` and `JobPreflightEvaluation::as_failed()` accessors.
  - Keep accessors as const borrowed views over enum variants to avoid allocation/copy overhead.
- Consequences:
  - Positive outcomes: callers can consume typed payloads with simple optional access patterns.
  - Risks or trade-offs: minimal API surface increase.
- Follow-up:
  - Prefer these accessors in app/API integration code for readable branch handling.

## Task Record

- Motivation:
  - Reduce repeated manual enum matching when consuming preflight outcomes.
- Design notes:
  - Added borrowed optional accessors alongside existing readiness helper.
  - Extended readiness test to assert accessor behavior for both variants.
- Test coverage summary:
  - Updated `preflight_evaluation_ready_flag_is_deterministic` to cover accessors.
  - Re-ran `cargo test -p revaer-media-runtime` (38 passed).
- Observability updates:
  - None; this is an ergonomic API helper only.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is low and isolated to jobs runtime helper methods.
  - Rollback is a single commit revert of accessor methods/test/ADR updates.
- Dependency rationale:
  - No new dependencies added.
