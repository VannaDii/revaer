# Media transcoding preflight evaluation failure accessors slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - `JobPreflightEvaluation` provided `is_ready`, `as_ready`, and `as_failed`, but callers still had to traverse nested failure structs for common diagnostic fields.
  - This adds repeated branching/read logic at call sites.
- Decision:
  - Add `failed_stage()` and `error_code()` accessors directly on `JobPreflightEvaluation`.
  - Return `None` for `Ready` and `Some(...)` for `Failed` using stable borrowed values.
- Consequences:
  - Positive outcomes: direct access to key failure diagnostics from top-level evaluation object.
  - Risks or trade-offs: minor API expansion with no behavior change.
- Follow-up:
  - Prefer these accessors in app/API code where only top-level diagnostics are needed.

## Task Record

- Motivation:
  - Reduce nested failure-struct access boilerplate.
- Design notes:
  - Implemented constant-time borrowed accessors for deterministic diagnostics extraction.
  - Extended existing evaluation test to assert all accessor semantics.
- Test coverage summary:
  - Updated `preflight_evaluation_ready_flag_is_deterministic` with failure metadata assertions.
  - Re-ran `cargo test -p revaer-media-runtime` (38 passed).
- Observability updates:
  - None; this is an ergonomic accessor addition.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is low and isolated to jobs runtime helper methods.
  - Rollback is a single commit revert of accessor additions and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
