# Media preflight failure error-detail field slice 14

- Status: Accepted
- Date: 2026-05-25
- Context:
  - Preflight failure payloads exposed deterministic `error_code` and stage timeline.
  - Callers still needed human-readable deterministic detail text to present actionable diagnostics without code lookup tables.
- Decision:
  - Add `error_detail` to `JobPreflightFailureReport`.
  - Add deterministic `preflight_error_detail(...)` mapping for all current preflight failure variants.
  - Extend backup-path subcode coverage with detail assertions.
- Consequences:
  - Positive outcomes: callers get stable machine code plus deterministic detail text from the same payload.
  - Risks or trade-offs: any downstream consumers constructing `JobPreflightFailureReport` manually must now populate `error_detail`.
- Follow-up:
  - Surface `error_detail` in API/UI preflight responses once endpoint integration lands.

## Task Record

- Motivation:
  - Continue slice 14 by improving preflight diagnostics while preserving deterministic error semantics.
- Design notes:
  - Introduced `preflight_error_detail` and wired it into `preflight_failure_report`.
  - Updated existing tests and added backup-variant detail mapping coverage.
- Test coverage summary:
  - Added test: `preflight_error_detail_maps_backup_path_variants_deterministically`.
  - Updated failure-report assertions to validate `error_detail`.
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment strengthens preflight explainability outputs for slice 14.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk additive payload field; rollback is localized revert.
- Dependency rationale:
  - No new dependencies added.
