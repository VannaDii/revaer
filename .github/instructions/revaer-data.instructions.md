---
applyTo:
  - "crates/revaer-data/**"
  - "crates/**/migrations/**"
  - "config/database-rebaseline.env"
  - "crates/revaer-test-support/**"
  - "tools/src/revaer_tooling/database/**"
  - "tools/src/revaer_tooling/tasks/database*.py"
  - "scripts/dev-seed.sql"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes database work.

# Approved V0 Development Authority

- ADR 591 was approved on 2026-09-15. Its single-init feature-development and
  qualification boundary replaces the pre-cutover feature-schema prohibition,
  legacy-parity prerequisite and candidate-test prohibition below. Those older
  transition instructions are historical only where superseded; do not run
  migration, backward-compatibility or legacy-equivalence investigations to
  unblock this unreleased product.
- Author approved feature schemas and stored procedures in
  `crates/revaer-data/init.sql`. Test the complete candidate in explicitly owned
  disposable databases through the real authenticated application and constrained
  runtime role. Do not create migrations or reset caller-owned databases.
- Profile readiness reads latest/active immutable heads, logical binding
  readiness, per-kind catalog counts and active associations in one repeatable
  read-only snapshot. Use stored procedures and the constrained runtime grant
  list; never load nullable retired path columns or treat reporting as admission
  or execution authority. Missing heads and malformed aggregates must not be
  repaired with defaults.
- Native discovery admission rechecks the server-selected manual, schedule or
  watcher mode against the locked immutable association in the same transaction
  as generation/version fences, fingerprint de-duplication and job creation.
  Enabling another mode is not authority for the requested trigger. Route-level
  validation never replaces that stored-procedure check; native automation
  activation remains held until its background runtime is integrated and tested.
- DISC-1 schedule cadence is explicitly selected in minutes or hours, with no
  default. Persist it against the exact immutable association version; the
  conditional create procedure locks the root-catalog fence before the association
  head, rejects stale/inactive heads and duplicates, and never enables automation.
  Conditional edits compare the exact persisted `updated_at` revision while
  holding the same fences; retain pending due/coalescing evidence and advance the
  revision monotonically. Runtime cannot call the shared private writer or touch
  tables. Grant only its guarded create/read/replace procedures. Configuration persistence
  does not qualify durable background claims, recovery or automatic activation.
- Portable export reads complete native heads, association-pinned immutable
  versions and catalogs in one bounded read-only repeatable-read snapshot.
  Keep overflow lookahead and reject oversized output instead of truncating it.
  Use explicit runtime procedure grants; no exported host paths, UUIDs or
  attestation evidence become import authority. Export qualification alone does
  not qualify the SERIALIZABLE atomic import or its resource fences.
- The transition configuration uses `feature-development` under ADR 591. Its
  static guard requires a regular, nonempty, parseable init and retains the
  archived migration freeze and PR-size limits; it must not require equality
  with historical finalization bytes. Exact packaged digest and sealed runtime
  validation remain mandatory, as do fresh-init workflow and failure tests.
- Exclude unqualified D3 compiler substitutions from the selected candidate.
  Correct an ambiguous routine only as required by the real workflow, with
  caller, failure and rollback tests. Do not resume the historical proof project.
- Cutover requires exact-init fresh installation, seal/digest verification,
  least-privilege/denied-operation tests, transaction and recovery tests, complete
  application suites and real configuration-through-recovery media evidence.
  ADR 591 preserves D1/D2/D4/D5, media safety and all applicable release gates.
- Runtime configuration bootstrap verifies the packaged baseline through
  `baseline::verify_runtime_pool` before constructing runtime stores with the
  verified pool. Store construction performs no I/O or schema changes. The
  dedicated verification pool supplies the approved PostgreSQL statement
  timeout through connection startup options, not direct session-setting SQL.
  The data build hashes exact `init.sql` bytes. Failed verification
  never initializes or repairs a database and never returns its verification
  connection to the pool. A successful reader test does not qualify the complete
  initialization command, process cancellation or application cutover.
- The scoped runtime-baseline fixture provisions distinct restricted owner and
  runtime roles only inside an owned `revaer_test_*` database, applies and seals
  the full init in one transaction, disables the owner login before runtime
  reads, and removes both fixture roles on completion, including failed reads.
- Deferred integrity triggers must enforce their invariants at commit under the
  sealed function owner with a fixed search path. Never grant runtime table
  access to accommodate a deferred trigger. Factory reset must reseed the root
  readiness catalog as missing/unverified, not manufacture ready roots.

# Database Rules

- Accepted ADR 594 replaces distributed discovery reservations, claim-generation
  takeover, separate lease clocks and cleanup ownership. Keep normalized
  stored-procedure persistence for schedules, duplicate-safe candidates, job
  checkpoints and outcomes. Preserve binding authority and configured local
  concurrency; do not recreate a distributed coordination schema. The single
  service/root lock owns execution, and cleanup acknowledges actual removal.

- Native association activation publishes a coalesced DISC-1 rescan request in
  the same transaction. The private publisher has no runtime grant. Keep one
  request row per immutable association version and at most the seven closed
  reason rows; checked sequence overflow must roll back both writes. Reading or
  processing a candidate must never satisfy a clean-census high-water mark.
  Worker claim/finish fencing and configuration-activation identity remain
  required before automatic admission can be qualified.

- The baseline-verification command is thin wiring around `verify_runtime_pool`.
  It reads only an injected database endpoint, applies the existing statement
  timeout at connection startup, and never initializes or repairs schema. Its
  diagnostics must not expose endpoints, credentials or raw database errors.
- Full profile replacement appends one immutable version and changes heads in
  the same serializable transaction. Lock and compare the latest version before
  writing; stale versions must not change any persisted state. Keep the shared
  create/replace writer private to the sealed owner and grant runtime only the
  validated create and replace procedures.
- Root-binding writers acquire the catalog fence and applicable source advisory
  locks before target, policy or profile parent locks. The owned media lock
  fixture must prove a blocked profile replacement holds no resource-parent
  relation locks, then completes after release without changing source bytes.
  This single-source proof does not qualify multi-source atomic YAML import.
- Native import writes explicit immutable profile body versions and exact
  association pins, forces imported profile bodies to dry-run, and keeps an
  unresolved latest draft from replacing an active head. Its bounded digest/actor
  audit commits with the writes; failures roll back all rows. Do not confuse
  implementation with qualified late-error, recovery or complete workflow proof.
  Import fences use each resource's canonical key grammar: catalog media keys
  are not root/profile/association logical keys. Keep compiler and SQL aligned.
  Explicit local export reads native configuration and referenced path diagnostics
  in one read-only snapshot. It is not portable import authority (ADR 557).
  Ordinary job get/list/recent reads project paths relative to immutable intent
  roots and fail on inconsistent snapshots. Worker filesystem paths stay internal.
- Configuration import begins its serializable transaction through the data
  adapter and explicitly rolls back any failed write. Envelope intent validation
  is not a database fence: locked head/create comparisons, source-before-parent
  ordering, native draft writes and one bounded audit must still be qualified
  before claiming atomic import complete.
- Import preparation uses the catalog singleton lock, ascending applicable
  source-attestation advisory locks, then explicit native kind/key parent locks.
  Compare create intent and expected heads in that same serializable transaction;
  unknown matched heads and already-existing create keys must fail before writes.
  Catalog factories take the singleton's shared lock before creating versions so
  a family head cannot advance around the import fence. Preparation and rejected
  writes alone do not qualify native application, drafts, audit or recovery.
- Runtime application code must call stored procedures for database behavior. Do not embed inline business SQL in Rust.
- Immutable discovery-association collection reads use one bounded keyset page
  and its complete single-association representation in the same database
  snapshot. Validate continuation membership and ordering; grant the runtime
  procedure execution only, never table access to implement reload continuity.
- Association reads expose effective dry-run as the OR of the referenced profile
  restriction and its output-policy restriction. Preview and admission must use
  this same value; missing output restrictions fail closed. Destructive readiness
  is required only when the effective requested job is non-dry-run.
- Policy creation writes complete normalized output settings in the same stored
  procedure transaction as the parent and seeded components. Require explicit
  dry-run, replacement, quarantine and preservation values; non-dry-run requires
  approved in-place atomic replacement. Failed writes must leave no parent or
  component. Never update a referenced version to enable execution.
- Materialize the canonical operation-cost rows through the common policy
  creation factory, including operator-authored versions. Job snapshots must
  capture those rows; the runtime must still reject missing costs rather than
  fall back to current defaults.
- Workspace retention snapshots carry coherent job/attempt/claim identities,
  protecting active-job attempts and unpublished terminal outcomes. Queued
  cancellation updates the job and current attempt atomically; a never-claimed
  cancelled attempt retains null claim/heartbeat timestamps and is retryable.
- `rv lint` mechanically enforces that `sqlx::query*` usage stays confined to `crates/revaer-data/src`, except for the disposable test-database provisioning helper at `crates/revaer-test-support/src/postgres.rs` and its integration test. Inline DDL/DML text must not appear in authored Rust.
- Raw DDL, DML, stored procedure bodies, and seed SQL belong in the single init script or tightly scoped operational bootstrap scripts only. Retained migrations are historical artifacts, not a development surface.
- `JSONB` and related conglomerate persistence formats are banned for application state.
- Shared behavior lives in shared stored procedures. Do not duplicate the same database behavior across multiple crates.
- Use named bind parameters and explicit transactions where a multi-step change must be atomic.


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

- Runtime fault-injection fixtures may retain their owned administrative endpoint
  to corrupt persisted test evidence deliberately. Keep injection SQL in reviewed
  `scripts/tests` files, verify the selected owned database before executing it,
  and leave the application's pool on the sealed restricted runtime identity.
  Never grant runtime snapshot mutation or weaken production triggers for a test.
  Canonical policy factories already seed operation costs; fixtures must not
  append duplicates to those immutable versions (ADR 595).
