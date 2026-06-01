# Media preflight backup-path subcodes slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight backup-path validation existed but used one coarse error code.
  - API/UI integration benefits from distinct machine-readable reasons for backup-path failures.
- Decision:
  - Replace string-based backup-path preflight failure payloads with typed `BackupPathError` variants.
  - Map each variant to explicit preflight error codes:
    - `preflight_backup_path_source_filename_missing`
    - `preflight_backup_path_matches_source`
    - `preflight_backup_path_matches_output`
- Consequences:
  - Positive outcomes: callers can distinguish backup-path validation failures deterministically without parsing text.
  - Risks or trade-offs: downstream consumers expecting only prior coarse backup-path code must accept the new finer-grained codes.
- Follow-up:
  - Thread these subcodes through API/UI preflight surfaces once endpoint integration lands.

## Task Record

- Motivation:
  - Continue slice 14 with stronger typed diagnostics and explainability for backup safety checks.
- Design notes:
  - Added `BackupPathError` enum plus `Display`.
  - Updated `JobPreflightError::BackupPath` to carry typed reason.
  - Updated preflight code classification and tests accordingly.
- Test coverage summary:
  - Updated existing backup-path preflight tests to assert precise typed variants/subcodes.
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment improves execution/preflight diagnostics in slice 14.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk; rollback is localized revert of typed error/refined code mapping.
- Dependency rationale:
  - No new dependencies added.
