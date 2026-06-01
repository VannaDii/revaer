# Media verification check app facade slice 6/10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Runtime `MediaStore` exposes verification-check persistence, but API/app callers still had no facade methods or DTOs.
  - The media plan requires verification outcomes to travel through injected app-service boundaries before HTTP and UI exposure.
- Decision:
  - Add app facade parameter and response types for media job verification checks.
  - Add append/list methods to `MediaFacade`.
  - Implement the methods in `MediaService` through the injected runtime store.
- Consequences:
  - Positive outcomes:
    - API handlers and workers can persist and inspect verification facts without direct runtime/data coupling.
    - The app service round-trip covers verification checks alongside operations, violations, and plan reasons.
  - Risks or trade-offs:
    - This is still row-level exposure; grouped verification report summaries remain follow-up work.
- Follow-up:
  - Add HTTP handlers/models and OpenAPI export for verification checks.
  - Render persisted verification checks in the media job detail UI.

## Task Record

- Motivation:
  - Continue slices 6 and 10 by carrying normalized verification-check rows through the app facade.
- Design notes:
  - Preserved dependency injection by routing through `MediaStore`.
  - Reused the data-layer input struct inside the service implementation and exposed API-facing facade DTOs at the boundary.
- Test coverage summary:
  - Added failing app-service assertions for appending and listing verification checks, then implemented the facade.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-app media_service_round_trips_profile_job_yaml_and_capability_paths -- --test-threads=1`.
- Observability updates:
  - None. This is app facade wiring for later API and UI exposure.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future verification report shape may require grouping or renaming facade methods. Roll back by removing the facade methods, service mapping, test assertions, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
