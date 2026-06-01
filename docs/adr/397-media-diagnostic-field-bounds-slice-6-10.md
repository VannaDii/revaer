# Media diagnostic field bounds slice 6/10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires diagnostic artifact references and compact audit facts to be bounded rather than unbounded text blobs.
  - Artifact and compact-audit persistence rejected empty values but did not bound kind, path, content type, or fact text lengths.
- Decision:
  - Bound diagnostic kinds to 64 characters, artifact paths to 1024 characters, artifact content types to 128 characters, and compact-audit fact text to 1024 characters.
  - Enforce these limits in HTTP handlers before facade calls.
  - Add database constraints and stored-procedure checks with stable error details for non-HTTP callers.
- Consequences:
  - Positive outcomes:
    - Diagnostic rows stay compact and suitable for retention after larger details are pruned.
    - API callers receive 400 responses instead of backend failures for oversized diagnostic inputs.
  - Risks or trade-offs:
    - Future diagnostic payloads requiring longer text must use managed artifacts rather than expanding database text rows.
- Follow-up:
  - Add retention cleanup for managed artifact files and keep compact audit facts after detail pruning.
  - Document artifact file naming once workspace execution starts producing managed diagnostics.

## Task Record

- Motivation:
  - Close the unbounded diagnostic field gap left after artifact and compact-audit row persistence was introduced.
- Design notes:
  - Kept database limits aligned with API validation so HTTP and stored-procedure callers share the same policy.
  - Preserved existing managed `jobs/...` artifact path validation while adding length checks.
- Test coverage summary:
  - Added failing API handler tests for oversized artifact paths and compact-audit fact text before implementation.
  - Extended the media job data round-trip with oversized artifact/audit rejection checks.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api append_media_job_artifact_rejects_oversized_path -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api append_media_job_compact_audit_rejects_oversized_fact_text -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data create_and_list_media_job -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-data media_tables_exist -- --test-threads=1`.
- Observability updates:
  - None. Validation failures are exposed through existing problem-detail and data error surfaces.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: oversized existing or future diagnostic rows are rejected. Roll back by removing migration 0135, API validation, tests, and this ADR before any release depends on these limits.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
