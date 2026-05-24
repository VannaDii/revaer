# Media transcoding preflight evaluation timeline accessor slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - `JobPreflightEvaluation` offered readiness and failure metadata accessors, but callers still needed branching to read timeline rows from nested ready/failed payload types.
- Decision:
  - Add `JobPreflightEvaluation::timeline() -> &[PreflightStageRecord]`.
  - Return shared borrowed slice for both `Ready` and `Failed` variants.
- Consequences:
  - Positive outcomes: callers can render timeline data via one uniform accessor.
  - Risks or trade-offs: minimal API expansion.
- Follow-up:
  - Use timeline accessor in app/API orchestration surfaces where evaluation outcomes are consumed.

## Task Record

- Motivation:
  - Remove remaining branch boilerplate around timeline access.
- Design notes:
  - Implemented borrowed slice accessor without cloning.
  - Extended existing evaluation accessor test with timeline assertions.
- Test coverage summary:
  - Updated `preflight_evaluation_ready_flag_is_deterministic` to assert timeline access semantics.
  - Re-ran `cargo test -p revaer-media-runtime` (38 passed).
- Observability updates:
  - None; this is an ergonomic accessor only.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is low and isolated to runtime accessor behavior.
  - Rollback is a single commit revert of accessor/test/ADR updates.
- Dependency rationale:
  - No new dependencies added.
