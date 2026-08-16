# Media API source service

- Status: Accepted
- Date: 2026-08-12
- Context:
  - The media domain had persistence and runtime services but no complete authenticated HTTP surface for operator workflows.
  - API handlers must depend on an injected application facade and keep the authored OpenAPI document aligned with the routed endpoints.
- Decision:
  - Add a narrow asynchronous media facade to the API application layer and inject it through API state and server construction.
  - Expose authenticated profile, target, policy, retention, planning, discovery, job, capability, compliance, and YAML endpoints.
  - Build media paths and schemas in the authored OpenAPI document builder and verify route and schema coverage in unit tests.
- Consequences:
  - Operators and the UI can use one typed API surface without reaching into persistence or runtime internals.
  - The API source is intentionally separate from generated client and documentation artifacts so each layer can be reviewed independently.
- Follow-up:
  - Regenerate client and documentation artifacts only in their dedicated downstream deliverable when the generated output changes.

## Task Record

- Motivation:
  - Deliver the externally visible media control plane required by the transcoding specification.
- Design notes:
  - Handler validation uses shared API-model contracts, and application behavior is supplied through an injected facade.
  - Evidence collections remain read-only over HTTP; mutation stays behind application-owned execution paths.
- Test coverage summary:
  - Added handler tests for validation boundaries, discovery trigger behavior, replay payload bounds, error mapping, and YAML operations.
  - Added router tests for authenticated media routes and read-only evidence collections.
  - Added OpenAPI tests that require every media route and schema to be present.
- Observability updates:
  - Existing RFC 9457 problem responses expose typed client failures, and profile changes continue through the existing event bus.
  - Dashboard disk-usage collection is injected so health responses remain deterministic in tests.
- Status-doc validation:
  - Reviewed the media foundation status and updated it for shared stream DTO and OpenAPI schema behavior.
- Risk & rollback plan:
  - A route/schema mismatch could break generated clients; route and OpenAPI completeness tests fail before integration.
  - Roll back this child commit to remove the API exposure while leaving the input and persistence contract intact.
- Dependency rationale:
  - No dependencies were added; the implementation uses existing Axum, Serde, and workspace abstractions.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or stale references were found.
