# Media transcoding preflight timeline records slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - `JobPreflightReport` carried planned data, summary, and steps but no explicit stage timeline for deterministic explainability.
  - Slice 4/8 explainability requires stable representation of decision flow stages.
- Decision:
  - Add `PreflightStageRecord { stage, ok }` to runtime jobs module.
  - Extend `JobPreflightReport` with ordered `timeline` entries for successful preflight runs.
  - Stage order is fixed: `inspect_plan`, `capability_ready`, `workspace_capacity`, `build_steps`, `summarize`.
- Consequences:
  - Positive outcomes: consumers can render/emit a deterministic preflight sequence without inferring it from disparate report fields.
  - Risks or trade-offs: timeline currently captures successful path stages only; failure-path detail can be expanded in a later slice.
- Follow-up:
  - Extend timeline with failure-stage recording when preflight error surfaces are integrated into API/event responses.

## Task Record

- Motivation:
  - Improve structured explainability for runtime preflight execution ordering.
- Design notes:
  - Added new record type and report field with fixed stage sequence.
  - Kept existing error semantics unchanged to avoid churn in current callers.
- Test coverage summary:
  - Updated `build_preflight_report_returns_summary_and_steps` to assert timeline ordering/flags.
  - Added `preflight_stage_record_shape_is_stable`.
  - Re-ran `cargo test -p revaer-media-runtime` (32 passed).
- Observability updates:
  - Added structured timeline data to preflight report for downstream observability surfaces.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to `revaer-media-runtime` report data shape.
  - Rollback is a single commit revert of jobs and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
