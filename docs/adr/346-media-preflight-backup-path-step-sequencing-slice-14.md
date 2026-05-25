# Media preflight backup-path step sequencing slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Slice 14 execution flow now models backup and atomic replacement steps.
  - Preflight output needed to mirror that real execution sequence so operators can audit destructive intent before execution.
- Decision:
  - Extend `PreflightBuildInput` with optional `backup_path`.
  - Route preflight step construction through replacement-aware step builder so preflight reports include backup and replace boundaries.
- Consequences:
  - Positive outcomes: preflight now reports execution steps consistent with replacement-capable runtime flow.
  - Risks or trade-offs: backup path is not yet propagated from higher-level profile/policy orchestration layers.
- Follow-up:
  - Thread backup policy/path from profile compilation and runtime orchestration into preflight callers.

## Task Record

- Motivation:
  - Keep preflight explainability aligned with real mutation flow as slice-14 execution support expands.
- Design notes:
  - Added `backup_path: Option<&str>` to `PreflightBuildInput`.
  - Updated `build_preflight_report` to use `build_job_execution_steps_with_replacement`.
  - Updated tests to assert first/last step sequencing when backup is configured.
- Test coverage summary:
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this change advances slice 14 execution operation coverage.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk due constrained API evolution in media-runtime; rollback is commit-level revert.
- Dependency rationale:
  - No new dependencies added.
