---
applyTo:
  - "crates/revaer-data/**"
  - "crates/revaer-test-support/**"
  - "crates/**/migrations/**"
  - "scripts/dev-seed.sql"
  - "tools/src/revaer_tooling/database/**"
  - "tools/src/revaer_tooling/tasks/database_rebaseline.py"
  - "tools/src/revaer_tooling/tasks/pristine.py"
  - "tools/src/revaer_tooling/tasks/database_probes.py"
  - "tools/src/revaer_tooling/external/postgres.py"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes database and migration work.

# Database Rules

- Configuration bootstrap verifies the packaged baseline without applying
  migrations. Runtime stores construct from the verified caller-owned pool;
  their construction performs no database operations.

- Packaged baseline verification is read-only and checks the exact initializer
  digest and restricted runtime identity. It does not adopt or repair databases.
  Root catalog and immutable profile adapters retain stored-procedure access.
- Capability, profile and import adapters use the complete sealed initializer
  in their database regressions, including restricted runtime verification.
- Job admission, discovery associations, rescan fences and step checkpoints
  retain their normalized stored-procedure contracts and recovery regressions.

- Runtime application code must call stored procedures for database behavior. Do not embed inline business SQL in Rust.
- `rv lint` mechanically enforces that `sqlx::query*` usage stays confined to `crates/revaer-data/src`, except for the disposable test-database provisioning helper at `crates/revaer-test-support/src/postgres.rs` and its integration test. Inline DDL/DML text must not appear in authored Rust.
- Raw DDL, DML, stored procedure bodies, and seed SQL belong in migrations or tightly scoped operational bootstrap scripts only.
- `JSONB` and related conglomerate persistence formats are banned for application state.
- Shared behavior lives in shared stored procedures. Do not duplicate the same database behavior across multiple crates.
- Use named bind parameters and explicit transactions where a multi-step change must be atomic.

# Migration Rules

- Preserve the selected phase: feature-development freeze validation permits
  initializer evolution while retaining the frozen corpus and SQL token checks.
  Do not rewrite historical pins or treat this structural check as runtime proof.

- Database transition tooling verifies the reviewed corpus, candidate, statement
  boundaries and final initializer hashes without updating the pins. Use the
  locked PostgreSQL scanner for token boundaries and the reviewed PostgreSQL
  image for execution. Preserve exact routine deltas and generated grants.
  Candidate generation must replay and redump the schema, validate the selected
  transition phase, and complete owned container/storage cleanup before publishing
  positive evidence. Never point these proof commands at an operator database.
- The foundation retains its existing migration behavior. In the media checkout,
  accepted ADR 591 supersedes legacy parity as a v0 prerequisite: author feature
  SQL in the single initializer, preserve the historical corpus, and require
  fresh-init, security, recovery and real application qualification. Do not
  create new media migrations or resume the historical D3 proof project.
- Migrations must be versioned, deterministic, and safe to replay in clean environments.
- If the runtime behavior changes, update the stored procedure layer and the Rust caller in the same change.

# Testing

- Ingestion matrix acceptance requires both permitted reference/final behavior
  and the expected final outcome. Equal failures do not pass. Stop at the first
  failed case and retain partial/interruption evidence. A passing case matrix
  cannot certify the source proof's explicitly unproven complete D3 scope.

- Approved ingestion corrections are exact counterexamples, not tolerated error
  classes. Preserve statement hashes, signatures, native source locations and
  complete expected table/result changes. Distinguish boolean evidence from
  numeric values and reject missing inventory or unrelated differences.

- Isolated ingestion cases clone only the owned proof's reference/final databases,
  preserve both table snapshots and remove successfully created clones before
  returning evidence. Never adopt or drop a clone-name collision. Seed/query/
  parser failures require cleanup; the outer proof retains container ownership.

- Ingestion diagnostic parsers must use independently verified reference bodies
  and signatures. Require the exact helper inventory and pinned server version;
  accept only the reviewed body deltas and reference-setting/final-directive
  transition. Retain both full native snapshots before declaring that check passed.

- Ingestion warm-session controls must use the actual session producer and prove
  successful second writes, distinguish changed second writes, and roll back
  only failed second writes. Retain private SQL/stdout/stderr and reject stale
  evidence, transport errors and unexpected diagnostics.

- Ingestion evidence requires direct expected roles, complete SQLSTATE/result
  framing, frozen routine-bound diagnostics, unchanged caller settings and
  helper-first observations in order. Normalize only committed valid identities
  and observed clocks. Preserve the source proof's explicit incomplete-D3 status;
  passing individual cases does not establish full qualification.

- Final schema/seed parity preserves exact approved deltas and normalizes only
  the reviewed routine headers, dump metadata, timestamp columns and two seed
  UUIDv4 identities. Keep empty tables, routine bodies and other settings in the
  comparison. Retain both private inputs and invalidate old seed evidence before
  execution; malformed identities must fail rather than disappear.

- Runtime routine inventory proofs must compare exact function identities and
  ordered settings against candidate-classified routines, not only a grant
  count. Preserve owner/definer/trigger checks, PUBLIC grants, relation privileges
  and the database privilege matrix. Keep extension rows in retained evidence;
  validate their membership classification before separating authored routines.

- Baseline proof denials require the actual native SQLSTATE and exact expected
  DETAIL, not matching text elsewhere in the diagnostic. Preserve malformed
  baseline rollback, sealing input rejection, runtime/surrogate privilege
  checks, post-seal atomicity and continued runtime access after bootstrap login
  removal. Record the tested initializer hash without changing reviewed pins.

- Reset-timeout proofs must exercise the actual routine under nested calls,
  caught errors, real cancellation and real lock contention. Preserve the
  five-second bound, caller-setting restoration, exact backend ownership,
  worker joining, observer cleanup and private diagnostic retention.

- Final-proof extension comparisons must retain complete stock definitions,
  membership, owners, security modes, ACLs and dictionary/template metadata.
  Preserve all eleven mutation/rollback cases and runtime primitive calls on
  the pinned disposable server. A passing isolated stage does not establish
  success of the complete final initializer proof.

- Pool and cancellation qualifications require the owning proof's explicit
  private input file. Preserve exact Rust library test names, all features,
  shown test output and native input/role/database validation. A missing
  qualification must fail even when Cargo exits successfully with zero tests.
- Pristine catalog proofs use the complete reviewed projection and a direct
  constrained owner in the pinned disposable server. Preserve every catalog
  class, security definition, exact reference comparison and explicit physical
  exclusion. Populated credential-bearing catalogs must fail when their complete
  projection is unreadable. Never substitute privileged reads or truncated output.
- Exercise database behavior through the same stored procedure entry points that production uses.
- Keep migration and procedure tests representative of runtime call patterns.
- If a migration or procedure change affects API or CLI behavior, update the relevant docs and task record in the same change.

# Disposable Single-Init Test Lifecycle

- `rv db-test-init` and `rv db-test-drop` retain the media test-service protocol.
  Require explicit container/admin selection, the reviewed image, unique bounded
  test names and a temporary runtime credential. Never adopt collisions.
- Stage the reviewed SQL without a shell, verify the staged initializer digest,
  apply/seal transactionally, disable owner login and retain restricted runtime
  access. Drop must verify database ownership. Failed initialization cleans only
  the database/roles it created; staging cleanup must still run if that fails.
- Native lifecycle fixtures do not substitute for complete media initializer,
  least-privilege, application, cancellation or recovery acceptance.

- Rust fixtures keep `start_postgres` databases empty for negative bootstrap
  tests. Application fixtures explicitly call `initialize_runtime` with the
  checkout's canonical initializer. Verify database identity before setup, seal
  the exact supplied digest, disable owner login and verify the restricted
  runtime identity before exposing its URL. Never bypass production validation.
- Call `close` after collecting a fixture's test result so cleanup failure fails
  the test. Drop must attempt and report cleanup after early returns. Remove only
  owned databases and confirmed fixture roles; do not log credential-bearing URLs.
