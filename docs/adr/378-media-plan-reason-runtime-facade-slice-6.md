# Media plan reason runtime facade slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Plan reason persistence existed in the data layer, but runtime callers still had no injected facade methods for writing or reading reasons.
  - The media plan requires workers and API wiring to remain insulated from data module layout.
- Decision:
  - Add `MediaStore::append_job_plan_reason` and `MediaStore::list_job_plan_reasons`.
  - Return the typed data row through the runtime facade for follow-up app/API mapping.
- Consequences:
  - Positive outcomes:
    - Workers and app services can use injected runtime dependencies to persist planner explanations.
    - Runtime tests now cover reasons alongside profiles, jobs, operations, violations, and capabilities.
  - Risks or trade-offs:
    - The runtime facade still exposes row-level records; higher-level report projection remains a follow-up.
- Follow-up:
  - Add app and HTTP API plan-reason DTOs and handlers.
  - Render persisted plan reasons in the media job detail UI.

## Task Record

- Motivation:
  - Continue slice 6 by carrying normalized plan reasons through the runtime dependency boundary.
- Design notes:
  - Reused the existing `MediaStore` facade and did not add new concrete construction in domain logic.
  - Preserved stored-procedure-only runtime database access.
- Test coverage summary:
  - Added failing runtime assertions for appending and listing media job plan reasons, then implemented the facade methods.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-runtime media_store_ -- --test-threads=1`.
- Observability updates:
  - None. This is runtime facade wiring for later diagnostic surfaces.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future plan reason projection may rename facade methods. Roll back by removing these methods, test assertions, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
