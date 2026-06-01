# Media verification check persistence slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires final verification outcomes to be represented as normalized data before source replacement.
  - Job operations, violations, and plan reasons were already persisted, but verification checks still lacked a table and stored-procedure accessors.
- Decision:
  - Add `media_job_verification_check` with deterministic per-job ordering.
  - Add append/list stored procedures for verification checks.
  - Add `revaer-data` rows and functions that call the stored procedures instead of using runtime SQL.
  - Track check kind, status, expected value, actual value, and details as bounded text fields.
- Consequences:
  - Positive outcomes:
    - Runtime verification can persist auditable check facts without JSONB or ad hoc diagnostics.
    - Later API/UI slices can expose verification outcomes through the same media job diagnostic model.
  - Risks or trade-offs:
    - Verification status starts with `passed`, `failed`, and `skipped`; future verification policy may need additional terminal classifications.
- Follow-up:
  - Expose verification checks through runtime, app, API, OpenAPI, and UI job details.
  - Add artifact and compact-audit persistence for retained diagnostic references and pruned job facts.

## Task Record

- Motivation:
  - Continue slice 6 by normalizing media job verification checks in persistence.
- Design notes:
  - Stored checks are keyed by `(media_job_id, check_index)` and upserted to keep planner/runtime retries deterministic.
  - Check status is constrained in the database and normalized to lowercase in the append procedure.
  - Optional expected, actual, and detail values are stored as nullable bounded text rather than structured conglomerates.
- Test coverage summary:
  - Added failing data tests for verification-check append/list access and schema/procedure presence, then implemented migration and callers.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data create_and_list_media_job -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_tables_exist -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_procedures_exist -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_job_queries_surface_query_errors_without_database -- --test-threads=1`.
- Observability updates:
  - None. This adds persisted verification facts for later observability/API exposure.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future verification policies may require more structured expected/actual fields. Roll back by removing the migration, data accessors, tests, and this ADR before any runtime callers depend on them.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
