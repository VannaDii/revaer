# Media plan reason API slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Media job plan reasons were persisted and exposed through the app facade, but HTTP clients could not append or list them.
  - The media plan requires planner explanations to be available to UI and automation through the same authenticated media job boundary as operations and violations.
- Decision:
  - Add shared request and response DTOs for appending and listing media job plan reasons.
  - Add authenticated HTTP handlers and routes at `/v1/media/jobs/{media_job_public_id}/plan-reasons`.
  - Validate required reason code and reason text fields before calling the app facade.
  - Export the plan-reason route and schemas in the generated OpenAPI document.
- Consequences:
  - Positive outcomes:
    - UI and API clients can inspect persisted planner explanations without direct runtime or data access.
    - The public media job diagnostic surface now includes operations, violations, and plan reasons.
  - Risks or trade-offs:
    - The endpoint exposes row-level reasons; richer grouped report views remain follow-up work.
- Follow-up:
  - Render persisted plan reasons in the media job detail UI.
  - Add report-level aggregation once verification checks and artifacts are persisted.

## Task Record

- Motivation:
  - Continue slices 6 and 10 by exposing normalized media job plan reasons through the HTTP and OpenAPI boundary.
- Design notes:
  - Reused the injected app facade instead of coupling API handlers to runtime or data crates.
  - Kept validation narrow to required text fields because planner reason taxonomy is already owned by upstream planning code.
- Test coverage summary:
  - Added failing handler and OpenAPI tests for plan-reason append/list exposure, then implemented the route and schemas.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api media_job_plan_reason -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api openapi_document_exports_media_routes -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red just api-export`.
- Observability updates:
  - None. This exposes stored planner explanation rows but does not add new logging, metrics, or events.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future clients may require grouped reason reports rather than row-level lists. Roll back by removing the DTOs, handlers, route, OpenAPI schemas, tests, and this ADR, leaving app/runtime/data persistence intact.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
