# Media job violation runtime facade slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The data layer now exposes stored-procedure wrappers for media job violation rows.
  - Runtime callers should use `MediaStore` rather than reaching into `revaer-data` directly.
- Decision:
  - Add `MediaStore::append_job_violation` and `MediaStore::list_job_violations`.
  - Cover the methods in the runtime media store round-trip and closed-pool error tests.
  - Treat a missing local test database URL as a skippable local integration-test condition, matching the media data tests.
- Consequences:
  - Positive outcomes:
    - Workers and app services can persist and list media job violations through the injected runtime facade.
    - Runtime tests exercise the same stored-procedure boundary production uses.
  - Risks or trade-offs:
    - The facade exposes low-level row operations; higher-level report persistence still needs a later helper that writes all violations from a planned job.
- Follow-up:
  - Add app/API methods that persist a planned job's compliance report through these runtime facade calls.

## Task Record

- Motivation:
  - Continue slice 6 by routing violation persistence through the runtime dependency boundary.
- Design notes:
  - Kept the facade narrow and aligned with the data-layer stored procedure signatures.
  - Preserved dependency injection by adding methods to `MediaStore` instead of constructing data-layer collaborators elsewhere.
- Test coverage summary:
  - Added failing runtime tests for appending/listing violations through `MediaStore` and for closed-pool error propagation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-runtime media_store_ -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo clippy -p revaer-runtime --all-targets --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions -A clippy::redundant_pub_crate`.
- Observability updates:
  - None. This is runtime persistence wiring only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: later report-level persistence may supersede direct row calls. Roll back by removing the facade methods, tests, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
