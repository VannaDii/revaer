# Packaged init-script bootstrap lifecycle

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- ADR 522 accepts `crates/revaer-data/init.sql` as the sole pre-v1 database
  authority, but it intentionally does not select how a packaged application
  initializes a fresh database after the migration runner is removed.
- Today `ConfigService` embeds and runs the migration directory before reading
  configuration, and `RuntimeStore` embeds and runs the same directory again
  when the native runtime feature is enabled. Application startup therefore
  mutates schema and can execute the corpus twice.
- The runtime Docker image contains the application binary but no independent
  database initializer. The Helm chart starts one application container with
  one `DATABASE_URL`; it has no init container, bootstrap principal, or
  baseline gate.
- Local, pull-request, Sonar, coverage, and development recipes call the SQLx
  migration runner. Playwright global setup invokes that runner directly for
  each temporary database. Removing migrations without one replacement entry
  point would make package, local, and CI bootstrap disagree.
- Automatic server-side initialization would require the long-running runtime
  process to receive schema-owner credentials. External raw-SQL initialization
  would create a second execution path whose transaction, concurrency, and
  digest behavior could drift from CI.
- The product has not shipped a stable v1. ADR 522 permits fresh database
  recreation, but it does not permit silent adoption, in-place upgrade, or
  destructive reset of an existing database.

## Current State

- `revaer-data::config::run_migrations` and `RuntimeStore::new` each construct a
  SQLx migrator from `crates/revaer-data/migrations`. With the native runtime
  enabled, normal application bootstrap reaches both paths.
- The Docker runtime stage copies `/usr/local/bin/revaer-app` and uses it as the
  entrypoint. A command added to that executable would be available in the
  existing image without introducing a second database image or toolchain.
- The Helm Deployment passes its database Secret only to the long-running
  application container. The same credential therefore currently covers both
  schema mutation and runtime stored-procedure access.
- `just db-migrate`, `just db-start`, coverage, development startup, PR jobs,
  Sonar, and Playwright temporary-database setup are coupled to the migration
  directory. The Playwright path is also outside the canonical `just` command
  surface required by repository policy.
- The migration corpus currently creates the `revaer_config` and
  `revaer_runtime` schemas and the `pgcrypto` and `unaccent` extensions. Its
  final replacement must preserve the required extension, ownership, grant,
  routine, trigger, seed, and comment state established by ADR 522 parity.

## Options

1. **Initialize from ordinary application startup.** The server would detect a
   fresh database, apply the embedded init script, and continue booting. This is
   convenient for a single process, but every runtime replica must be prepared
   to mutate schema, the runtime Secret retains elevated privileges, server
   startup becomes destructive-capable, and mixed-version races become part of
   the serving path.
2. **Add explicit init and verify modes to the packaged application binary.**
   The default server mode never creates schema. A one-shot invocation of the
   exact same binary applies its embedded `init.sql`; a read-only verify mode
   gates later starts. Docker operators invoke the command explicitly, and the
   Helm chart runs the same image as an init container before starting the
   application container.
3. **Require external or operator-managed SQL bootstrap.** Publish `init.sql`
   for a DBA, `psql`, Terraform, or a database service to apply out of band.
   This gives operators complete credential control, but it introduces an
   alternate runner, makes transaction and advisory-lock behavior optional,
   and cannot prove that development, CI, Docker, Helm, and operators executed
   the same baseline contract.
4. **Publish a separate initializer image or service.** This separates process
   roles, but creates another release artifact, signature, SBOM, image policy,
   compatibility surface, and lifecycle for logic that already belongs to the
   packaged application version.

## Recommendation

- Adopt option 2.
- Add exactly three top-level executable modes to `revaer-app`:
  - default server mode, which requires `DATABASE_URL`, verifies the packaged
    baseline, and never executes DDL;
  - `database init`, which is the only production-capable executor for the
    packaged `init.sql`; and
  - `database verify`, which performs the same non-mutating baseline check used
    by server startup.
- Keep `crates/revaer-data/init.sql` as the one authored SQL authority. The
  application artifact embeds its exact committed bytes and an SHA-256 digest
  over those bytes without newline or content normalization. No Rust source,
  shell recipe, workflow, container layer, or chart may carry a second SQL
  definition.
- Package and release validation must expose the expected digest for operator
  inspection, but the digest does not replace the signed application or image
  identity. A database baseline record proves which init bytes completed; it
  does not claim to detect arbitrary later catalog changes by a database
  superuser.

### Initialization Contract

- Revaer requires a dedicated PostgreSQL database for this v0 contract. The
  operator creates the database, the one-shot bootstrap principal, and the
  runtime principal outside `init.sql`; credentials and role creation are not
  committed or inferred by the application.
- `database init` receives a bootstrap connection separately from
  `DATABASE_URL`. The bootstrap connection is available only to the one-shot
  mode and is never retained by, forwarded to, or readable from server mode.
- Before classifying or changing the database, the initializer starts one
  transaction and obtains a deterministic transaction-scoped PostgreSQL
  advisory lock dedicated to Revaer baseline initialization. Every catalog
  check, init statement, grant, and baseline seal occurs while that lock and
  transaction are held.
- The initializer then classifies exactly one of these states:
  - **matching baseline:** the normalized singleton baseline record exists,
    has the supported contract version, and contains the exact 32-byte digest
    embedded in the binary; initialization succeeds without executing SQL;
  - **pristine database:** no baseline record and no user-created schema,
    relation, sequence, routine, type, trigger, or non-default extension exists
    outside the explicitly versioned pristine-catalog allowlist; initialization
    may proceed;
  - **unmanaged nonempty database:** no valid baseline record exists but any
    non-allowlisted user object exists; initialization fails without change;
  - **different or invalid baseline:** a baseline record is missing required
    fields, duplicated, unsupported, or has a different digest; initialization
    fails without change.
- The pristine allowlist is repository-versioned and limited to PostgreSQL
  system schemas, temporary schemas, the empty default `public` schema, and
  built-in objects present in the pinned clean PostgreSQL test image. Provider-
  specific or user-created objects are not silently accepted. Supporting such
  a database requires a separately reviewed allowlist change or a dedicated
  clean database.
- On a pristine database, the initializer executes the complete init script
  and writes the singleton baseline contract version and digest before commit.
  The baseline state is normalized relational data, not JSONB. It is owned by
  the schema owner and exposed to the runtime principal only through a bounded
  stored-procedure read contract.
- The final init script must be valid inside one PostgreSQL transaction. The
  freeze and cutover guard rejects explicit transaction control and statements
  that PostgreSQL cannot execute transactionally. Any statement error,
  cancellation, connection loss, or process termination rolls back the schema,
  grants, seed state, and baseline record together.
- After acquiring the lock, every waiter re-runs classification. Concurrent
  initializers carrying the same digest converge on one commit and matching
  no-op results. A different packaged digest loses the race and fails as a
  baseline mismatch; it never alters the winner's database.

### Runtime And Readiness Contract

- Server startup verifies the baseline before constructing `ConfigService`,
  `RuntimeStore`, watchers, workers, listeners, or health endpoints. Missing,
  unreadable, malformed, unsupported, or mismatched baseline state is a fatal
  startup error.
- `ConfigService` and `RuntimeStore` stop embedding or executing initialization
  logic. Their constructors receive an already verified runtime pool and retain
  only runtime stored-procedure privileges.
- The normal application principal cannot create, alter, drop, grant, revoke,
  install extensions, update the baseline record, or execute the initializer.
  Baseline verification is read-only and uses the stored-procedure boundary
  required for runtime database access.
- Logs and readiness evidence may expose the expected digest, observed digest,
  baseline contract version, classified state, mode, and bounded reason code.
  They must never expose either database URL, credentials, arbitrary catalog
  names, or raw SQL.

### Docker And Helm Contract

- The default Docker entrypoint remains server mode and never initializes a
  database. A fresh Docker deployment first invokes the same immutable image
  with the `database init` arguments and one-shot bootstrap Secret, then starts
  the image normally with only the runtime Secret.
- The Helm Deployment uses the exact same image identity for an init container
  and the application container. The init container always gates application
  startup with `database verify`; fresh initialization is an explicit chart
  setting that switches that one-shot container to `database init` and mounts a
  separately named existing bootstrap Secret.
- The bootstrap Secret is never rendered from an inline chart value, mounted
  into the application container, or copied into a shared volume. After first
  successful initialization, the operator disables init mode and deletes,
  rotates, or revokes the bootstrap credential. Subsequent pod starts use
  verify mode with the runtime Secret only.
- Multiple Helm replicas may run verify mode concurrently. If explicit init
  mode is accidentally started more than once, the database advisory lock and
  digest classification remain the authority; Kubernetes scheduling is not the
  concurrency control.
- External operators may run the same standalone binary or container command
  before deploying the server. Applying raw `init.sql` with `psql` is not a
  supported production bootstrap because it bypasses locking, classification,
  sealing, and package-digest verification.

### Development And CI Contract

- Replace `just db-migrate` with one canonical fresh-database initialization
  recipe that invokes `revaer-app database init`. Database reset recipes may
  drop and recreate only explicitly managed disposable local or CI databases,
  then call that same entry point.
- PR, Sonar, coverage, feature, native, and release-image validation call the
  canonical `just` recipe. Workflows may provision PostgreSQL and credentials,
  but may not invoke SQLx migrations, `psql`, or init SQL directly.
- Playwright global setup provisions each isolated temporary database and calls
  the canonical recipe or the exact packaged init mode through a shared test
  helper. It may not retain its current direct SQLx migration path.
- After cutover, SQLx migration embedding, migration-specific errors, build
  watches, CLI installation used only for migrations, `_sqlx_migrations`, and
  the migration directory are removed as required by ADR 522. SQLx remains
  available for ordinary runtime database access.

### Privilege Lifetime And Rollback

- Bootstrap credentials are short-lived operator inputs. The initializer keeps
  them only in process memory, never logs them, and closes the pool on every
  terminal path. The durable server process and its environment receive only
  the runtime credential.
- Successful initialization does not automatically drop an operator-owned
  principal because role ownership and extension administration can be
  provider-specific. The runbook requires immediate credential rotation,
  revocation, or deletion and proves that server and verify modes continue with
  only runtime privileges.
- There is no down migration or in-place baseline replacement in v0. A failed
  transaction is retried from pristine state. A committed baseline is rolled
  back only by restoring a verified full-database backup or recreating a
  disposable database from the exact prior artifact, consistent with ADR 522.
- Rolling the application image back is allowed only when the prior image
  expects the same baseline digest. If it expects another digest, its init,
  verify, and server modes fail closed. Helm rollback never rewrites the
  database and cannot convert a baseline mismatch into success.
- A nonempty database without a valid baseline is never reset automatically.
  Retaining or converting its data requires a separately Proposed and approved
  export/import or forward-migration decision.

## Consequences

- Schema mutation is removed from ordinary application startup and from the
  long-lived runtime principal.
- Docker, Helm, local development, tests, coverage, Sonar, and external
  operators can execute one version-bound initializer rather than several SQL
  runners.
- Fresh initialization is explicit and less convenient than unconditional
  startup self-init. Operators must provision a dedicated database and a
  short-lived bootstrap principal.
- The Helm chart gains an init-container and two-secret lifecycle that must be
  documented and tested. Leaving init mode enabled unnecessarily extends
  credential exposure, so post-init rotation is a required operating step.
- Strict pristine detection rejects shared or provider-decorated databases
  unless their extra objects receive explicit review. This favors safety over
  automatic adoption.
- Baseline mismatch prevents startup and rollback rather than attempting an
  unapproved upgrade or downgrade. Before v1, recreation remains the supported
  correction.

## Implementation Boundary

- Implementation is limited to:
  - same-binary `database init` and `database verify` routing;
  - exact-byte init embedding and SHA-256 baseline identity;
  - the normalized singleton baseline record and bounded read procedure;
  - transaction-scoped locking, pristine classification, atomic initialization,
    and fail-closed verification;
  - removal of application self-migration and migration-only dependencies at
    ADR 522 cutover;
  - Docker command documentation, the Helm init-container and separate Secret
    contract, canonical `just` bootstrap, and aligned CI/E2E paths; and
  - focused runbook, instruction, generated-documentation, and release evidence
    needed to describe and verify those exact behaviors.
- This ADR does not authorize remote database creation, role or password
  creation, automatic reset, nonempty database adoption, in-place schema
  upgrade, down migration, post-v1 migration design, a separate initializer
  image, raw operator SQL as a supported path, or long-lived bootstrap
  credentials.
- This ADR does not itself implement ADRs 533-537, fill any held
  numeric media limit, alter the final schema beyond baseline lifecycle state
  and grants, relax a required check or Sonar criterion, or exceed the stacked
  review-size limits.

## Exact Validation

- **Single authority:** prove the binary-embedded bytes equal the tracked
  `crates/revaer-data/init.sql` byte-for-byte and that the reported SHA-256
  equals an independently computed digest. Reject stale generated digest data,
  alternate SQL copies, a migration directory, and every direct migration or
  `psql` bootstrap call outside transition-only parity tooling.
- **Fresh database:** initialize the pinned clean PostgreSQL image from its
  pristine catalog, verify the baseline row and read procedure, prove the
  observed schema/seed/extension/grant catalog equals the ADR 522 reference in
  both directions, and prove `_sqlx_migrations` is absent.
- **Idempotency:** snapshot normalized catalog objects, seed rows, grants, and
  baseline state; rerun the same initializer and verify a no-op result with no
  row, object, timestamp, ownership, or privilege change.
- **Empty/nonempty classification:** cover an empty default database, every
  allowlisted system object, one extra schema, relation, sequence, routine,
  type, extension, malformed marker, duplicate marker, unsupported contract
  version, and missing marker on an otherwise initialized schema. Every
  unmanaged case must fail before mutation.
- **Digest mismatch:** initialize with baseline A, then run init, verify, and
  server modes from baseline B. All three must fail with a bounded mismatch
  reason, no SQL execution, no listener, and no ready health state.
- **Concurrency:** start multiple same-version initializers at a barrier and
  prove exactly one applies SQL while all others observe the matching baseline.
  Repeat with mixed baseline digests and prove only the committed version can
  succeed.
- **Transactional failure:** inject failure before the first statement, between
  representative schema/extension/table/routine/seed/grant sections, before the
  baseline seal, and before commit; also cancel and terminate the connection.
  Each case must release the lock, leave no partial user objects or baseline
  record, and permit a clean retry. A guard and live PostgreSQL test must reject
  every nontransactional or explicit transaction-control statement.
- **Privileges and secrets:** prove the documented bootstrap principal has no
  unnecessary cluster privilege, the runtime principal cannot perform DDL or
  mutate baseline state, and initialization fails under runtime credentials.
  Inspect rendered Helm resources and running process environments to prove the
  application container never receives the bootstrap Secret. Remove or revoke
  that Secret after initialization and prove verify and server modes still run.
- **Packaged paths:** start a fresh external PostgreSQL instance, initialize it
  through the release binary and each supported architecture image, then start
  server mode. Prove default container startup against a pristine database does
  not create objects. Render and execute the Helm fresh-init, matching-restart,
  concurrent-replica, absent-bootstrap-secret, and digest-mismatch cases with
  the init container using the exact application image identity.
- **Development and CI:** prove local startup, database reset, all database
  suites, Playwright temporary databases, coverage, Sonar, PR workflows, image
  verification, and release validation call the same `just` entry point and
  fail when initialization or verification fails.
- **Rollback:** prove a failed init retries from pristine state, a same-digest
  application rollback starts, a different-digest application rollback fails
  before serving, and verified backup restore or disposable recreation is the
  only documented committed-baseline rollback.
- **Full gates:** any accepted implementation must pass focused database and
  package tests, `just ci`, `just ui-e2e`, strict Sonar analysis with positive
  coverage, documentation generation and links, Helm/image validation, and
  test-media cleanup without suppressions or skipped checks.

## Follow-up

- Implement it in bounded outside-in stack slices: executable and
  package contract tests, baseline persistence and command modes, Docker/Helm
  lifecycle, local/CI/E2E convergence, then ADR 522 migration retirement.
- Record implementation evidence in separate Recorded task ADRs. Do not change
  this accepted boundary without another decision-specific operator approval.

## Task Record

- Motivation:
  - Resolve who initializes a packaged fresh database before ADR 522 removes the
    migration runner, without giving ordinary server startup schema privileges.
- Design notes:
  - The recommendation keeps one exact SQL authority and one version-bound
    executor while separating init and runtime credentials and process modes.
  - It treats Helm scheduling as orchestration only; PostgreSQL locking,
    classification, transactionality, and digest identity remain authoritative.
  - Implementation is authorized only within this accepted boundary.
- Test coverage summary:
  - This change is documentation-only and adds no Rust, SQL, container, Helm,
    CI, E2E, or database test.
  - Documentation generation, link, policy, instruction-drift, and diff checks
    are the applicable validation for this acceptance-only change.
- Observability updates:
  - No telemetry changes are made.
  - A future accepted implementation may emit only bounded mode, baseline
    digest, contract version, state, result, and reason evidence; credentials,
    URLs, raw SQL, and arbitrary catalog details remain prohibited.
- Status-doc validation:
  - Reviewed ADRs 030, 059, 482, 522, and the current application, Docker, Helm,
    release, local database, workflow, Sonar, and Playwright bootstrap paths.
  - Product and operator documentation remains unchanged because the lifecycle
    is not implemented. The ADR index and documentation summary expose the
    accepted decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior and needs no runtime
    rollback.
  - Transaction rollback handles uncommitted initialization;
    committed v0 rollback remains verified restore or disposable recreation,
    never an automatic downgrade.
- Dependency rationale:
  - No dependency is added. A future implementation must first use existing
    SQLx, PostgreSQL advisory locks and transactions, repository SHA-256
    tooling, and standard argument routing. Any new dependency requires its own
    written rationale in the implementing task ADR.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No quality, Sonar, stored-procedure, or approval rule is relaxed. The
    migration-specific instruction text still describes the implemented
    pre-cutover state; if this ADR is accepted, ADR 522 cutover must replace it
    with the single-init contract in the same implementation change.
