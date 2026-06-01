# Media transcoding preflight success timeline builder slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight timeline stage ordering existed in both success report construction and failure projection logic.
  - Duplicated stage literals increase drift risk when preflight stages evolve.
- Decision:
  - Add `preflight_success_timeline()` built from canonical `PREFLIGHT_STAGE_ORDER`.
  - Use this helper inside `build_preflight_report(...)` for successful timeline population.
  - Keep `preflight_timeline_for_error(...)` using the same canonical stage ordering.
- Consequences:
  - Positive outcomes: one source of truth for stage order across success/failure timelines.
  - Risks or trade-offs: helper surface grows slightly, but reduces maintenance risk.
- Follow-up:
  - Reuse timeline builders when preflight details are projected in API/events.

## Task Record

- Motivation:
  - Remove duplicated stage construction and ensure deterministic timeline semantics stay synchronized.
- Design notes:
  - Added reusable success timeline helper.
  - Added unit test for success timeline ordering and flags.
- Test coverage summary:
  - Added `preflight_success_timeline_marks_all_stages_successful`.
  - Re-ran `cargo test -p revaer-media-runtime` (35 passed).
- Observability updates:
  - Timeline construction is now centralized for consistent report output.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is low and isolated to timeline helper wiring in jobs runtime module.
  - Rollback is a single commit revert of helper additions and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
