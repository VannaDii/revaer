# Media job violation API slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Job violation rows were persisted and exposed through the app facade, but the public HTTP surface did not yet expose them.
  - The media plan requires API handlers to surface compliance reports and normalized violation details for UI and automation.
- Decision:
  - Add shared request and response DTOs for appending and listing media job violations.
  - Add authenticated HTTP handlers and routes at `/v1/media/jobs/{media_job_public_id}/violations`.
  - Validate violation kind and restrict severity to the initial low, medium, or high values before calling the app facade.
- Consequences:
  - Positive outcomes:
    - Workers, UI, and API clients can persist and inspect normalized compliance violations for a job through the public media API.
    - The API layer now mirrors the existing operation append/list shape for row-level diagnostic records.
  - Risks or trade-offs:
    - Severity is intentionally small until the planner and UI require a richer taxonomy.
    - Report-level summary endpoints remain separate follow-up work.
- Follow-up:
  - Add OpenAPI export coverage for media endpoints.
  - Add UI job-detail rendering for persisted violations.

## Task Record

- Motivation:
  - Continue slices 6 and 10 by exposing persisted media job violations through the HTTP boundary.
- Design notes:
  - Reused the app facade rather than reaching into runtime or data crates from handlers.
  - Kept validation deterministic and small, matching current compliance report severities.
- Test coverage summary:
  - Added failing handler tests for invalid severity rejection and default-facade empty list behavior.
  - Planned verification: `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api media_job_violation -- --test-threads=1`.
- Observability updates:
  - None. This exposes stored diagnostic rows but does not add new logging, metrics, or events.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future clients may need additional severity values. Roll back by removing the DTOs, handlers, route, tests, and this ADR, leaving app/runtime/data persistence intact.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
