# Media worker-owned job records

- Status: Accepted
- Date: 2026-07-30
- Context:
  - Media job phases were already made read-only through the public API so clients cannot forge worker lifecycle evidence.
  - The related operation, violation, plan-reason, verification-check, artifact, and compact-audit append DTOs and facade methods still advertised mutation-shaped API contracts even though the HTTP router only exposes read endpoints.
  - Runtime workers and data-layer tests still need internal stored-procedure append paths for durable execution evidence.
- Decision:
  - Remove public API model request types and facade append methods for media job record rows.
  - Keep data-store append functions and stored procedures as worker-owned internal persistence paths.
  - Keep public OpenAPI paths for job records read-only and scrub stale append schemas from both OpenAPI artifacts.
- Consequences:
  - API clients can read job execution evidence without being given mutation-shaped contracts that imply client-owned audit rows.
  - Runtime execution can continue to persist records through injected store dependencies.
  - Existing service tests seed internal rows through the store, then assert the public read models.
- Follow-up:
  - Continue auditing media APIs for contracts that imply caller-owned runtime state.

## Task Record

- Motivation:
  - A production transcoding worker needs durable records operators can trust; public append-shaped contracts undermine that by suggesting clients may create audit evidence.
- Design notes:
  - The router was already read-only for these record collections, so the change removes stale DTO/facade surfaces instead of changing route behavior.
  - The OpenAPI stale-schema denylist now removes the old append request names from the embedded document overlay.
  - Data-layer append functions remain because they enforce stored-procedure access and are used by the worker and schema/data tests.
- Test coverage summary:
  - `just api-export` was run to refresh OpenAPI output.
  - Focused Rust/API, generated API-client, policy, Sonar, and diff checks are rerun with this change before handoff.
- Observability updates:
  - No new logs or metrics were added; this tightens who can create existing observability rows.
- Status-doc validation:
  - Re-checked `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md` for applicable policy constraints.
- Risk & rollback plan:
  - Risk: an internal caller outside the worker may have been depending on facade append methods. Compile checks should catch this.
  - Rollback: restore the API facade methods, DTOs, OpenAPI append schemas, and service-test facade append calls, then rerun API export and focused checks.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none.
