# Media verification check API slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Verification-check rows are persisted and exposed through runtime/app facades, but HTTP clients could not append or list them.
  - The media plan requires verification outcomes to be available through public API, OpenAPI, and UI job detail surfaces.
- Decision:
  - Add shared request and response DTOs for appending and listing media job verification checks.
  - Add authenticated HTTP handlers and routes at `/v1/media/jobs/{media_job_public_id}/verification-checks`.
  - Validate required check kind and restrict status to `passed`, `failed`, or `skipped`.
  - Export the verification-check route and schemas in the generated OpenAPI document.
- Consequences:
  - Positive outcomes:
    - Workers, UI, and API clients can persist and inspect normalized verification facts through the public media API.
    - The media job diagnostic API now covers operations, violations, plan reasons, and verification checks.
  - Risks or trade-offs:
    - The endpoint exposes row-level checks; grouped verification summaries remain follow-up work.
- Follow-up:
  - Render persisted verification checks in the media job detail UI.
  - Add artifact and compact-audit API surfaces after their persistence/facades exist.

## Task Record

- Motivation:
  - Continue slice 10 by exposing normalized media job verification checks through the HTTP and OpenAPI boundary.
- Design notes:
  - Reused the injected app facade rather than coupling API handlers to runtime or data crates.
  - Kept status validation aligned with the persistence constraint.
  - Normalized optional expected, actual, and detail text by treating blank values as absent.
- Test coverage summary:
  - Added failing handler and OpenAPI tests for verification-check append/list exposure, then implemented the route and schemas.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api media_job_verification_check -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api openapi_document_exports_media_routes -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red just api-export`.
- Observability updates:
  - None. This exposes stored verification rows but does not add new logging, metrics, or events.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future clients may need grouped verification summaries or additional check statuses. Roll back by removing the DTOs, handlers, route, OpenAPI schemas, tests, and this ADR, leaving app/runtime/data persistence intact.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
