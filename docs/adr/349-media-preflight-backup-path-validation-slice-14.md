# Media preflight backup-path validation slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight template evaluation now derives backup paths from policy backup roots.
  - If a backup root is configured but source path has no resolvable filename, previous flow could silently skip backup path generation.
- Decision:
  - Add deterministic validation in `evaluate_preflight_from_template`:
    - when backup root is configured and backup path cannot be resolved, fail preflight with stable error code.
  - Add error classification mapping for this case:
    - `preflight_backup_path_invalid`
    - failed stage `build_steps`.
- Consequences:
  - Positive outcomes: invalid destructive backup configuration is surfaced explicitly and early.
  - Risks or trade-offs: callers must provide source paths with terminal filenames when backup roots are enabled.
- Follow-up:
  - Propagate this failure code to API/UI error surfaces once preflight endpoint wiring lands.

## Task Record

- Motivation:
  - Continue slice 14 with safer mutation preflight behavior and clearer operator diagnostics.
- Design notes:
  - Added `JobPreflightError::BackupPath`.
  - Updated preflight error-code and failed-stage mapping.
  - Added focused template-entrypoint test for invalid source/backup combination.
- Test coverage summary:
  - Added test: `evaluate_preflight_from_template_rejects_unresolvable_backup_path`.
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment improves slice-14 preflight safety semantics.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk additive validation path; rollback is localized revert.
- Dependency rationale:
  - No new dependencies added.
