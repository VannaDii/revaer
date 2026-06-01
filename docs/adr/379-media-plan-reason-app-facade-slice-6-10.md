# Media plan reason app facade slice 6/10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Runtime `MediaStore` exposes plan-reason persistence, but app/API facade callers could not append or list those rows.
  - The media plan requires planner explanations to travel through injected service boundaries before HTTP/UI exposure.
- Decision:
  - Add app facade parameter and response types for media job plan reasons.
  - Add append/list methods to `MediaFacade`.
  - Implement the methods in `MediaService` through the injected runtime store.
- Consequences:
  - Positive outcomes:
    - API handlers and future workers can persist and read plan reasons without direct runtime/data coupling.
    - The app service round-trip covers plan reasons alongside operations and violations.
  - Risks or trade-offs:
    - This is still row-level facade exposure; report-level grouping remains follow-up work.
- Follow-up:
  - Add HTTP handlers/models for plan reason append/list.
  - Render persisted reasons in the media job detail UI.

## Task Record

- Motivation:
  - Continue slices 6 and 10 by carrying normalized plan-reason rows through the app facade.
- Design notes:
  - Preserved dependency injection by routing through `MediaStore`.
  - Kept DTOs explicit and typed to avoid generic diagnostic maps.
- Test coverage summary:
  - Added failing app-service test assertions for appending and listing plan reasons, then implemented the facade.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-app media_service_round_trips_profile_job_yaml_and_capability_paths -- --test-threads=1`.
- Observability updates:
  - None. This is app facade wiring for later API exposure.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future reason report shape may require renaming facade methods. Roll back by removing the facade methods, service mapping, test assertions, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
