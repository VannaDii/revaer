# Media artifact and compact audit persistence slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires bounded diagnostic artifact references and compact audit facts to survive detail pruning.
  - Existing job persistence covered operations, violations, plan reasons, and verification checks, but not artifact references or retained audit facts.
- Decision:
  - Add `media_job_artifact` for managed diagnostic artifact references.
  - Add `media_job_compact_audit` for normalized compact audit facts.
  - Add append/list stored procedures and `revaer-data` callers for both row families.
  - Keep artifact content out of database state; persist only bounded references and metadata.
- Consequences:
  - Positive outcomes:
    - Runtime execution and retention work can store artifact references and compact audit facts without JSONB or unbounded detail blobs.
    - Later cleanup can prune bulky job details while preserving audit facts needed to explain destructive work.
  - Risks or trade-offs:
    - Artifact references assume a managed artifact namespace; future workspace code must enforce that references do not point at unmanaged paths.
- Follow-up:
  - Expose artifact and compact-audit rows through runtime, app, API, OpenAPI, and UI job details.
  - Enforce artifact retention and managed-path validation in workspace cleanup slices.

## Task Record

- Motivation:
  - Continue slice 6 by normalizing diagnostic artifact references and compact audit facts in persistence.
- Design notes:
  - Stored artifact rows are keyed by `(media_job_id, artifact_index)` and include kind, path, optional size, and optional content type.
  - Stored compact audit rows are keyed by `(media_job_id, audit_index)` and include fact kind plus bounded text.
  - Both append procedures upsert by deterministic per-job index.
- Test coverage summary:
  - Added failing data tests for artifact and compact-audit append/list access plus schema/procedure presence, then implemented migration and callers.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data create_and_list_media_job -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_tables_exist -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_procedures_exist -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_job_queries_surface_query_errors_without_database -- --test-threads=1`.
- Observability updates:
  - None. This adds persisted rows for later observability/API/UI exposure.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future artifact retention may require additional metadata. Roll back by removing the migration, data accessors, tests, and this ADR before runtime callers depend on them.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
