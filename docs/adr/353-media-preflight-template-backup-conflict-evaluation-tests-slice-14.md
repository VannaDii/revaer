# Media preflight template backup-conflict evaluation tests slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Backup-path conflict handling was implemented and validated in lower-level report-builder tests.
  - The public template preflight evaluation entrypoint needed direct coverage for these conflict modes.
- Decision:
  - Add `evaluate_preflight_from_template` tests that assert explicit failure subcodes for:
    - backup path matching source path
    - backup path matching output path
- Consequences:
  - Positive outcomes: caller-facing template entrypoint behavior is now locked for backup conflict semantics.
  - Risks or trade-offs: none beyond additional test runtime.
- Follow-up:
  - Reuse these deterministic subcodes when exposing preflight results in API/UI.

## Task Record

- Motivation:
  - Continue slice 14 with stronger guarantee that the public preflight entrypoint preserves backup-path safety semantics.
- Design notes:
  - Added two focused tests in media-runtime jobs module for conflict subcode projection.
  - No production behavior changes in this increment; tests only.
- Test coverage summary:
  - Added tests:
    - `evaluate_preflight_from_template_rejects_backup_path_matching_source`
    - `evaluate_preflight_from_template_rejects_backup_path_matching_output`
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment strengthens slice-14 preflight safety coverage.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Very low risk; tests only. Rollback is localized revert.
- Dependency rationale:
  - No new dependencies added.
