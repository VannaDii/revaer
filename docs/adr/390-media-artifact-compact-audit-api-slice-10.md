# Media artifact and compact audit API slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires API access to diagnostic artifact references and compact audit facts.
  - App-facade DTOs now exist, but HTTP request/response models, handlers, and routes were missing.
- Decision:
  - Add typed API models for artifact append/list payloads.
  - Add typed API models for compact-audit append/list payloads.
  - Add handlers and authenticated routes under media job diagnostics:
    - `/v1/media/jobs/{media_job_public_id}/artifacts`
    - `/v1/media/jobs/{media_job_public_id}/compact-audits`
  - Validate required text fields and reject negative artifact sizes before calling the app facade.
- Consequences:
  - Positive outcomes:
    - Workers and operator tooling can use HTTP APIs to persist and read artifact references and compact audit facts.
    - API validation keeps obvious malformed input from reaching stored procedures.
  - Risks or trade-offs:
    - OpenAPI and UI exposure remain separate slices so generated API artifacts and frontend state can be verified independently.
- Follow-up:
  - Add OpenAPI schemas/path entries and regenerate the exported specification.
  - Load artifact and compact-audit rows into UI job diagnostics.

## Task Record

- Motivation:
  - Continue media diagnostics API coverage for normalized artifact references and compact audit facts.
- Design notes:
  - Reused the existing media job diagnostic endpoint shape: GET lists ordered rows and POST appends one deterministic row.
  - Kept model fields aligned with app-facade DTOs and persistence row names.
- Test coverage summary:
  - Added failing handler tests for artifact and compact-audit request/list behavior before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api append_media_job_artifact_rejects_missing_kind -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api list_media_job_artifacts_returns_empty_payload_with_default_facade -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api append_media_job_compact_audit_rejects_missing_fact_text -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api list_media_job_compact_audits_returns_empty_payload_with_default_facade -- --test-threads=1`.
- Observability updates:
  - None. Existing API problem-detail mapping adds operation context on facade errors.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: external clients may adopt endpoint names before OpenAPI is regenerated. Roll back by removing the models, handlers, routes, tests, and this ADR before exposing the OpenAPI update.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
