# Media verification check runtime facade slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Verification-check persistence exists in `revaer-data`, but workers and app services need an injected runtime boundary.
  - The media plan requires runtime database access to remain stored-procedure backed and isolated behind narrow facades.
- Decision:
  - Add `MediaStore::append_job_verification_check`.
  - Add `MediaStore::list_job_verification_checks`.
  - Return typed data rows for follow-up app/API projection.
- Consequences:
  - Positive outcomes:
    - Runtime callers can persist and read verification check facts without reaching into data modules.
    - Runtime tests now cover verification checks alongside jobs, operations, violations, plan reasons, and capabilities.
  - Risks or trade-offs:
    - This remains a row-level facade until higher-level verification reports are modeled.
- Follow-up:
  - Carry verification checks through app facade, HTTP/OpenAPI, and media UI details.
  - Add artifact and compact-audit runtime facades after persistence exists.

## Task Record

- Motivation:
  - Continue slice 6 by carrying normalized verification checks through the runtime dependency boundary.
- Design notes:
  - Reused `AppendMediaJobVerificationCheckInput` to avoid argument-list drift across data/runtime layers.
  - Preserved stored-procedure-only access by delegating to `revaer-data` callers.
- Test coverage summary:
  - Added failing runtime assertions for appending/listing verification checks and closed-pool error propagation, then implemented the facade methods.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-runtime media_store_ -- --test-threads=1`.
- Observability updates:
  - None. This is runtime facade wiring for later report and event exposure.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: future report projections may rename these methods. Roll back by removing the runtime methods, test assertions, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
