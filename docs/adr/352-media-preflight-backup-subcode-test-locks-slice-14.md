# Media preflight backup subcode test locks slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Backup-path preflight failures were split into typed subcodes.
  - Regression risk remained unless test coverage explicitly locked per-variant code and timeline projection behavior.
- Decision:
  - Add targeted tests for backup-path subcode mapping and failure-report timeline projection.
  - Assert deterministic build-stage failure semantics for backup-path output-conflict variant.
- Consequences:
  - Positive outcomes: backup-path preflight diagnostics are now locked at the unit level and protected against accidental remapping.
  - Risks or trade-offs: no runtime behavior change; test surface increases slightly.
- Follow-up:
  - Mirror these subcodes in API/UI preflight surfaces when endpoint integration lands.

## Task Record

- Motivation:
  - Continue slice 14 with stronger deterministic guarantees for typed backup-path diagnostics.
- Design notes:
  - Added explicit unit checks for each backup-path subcode.
  - Added failure-report/timeline assertion for build-stage projection.
- Test coverage summary:
  - Added tests:
    - `preflight_error_code_maps_backup_path_variants_deterministically`
    - `preflight_failure_report_projects_backup_path_subcode_on_build_stage`
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment strengthens preflight diagnostics coverage for slice 14.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Very low risk; tests only. Rollback is localized revert.
- Dependency rationale:
  - No new dependencies added.
