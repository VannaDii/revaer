# Media job violation app facade slice 6/10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Runtime `MediaStore` exposes job violation persistence, but the app/API facade did not yet carry those operations.
  - The media plan requires API and service layers to expose normalized job compliance details.
- Decision:
  - Add app facade parameter and response types for job violations.
  - Add append/list methods to `MediaFacade`.
  - Implement the methods in `MediaService` through the injected `MediaStore`.
- Consequences:
  - Positive outcomes:
    - API handlers and future workers can append and list job violations without depending on `revaer-runtime` internals.
    - The existing app service round-trip now covers violation persistence alongside jobs, operations, YAML, and capabilities.
  - Risks or trade-offs:
    - This is still a row-level facade; report-level persistence and public HTTP routes remain follow-up work.
- Follow-up:
  - Add HTTP handlers/models for job violation append/list and wire them into the media routes.

## Task Record

- Motivation:
  - Continue slice 6/10 by carrying normalized violation persistence through the app facade boundary.
- Design notes:
  - Preserved dependency injection by using the existing `MediaStore` collaborator.
  - Kept DTOs explicit and typed instead of passing loosely structured maps.
- Test coverage summary:
  - Added failing app-service test assertions for appending and listing job violations.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-app media_service_round_trips_profile_job_yaml_and_capability_paths -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo clippy -p revaer-app -p revaer-api --all-targets --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions -A clippy::redundant_pub_crate`.
- Observability updates:
  - None. This is app facade wiring for future API exposure.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: public route shape may require renaming the app facade methods. Roll back by removing the facade methods, service mapping, test assertions, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
