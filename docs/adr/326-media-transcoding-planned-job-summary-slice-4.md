# Media transcoding planned job summary slice 4

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Runtime planning produced operation lists but had no structured summary artifact for counts and explanations.
  - Slice 4 requires explainability surfaces that are deterministic and easy to consume by API/UI layers.
- Decision:
  - Add `PlannedJobSummary` and `summarize_planned_job` in `revaer-media-runtime::jobs`.
  - Summary includes operation-kind counts and deterministic explanation rows via `revaer-media-core::explain::explain_plan`.
- Consequences:
  - Positive outcomes: call sites can consume structured plan telemetry/explanation without rebuilding operation analysis logic.
  - Risks or trade-offs: summary schema must evolve carefully once additional operation kinds are introduced.
- Follow-up:
  - Wire this summary into media API responses/events when planning endpoints are expanded.

## Task Record

- Motivation:
  - Close explainability gap between raw planned operations and higher-level service surfaces.
- Design notes:
  - Added runtime-local summary struct with explicit counters for remux/audio/video operations.
  - Reused existing deterministic explanation generator from `revaer-media-core`.
- Test coverage summary:
  - Added `summarize_planned_job_counts_kinds_and_includes_explanations`.
  - Re-ran `cargo test -p revaer-media-runtime` (26 passed).
- Observability updates:
  - None yet; this creates a reusable data shape for later API/event surfacing.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional policy/doc drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to media-runtime planning summary behavior.
  - Rollback is a single commit revert of jobs runtime summary changes and ADR/index entries.
- Dependency rationale:
  - No new dependencies added.
