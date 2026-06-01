# Media artifact and compact audit app facade slice 6/10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires artifact references and compact audit facts to move through the same application facade as other job diagnostics.
  - Runtime persistence access now exists, but API handlers should consume typed app-facade DTOs instead of calling runtime or data crates directly.
- Decision:
  - Add app-facade append/list parameters and response rows for media job artifacts.
  - Add app-facade append/list parameters and response rows for media job compact audit facts.
  - Wire `MediaService` to map facade DTOs to runtime store calls and project rows back to response types.
  - Keep unavailable media behavior consistent by returning empty lists and `media_unavailable` append errors from the noop facade.
- Consequences:
  - Positive outcomes:
    - API and handler slices can expose artifact and audit diagnostics through stable application boundaries.
    - The production app service round-trip now covers artifact and compact-audit persistence alongside existing job diagnostics.
  - Risks or trade-offs:
    - Facade DTOs currently mirror persistence rows; later execution work may introduce stricter managed-path DTO validation before append.
- Follow-up:
  - Add API handlers, OpenAPI schemas, and UI diagnostics for artifacts and compact audit facts.
  - Add execution-time managed-path validation before artifact rows are written by workers.

## Task Record

- Motivation:
  - Continue the media diagnostic pipeline by exposing normalized artifact references and compact audit facts through the app service facade.
- Design notes:
  - Reused existing app facade patterns for verification checks and plan reasons.
  - The service maps directly between app DTOs and runtime store methods, leaving storage errors to the existing data-error mapper.
- Test coverage summary:
  - Added failing app-service round-trip assertions for artifact and compact-audit append/list calls before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-app media_service_round_trips_profile_job_yaml_and_capability_paths -- --test-threads=1`.
- Observability updates:
  - None. This prepares diagnostics for API/UI exposure without adding new logs or metrics.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: API handlers may need different naming than the facade methods. Roll back by removing facade DTOs, trait methods, service mappings, test assertions, and this ADR before handlers depend on them.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
