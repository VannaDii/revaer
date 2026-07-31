# Container Metadata Budget Defense

- Status: Accepted
- Date: 2026-08-12
- Context:
  - ADR 373 established database and core limits for exact container metadata, but API and YAML ingress did not reject over-budget requests early.
  - Runtime snapshot reads and FFmpeg argument construction also trusted persisted rows without independently proving the shared limits.
  - Job creation selected and copied desired-target rows without locking the target against a concurrent append transaction.
- Decision:
  - Publish one media-core contract: 64 entries, 128 UTF-8 key bytes, 4096 UTF-8 value bytes, 65536 aggregate key/value bytes, and a 131072-byte metadata argument budget.
  - Enforce the contract at API normalization, YAML validation, immutable job snapshot materialization, and FFmpeg argument construction.
  - Lock the desired-target row in share mode for the entire job snapshot transaction; append continues to require an exclusive target-row lock and rechecks immutability.
  - Bound the job metadata list procedure at one row beyond the accepted maximum so corruption is observable without unbounded materialization.
  - Preserve exact normalized-map comparison: unknown source keys are removed by `strip` and `replace`, while `preserve` compiles the complete observed map.
  - Per-boundary private limits were rejected because they can drift and turn an accepted request into a later durable failure.
- Consequences:
  - Over-budget API and YAML requests fail before database work.
  - Corrupt or racing persisted state fails closed before process execution.
  - The accepted worst case remains below the reserved FFmpeg metadata argument budget.
- Follow-up:
  - Keep every ingress and execution boundary on the exported media-core constants.
  - Retain execute-reinspect idempotence coverage when metadata probe normalization changes.

## Task Record

- Motivation:
  - Resolve both PR 124 review threads completely: normalized desired-state idempotence and bounded metadata amplification.
- Design notes:
  - Existing normalized source/desired comparison already made equal replacement and empty stripping no-ops; this change completes defense in depth around that contract.
  - PostgreSQL row locks serialize append and snapshot operations without introducing an application-side transaction protocol.
- Test coverage summary:
  - Run focused media-core, media-runtime, API, app, and data migration tests, plus formatting, policy, and instruction drift.
  - Database-backed concurrency remains covered by the target-row lock ordering and immutable append recheck; local execution depends on the configured test database.
- Observability updates:
  - Existing invalid desired-graph error codes identify count and aggregate-byte corruption before execution.
- Status-doc validation:
  - Reviewed ADR 373 and the implementation indexes; no user-facing status claim required revision.
- Risk & rollback plan:
  - The main risk is rejecting previously accepted over-budget metadata documents.
  - Roll back the complete boundary change together; do not retain an ingress limit that disagrees with persistence or execution.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - No policy contradiction was found; the implementation now satisfies the existing bounded, transactional stored-procedure rules.
