# Admitted media source intent preservation

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record.
- Context:
  - Fingerprint-aware enqueue accepted a caller's requested dry-run mode but persisted only the profile's `dry_run_only` value. An explicit dry run on a normal profile could therefore become a destructive job.
  - Callers labeled responses and telemetry from a profile read performed before the enqueue procedure acquired its profile-row lock. A concurrent profile-mode change could make those labels disagree with durable job state.
  - The media-job configuration trigger was created before the source-fingerprint intent columns and did not protect those fields from updates after enqueue.
- Decision:
  - Add a fingerprint enqueue procedure that accepts the requested mode, locks the profile row, persists `requested_dry_run OR profile.dry_run_only`, and returns both the job identifier and effective durable mode.
  - Keep the previous enqueue procedure as a compatibility entry point that always requests dry-run execution.
  - Drive API responses and queue telemetry from the effective mode returned by the transaction.
  - Extend the existing media-job configuration immutability trigger after the fingerprint columns are introduced so every `intent_source_*` field is immutable.
- Consequences:
  - Explicit dry-run requests remain dry-run jobs on normal profiles, while dry-run-only profiles continue to force safe execution.
  - Concurrent profile-mode changes are resolved while holding the profile-row lock, and callers observe the same mode that was persisted.
  - Attempts to replace one field or the complete source-fingerprint intent tuple fail with `media_job_configuration_immutable`.
- Follow-up:
  - Descriptor-bound downstream tool input, atomic identity-checked replacement, and bounded fingerprint admission remain outside this nonarchitectural correction.
  - Integrate this commit into the review stack without restoring the removed public manual-job creation surface.

## Task Record

- Motivation:
  - Resolve the nonarchitectural admitted-mode, telemetry, and fingerprint immutability defects found in the PR 111 review.
- Design notes:
  - The database remains the authority for the effective execution mode because it owns the profile-row lock and job insert transaction.
  - The compatibility procedure fails safe by requesting dry-run execution; active Rust callers use the mode-returning procedure.
  - The source-fingerprint trigger correction is applied only after all five columns exist.
- Test coverage summary:
  - Added migration-backed coverage for the complete admitted-mode OR truth table: a normal profile with an execution request, a normal profile with an explicit dry-run request, and a dry-run-only profile with an execution request. Every case uses a distinct source fingerprint and asserts both the returned and persisted mode.
  - Added service-level coverage proving the direct command preserves an explicit dry-run request on an execution-enabled profile.
  - Added a concurrent migration-backed test proving enqueue waits for the profile-row lock and returns the committed effective mode.
  - Added migration-backed rejection coverage for each source-fingerprint intent field and the complete tuple, followed by a claim assertion for the original values.
  - Passed the full workspace test suite against an isolated migrated PostgreSQL instance plus `just check`, `just lint`, `just policy`, and `just instruction-drift`; after the final truth-table extension, reran the three affected migration-backed regressions independently before documentation regeneration and final diff checks.
- Observability updates:
  - Existing `media_jobs_queued_total` labels now use the effective durable dry-run mode returned by enqueue. No metric names or label dimensions changed.
- Status-doc validation:
  - Updated the ADR index and mdBook summary. No public API or operator workflow changed.
- Risk & rollback plan:
  - Risk is limited to internal enqueue result handling and stricter rejection of invalid post-enqueue writes.
  - Revert the procedure, caller, trigger, and tests together. Do not retain caller-derived telemetry with database-selected durable state.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: ADRs 354 and 423 omitted the UI instruction review and claimed tests or guarantees not present in the implementation.
  - Corrected those records without relaxing lint, database, Sonar, coverage, or completion criteria.
