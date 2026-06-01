# Media preflight backup-path conflict guards slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Template-driven preflight now resolves backup paths from backup-root policy.
  - A resolved backup path can still be unsafe if it equals the source or output path.
- Decision:
  - Add deterministic preflight guardrails in `build_preflight_report_from_template`:
    - reject backup path equal to source path
    - reject backup path equal to output path
  - Keep classification under `JobPreflightError::BackupPath`, surfacing as existing preflight backup-path invalid category.
- Consequences:
  - Positive outcomes: preflight now blocks path-aliasing backup configs before any execution planning proceeds.
  - Risks or trade-offs: configurations that previously slipped through now fail early and must be corrected by operators.
- Follow-up:
  - Surface distinct backup-path conflict reason codes in API/UI status payloads when preflight endpoint wiring lands.

## Task Record

- Motivation:
  - Continue slice 14 by tightening deterministic safety invariants around backup and replacement paths.
- Design notes:
  - Added path-equality checks after backup-path resolution in template preflight entrypoint.
  - Extended unit coverage for source-path and output-path conflict cases.
- Test coverage summary:
  - Added tests:
    - `build_preflight_report_from_template_rejects_backup_path_equal_to_source`
    - `build_preflight_report_from_template_rejects_backup_path_equal_to_output`
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment improves slice-14 preflight safety checks.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk additive validation; rollback is localized revert of this slice.
- Dependency rationale:
  - No new dependencies added.
