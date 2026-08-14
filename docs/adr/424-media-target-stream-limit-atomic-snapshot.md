# Media target stream limit and atomic snapshot

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Desired-target stream cardinality was bounded only in one YAML parser while API, import, service, and database paths could accept larger graphs.
  - Profile activation and job creation checked for one row without sealing the validated target, allowing an append race to change the graph after validation.
- Decision:
  - Define `MAX_DESIRED_TARGET_STREAMS` as the application domain maximum of 1,024 and reuse it in core, API, OpenAPI, service, and YAML import validation.
  - Enforce the same maximum in PostgreSQL with a bounded count that reads at most 1,025 rows.
  - Persist target activation and serialize stream appends, profile activation, and job snapshot creation on the desired-target parent row.
  - Seal the target in the same transaction that activates a profile or inserts a job, so the subsequent job snapshot cannot race with an append.
- Consequences:
  - Desired targets accept one through 1,024 streams; zero and 1,025 or more fail closed.
  - Once activated, a desired-target version remains immutable even if a profile is later detached.
  - Stream append work serializes per target version, which is acceptable for immutable configuration writes.
- Follow-up:
  - Keep the Rust constant and database limit function aligned when the operator intentionally changes the domain contract.

## Task Record

- Motivation:
  - Resolve PR 99 review feedback by making stream cardinality and snapshot immutability complete across every write boundary.
- Design notes:
  - Database triggers protect stored-procedure and administrative writes without adding runtime inline SQL.
  - The count helper stops after one row beyond the limit, avoiding an unbounded aggregate on corrupt or adversarial data.
  - Activation is durable state, rather than inferred only from current profile or job references.
- Test coverage summary:
  - Added core YAML, API, app-service, OpenAPI, database boundary, and concurrent job creation coverage for zero, one, 1,024, and 1,025 streams.
- Observability updates:
  - Limit failures use `media_desired_target_stream_limit_exceeded`; immutable writes retain `media_desired_target_immutable`.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`; no stale policy or contradictory guidance was found.
- Risk & rollback plan:
  - Risk: migration fails when legacy activated targets are empty or any target exceeds 1,024 rows; this is intentional fail-closed behavior requiring data repair.
  - Rollback: revert this migration and application validation together before activation state is relied upon by later releases.
- Dependency rationale:
  - No dependencies were added.
