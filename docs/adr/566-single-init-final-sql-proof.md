# Single-init final SQL and constrained-role proof

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context: Implements the SQL/proof portion of accepted ADRs 522, 541, and 551.
  It does not authorize a runtime cutover, change the approved privilege matrix,
  or resolve the two failing proof assertions below.
- Decision: Keep finalization inert and fail closed pending full conformance.
  The frozen 167 migrations remain byte-identical and authoritative.

## Implementation

- `init.sql` has the approved final header, exact baseline table and seal/read
  signatures, owner-only seal, bounded read, default-privilege revocations, and
  an explicit generated grant list for 514 non-trigger authored routines plus
  baseline read. Trigger paths are fixed but direct execution is not granted.
  Extension routines are not added to that list or converted to definer rights.
- The frozen candidate remains exactly 1,624 statements / 1,593,023 bytes with
  SHA-256 `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`.
  Candidate generation, fresh apply, normalized re-dump, and statement map are
  retained before verifying finalization. No frozen gate is bypassed.
- Only the three dump-generated zero timeout resets are removed. The legacy
  `search_result_ingest_v1` superuser-only function configuration is moved to
  its equivalent `#variable_conflict use_column` compiler directive, preserving
  the routine's resolution semantics without granting parameter privileges.
  This one exact routine-security delta is independently checked alongside
  byte-identical remaining legacy SQL. PostgreSQL documents the equivalent
  function-local directive in its [PL/pgSQL implementation reference](https://www.postgresql.org/docs/16/plpgsql-implementation.html).
- Finalization-review SHA-256 is
  `d27d2d99a0957b2502d0461b27c29ed1f2a53e486d1a140e7aff8e5429d7f069`.
  This is **not a released/embedded baseline certification**: package identity,
  full application parity, and cutover gates are outstanding. Any byte change
  requires a newly reviewed digest and repeated proof.
- Read failures use SQLSTATE `P0001` with MESSAGE and DETAIL
  `baseline_shape_invalid`, or `42501` with MESSAGE and DETAIL
  `baseline_read_denied`. Missing, malformed, duplicate, and surrogate-login
  cases have live PostgreSQL coverage. The parent-owned SQLx wrapper is untouched.

## Evidence And Holds

- `just policy`: passed, including 15 new exact-delta adversarial assertions and
  the existing 43 rebaseline assertions. Mutations cover legacy SQL, headers,
  timeout resets, conflict mode, invoker rights, `pg_temp`, trigger/extension
  grants, broad grants, baseline constraints, transaction control, and candidate
  drift. `just db-init-prefix-check` validates the exact finalization form;
  assembly-phase prefix validation remains unchanged.
- `just db-init-final-proof` uses a uniquely named, unexposed disposable
  PostgreSQL 16.14 container from the canonical pinned digest, with a constrained
  owner and separate runtime/outsider roles created only in the test harness.
  Ordinary runtime and migration-backed tests do not select this init script.
- The full live proof currently reports **62 passed / 2 failed** checks. Passed
  evidence includes two-way legacy schema/extensions, canonical seed state,
  exact columns/constraints, rejected inputs/reseal, malformed read rollback,
  515 authored runtime routine identities/paths/ownership, PUBLIC revocation,
  runtime relation/database privileges, six real stored-procedure call paths,
  post-seal transaction rollback, and runtime read after owner login is disabled.
- Seed comparison normalizes generated timestamp columns and the two fresh
  UUIDv4 `rate_limit_policy_public_id` values by their stable policy identity;
  it rejects missing, duplicate, or invalid generated UUIDs and compares every
  remaining row value in both directions. This is not literal clock/UUID equality.
- **Unresolved extension privilege interpretation:** installing trusted `pgcrypto`
  and `unaccent` creates 40 extension routines owned by the image superuser,
  with inherited PUBLIC EXECUTE ACLs despite exclusion from authored grants.
  This catalog observation is not proof of direct invocation for every member;
  two entries have internal-only callback signatures. The constrained owner cannot
  revoke those privileges. No elevated owner grant, extension replacement,
  server mutation, or criteria exception is introduced to conceal this result.
  Peer review found that the proof's zero-extension-EXECUTE interpretation is
  stricter than ADR 551 unambiguously states. ADR 569 requests operator
  clarification without changing or concealing this failed assertion.
- **Unresolved timeout conflict:** the frozen canonical seed function
  `factory_reset_without_media_defaults_v1` sets transaction-local `lock_timeout`
  to `5s`. With initializer settings `120s / 120s / 30s`, the final script leaves
  `2min / 5s / 30s`. Its body is preserved, and timeout-preservation proof fails.
- The complete proof recipe remains nonzero for both conflicts. Finalization
  is not approved for cutover until they are reconciled with the operator's
  existing exact contract. The seal/read SQL is testable, but successful sealing
  alone must not be represented as proof of the complete privilege contract.
- The coordinating operator directed that both failures remain visible and that
  parent prepare Proposed ADR 569 for review. No SQLx transport hook, elevated
  owner, extension-privilege exception, or seed timeout change is included here.
- Documentation index generation, instruction drift, and diff hygiene passed;
  the link check passed 1,047 links. Documentation build exited successfully but
  emitted the existing large-search-index warning, so it is not a warning-free
  documentation gate. MAIN-scope Sonar snippet analysis reported zero issues for
  the six changed Ruby files. SQL is not supported by that snippet analyzer;
  full repository Sonar and published coverage remain unproven.
- Evidence is retained under ignored `target/database-rebaseline/`, including
  final proof JSON, effective runtime routines, normalized schema/seed pairs,
  observed timeouts, and frozen candidate evidence. Disposable containers are
  removed even when proof fails; no test media is required.

## Task Record

- Motivation: Close the approved single-init SQL portion and expose real
  constrained-owner failures before the coordinated runtime cutover.
- Design notes: Lifecycle SQL exists only in `init.sql`; the mechanical tool
  derives explicit routine hardening/grants from the pinned candidate and
  verifies exact allowed deltas. It never uses a broad catalog grant.
- Test coverage summary: Focused checks above are scoped evidence, not full
  `just ci`, `just ui-e2e`, SQLx transport/concurrency/cancellation, all 514 SP
  workflows, package/Helm validation, or published Sonar coverage. Parent runs
  integrated gates. Ruby MAIN-scope Sonar guidance is supplemental only.
- Observability updates: Bounded lifecycle SQLSTATE/reason pairs and retained
  local proof evidence; no URL, credential, arbitrary role name, or dynamic
  server message is emitted by authored lifecycle exceptions.
- Status-doc validation: Reviewed current ADRs 522/541/551 and operational
  instructions; no operator documentation claims init is a supported runtime
  path. ADR index, summary, and generated catalog reflect this task record.
- Risk and rollback plan: This local slice is intentionally unpushed and inert.
  Revert the slice before cutover without changing migrated databases. Do not
  seal or deploy this review digest as a release until all held checks pass.
- Dependency rationale: No new dependencies. Existing Ruby standard library,
  PostgreSQL tools, Docker, SQLx candidate builder, and `just` are used.
- Stale-policy check: Reviewed `AGENTS.md`, Rust/data/devops and Sonar scoped
  instructions. Replaced stale finalization prohibition with the accepted
  finalization boundary while preserving frozen authority, exact-delta proof,
  all quality gates, and no-runtime-selection rules. No criteria were relaxed.
