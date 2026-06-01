# Media transcoding preflight evaluation outcome API slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Runtime provided `build_preflight_report` as `Result<...>`, plus failure-report helpers, but callers still had to map success/error into a single structured outcome type.
  - This mapping can drift across call sites.
- Decision:
  - Add `JobPreflightEvaluation` enum with `Ready(JobPreflightReport)` and `Failed(JobPreflightFailureReport)`.
  - Add `evaluate_preflight(...)` helper that always returns structured outcome payload.
  - Reuse `build_preflight_report` + `preflight_failure_report` to keep behavior consistent.
- Consequences:
  - Positive outcomes: callers can consume one deterministic outcome type without manual `Result` translation.
  - Risks or trade-offs: introduces another API entrypoint that must stay aligned with existing preflight builders.
- Follow-up:
  - Adopt `evaluate_preflight` in app/API preflight orchestration as those surfaces are implemented.

## Task Record

- Motivation:
  - Remove repeated caller boilerplate and enforce one canonical success/failure shape.
- Design notes:
  - Added `JobPreflightEvaluation` type adjacent to other preflight report models.
  - Added failure-path test to assert deterministic stage/code projection through `evaluate_preflight`.
- Test coverage summary:
  - Added `evaluate_preflight_returns_structured_failed_outcome`.
  - Re-ran `cargo test -p revaer-media-runtime` (37 passed).
- Observability updates:
  - No new telemetry signals; this is a deterministic data-shape consolidation.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to jobs runtime API surface.
  - Rollback is a single commit revert of evaluation enum/helper and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
