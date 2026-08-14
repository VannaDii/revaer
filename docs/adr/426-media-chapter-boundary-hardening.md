# Media chapter boundary hardening

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Exact chapter replacement accepts operator-controlled chapter and metadata collections through HTTP, YAML, and stored-procedure boundaries.
  - Snapshot reconstruction runs in the worker path and must remain bounded, deterministic, and linear at the accepted maximum.
  - Concurrent direct stored-procedure callers must not exceed collection limits through a check-then-insert race.
- Decision:
  - Share chapter count, per-chapter metadata count, and aggregate metadata byte limits from the media-core contract.
  - Enforce those limits independently at HTTP, YAML, and database boundaries.
  - Serialize chapter and metadata appends on the desired-target row before checking limits and inserting.
  - Reconstruct the database's ordered chapter snapshot in one pass and reject malformed chapter or metadata ordering.
- Consequences:
  - A desired target accepts at most 1,024 chapters, 64 metadata entries per chapter, and 65,536 aggregate chapter-metadata UTF-8 bytes.
  - Database snapshot reads are capped at one row beyond the maximum valid joined result so corruption fails closed.
  - Callers that previously submitted oversized collections now receive a validation error.
- Follow-up:
  - Run the database-backed boundary and concurrency tests in CI where the managed PostgreSQL fixture is available.

## Task Record

- Motivation:
  - Resolve review findings about unbounded chapter collections, aggregate metadata growth, racing stored-procedure appends, and superlinear snapshot reconstruction.
- Design notes:
  - The target row is the common lock for both chapter and chapter-metadata mutation, making the count and aggregate checks atomic across writers.
  - Reconstruction consumes the stored procedure's documented order directly and treats any order violation as database corruption.
  - OpenAPI publishes the chapter and per-chapter metadata collection ceilings.
- Test coverage summary:
  - Added maximum and maximum-plus-one HTTP and YAML validation tests.
  - Added maximum-size linear reconstruction and malformed chapter/metadata ordering tests.
  - Added database-backed exact aggregate-byte and concurrent chapter-ceiling tests; local execution skips when the test database URL is unavailable.
  - Re-ran focused API, application, media-core, data, OpenAPI, formatting, and lint checks.
- Observability updates:
  - No new telemetry is required. Boundary failures retain field-specific API errors or stable database error details.
- Status-doc validation:
  - Reviewed the media specification and API schema. This hardens documented chapter replacement behavior without adding a new capability.
- Risk & rollback plan:
  - The main risk is rejecting previously accepted oversized target definitions. Roll back the commit before deployment if compatibility evidence requires a different bound; do not remove only one enforcement layer.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - No contradictions or stale references were found.
