# Media transcoding preflight stage-code enrichment slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight timeline rows carried stage + success flag but not a machine-readable error code on failed stage rows.
  - Consumers still needed separate classification calls to attach failure semantics to timeline rendering.
- Decision:
  - Add optional `code` field to `PreflightStageRecord`.
  - Keep success rows `code=None`.
  - Set failed-stage row `code=Some(preflight_error_code(error))` in `preflight_timeline_for_error`.
- Consequences:
  - Positive outcomes: timeline rows are self-describing for both stage progression and failure class.
  - Risks or trade-offs: record schema changed; downstream users of the struct need to handle the new field.
- Follow-up:
  - Reuse stage-code timelines directly in API/event payloads when preflight surfacing is added.

## Task Record

- Motivation:
  - Improve deterministic failure explainability in a single timeline projection artifact.
- Design notes:
  - Extended `PreflightStageRecord` with optional code.
  - Updated success/failure timeline builders and associated tests.
- Test coverage summary:
  - Updated timeline-shape and timeline-failure assertions for `code` semantics.
  - Re-ran `cargo test -p revaer-media-runtime` (35 passed).
- Observability updates:
  - Failure timelines now include machine-readable failure code on the failed stage.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to runtime timeline record schema and helpers.
  - Rollback is a single commit revert of jobs changes and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
