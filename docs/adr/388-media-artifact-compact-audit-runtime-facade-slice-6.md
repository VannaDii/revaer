# Media artifact and compact audit runtime facade slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Runtime callers should not depend on the `revaer-data` module layout when persisting media job diagnostic facts.
  - Artifact references and compact audit facts now exist in stored-procedure-backed persistence, but higher layers need the same narrow runtime facade used for operations, violations, plan reasons, and verification checks.
- Decision:
  - Expose append/list methods for media job artifact references on `MediaStore`.
  - Expose append/list methods for media job compact audit facts on `MediaStore`.
  - Keep the facade as a direct typed pass-through over stored procedures with no environment reads or concrete collaborator construction.
- Consequences:
  - Positive outcomes:
    - Application and API layers can consume normalized artifact and audit rows through the runtime boundary.
    - Runtime tests now exercise both successful round-trips and query-error propagation for the new row families.
  - Risks or trade-offs:
    - The runtime facade intentionally does not validate managed artifact paths; workspace cleanup and execution slices must own that policy.
- Follow-up:
  - Expose artifact and compact-audit rows through the app service, API, OpenAPI, and UI job diagnostics.
  - Add managed-path validation when execution creates artifact references.

## Task Record

- Motivation:
  - Continue slice 6 by making normalized artifact references and compact audit facts available through the runtime persistence facade.
- Design notes:
  - Reused existing `MediaStore` pass-through style and returned `revaer-data` row types directly, matching adjacent media job diagnostic methods.
  - Kept all runtime database access behind stored-procedure-backed `revaer-data` calls.
- Test coverage summary:
  - Added failing runtime tests for artifact and compact-audit append/list facade methods before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-runtime media_store_round_trips_profiles_jobs_and_capabilities -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-runtime media_store_methods_surface_query_errors_without_database -- --test-threads=1`.
- Observability updates:
  - None. This adds runtime accessors for diagnostic data that will be surfaced by later API and UI slices.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: later service layers may need more constrained DTOs. Roll back by removing the facade methods, tests, and this ADR before app/API callers depend on them.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
