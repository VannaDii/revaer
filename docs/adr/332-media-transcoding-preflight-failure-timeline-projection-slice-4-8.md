# Media transcoding preflight failure timeline projection slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight success reports include ordered timeline stages, but failure paths did not provide timeline projection for stage-by-stage UX/event rendering.
  - Callers need deterministic failure timeline rows without re-encoding stage ordering.
- Decision:
  - Add `preflight_timeline_for_error(&JobPreflightError) -> Vec<PreflightStageRecord>`.
  - Define a single canonical stage order and project timelines up to the failed stage.
  - Stages before failure are marked `ok=true`; failed stage is marked `ok=false`.
- Consequences:
  - Positive outcomes: consumers can render consistent failure timelines directly from a typed error.
  - Risks or trade-offs: function intentionally omits post-failure stages; if needed later they can be added with explicit semantics.
- Follow-up:
  - Use projected failure timelines in API/event response shaping once preflight surfaces are exposed.

## Task Record

- Motivation:
  - Improve deterministic explainability for preflight failure states.
- Design notes:
  - Added `PREFLIGHT_STAGE_ORDER` constant for one source of truth.
  - Added helper using existing `preflight_failed_stage` classifier to avoid duplicate mapping logic.
- Test coverage summary:
  - Added `preflight_timeline_for_error_marks_prior_stages_successful`.
  - Re-ran `cargo test -p revaer-media-runtime` (34 passed).
- Observability updates:
  - Failure-path timeline can now be derived in a structured way from typed errors.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to helper behavior in `revaer-media-runtime` jobs module.
  - Rollback is a single commit revert of jobs helper and ADR/index changes.
- Dependency rationale:
  - No new dependencies added.
