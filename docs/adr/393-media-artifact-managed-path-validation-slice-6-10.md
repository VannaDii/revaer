# Media artifact managed path validation slice 6/10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires diagnostic artifacts to be managed, bounded references rather than arbitrary paths.
  - Artifact persistence previously rejected empty paths but did not reject traversal, absolute, or unmanaged namespaces.
- Decision:
  - Require media job artifact references to use a relative `jobs/...` managed namespace.
  - Reject artifact paths with empty, `.`, or `..` segments, trailing slashes, duplicate slashes, or backslashes.
  - Enforce the rule in the HTTP handler before facade calls and in the database through an immutable validation function plus stored-procedure check.
- Consequences:
  - Positive outcomes:
    - Unmanaged artifact references are rejected before they can be persisted.
    - The stored procedure gives a stable `media_job_artifact_path_invalid` detail for non-HTTP callers.
  - Risks or trade-offs:
    - Future artifact namespaces beyond `jobs/...` will need an explicit migration and API validation update.
- Follow-up:
  - Have execution/workspace code generate artifact references under the same `jobs/...` namespace.
  - Add retention enforcement for artifact age/size policies.

## Task Record

- Motivation:
  - Close the managed artifact-path gap left after artifact references were added to job diagnostics.
- Design notes:
  - Kept validation deterministic and string-based at the persistence/API boundary; no filesystem reads happen during append.
  - Added a database function used by both the table check constraint and append procedure.
- Test coverage summary:
  - Added failing tests for API unmanaged path rejection and data append rejection before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api append_media_job_artifact_rejects_unmanaged_path -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data create_and_list_media_job -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_procedures_exist -- --test-threads=1`.
- Observability updates:
  - None. Existing problem-detail mapping surfaces API validation failures, and stored-procedure errors carry stable details.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: callers using artifact paths outside `jobs/...` will fail. Roll back by removing migration 0134, API validation, tests, and this ADR before any release depends on the stricter rule.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
