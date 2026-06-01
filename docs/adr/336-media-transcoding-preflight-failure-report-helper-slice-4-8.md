# Media transcoding preflight failure report helper slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Runtime provided separate helpers for failed stage, error code, and failure timeline projection.
  - Callers still needed to compose these pieces manually to build a complete failure payload.
- Decision:
  - Add `JobPreflightFailureReport` and `preflight_failure_report(&JobPreflightError)`.
  - Failure report includes `failed_stage`, `error_code`, and projected failure timeline.
  - Reuse existing deterministic helper functions to avoid duplication.
- Consequences:
  - Positive outcomes: callers can return a single deterministic failure artifact without custom mapping code.
  - Risks or trade-offs: adds one more report type to maintain as preflight stages evolve.
- Follow-up:
  - Wire this helper into app/API preflight error responses when media preflight endpoints are added.

## Task Record

- Motivation:
  - Reduce caller duplication and enforce consistent failure diagnostics payloads.
- Design notes:
  - Added structured failure report type adjacent to success report types in jobs module.
  - Helper delegates to canonical stage/code/timeline projections.
- Test coverage summary:
  - Added `preflight_failure_report_projects_stage_code_and_timeline`.
  - Re-ran `cargo test -p revaer-media-runtime` (36 passed).
- Observability updates:
  - Failure diagnostics now available as one deterministic runtime payload.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to jobs runtime helper/report data shape.
  - Rollback is a single commit revert of jobs helper and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
