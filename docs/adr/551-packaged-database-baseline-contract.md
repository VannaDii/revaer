# Packaged database baseline contract

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Accepted ADR 522 makes `crates/revaer-data/init.sql` the sole pre-v1 database
  authority, and accepted ADR 541 selects explicit `database init` and
  `database verify` modes in the packaged application. Those decisions leave
  several implementation-significant values unspecified.
- The baseline row does not yet have an approved schema, table shape, contract
  version, stored-procedure signature, or ownership and privilege model.
- The assembly candidate has a frozen digest, but it still has an
  assembly-only header and does not contain the baseline lifecycle SQL. Sealing
  that digest and then changing either item would make the database record
  identify bytes that the final application no longer embeds.
- Candidate generation is pinned to PostgreSQL 16.14 in one immutable image,
  while PR, Sonar, E2E, and local database paths still use a mutable
  `postgres:16-alpine` tag. The pristine-database classifier has no exact
  supported server identity or committed catalog evidence.
- The bootstrap connection name, runtime-role input, advisory-lock identity,
  deadlines, cancellation behavior, and externally visible reason codes are
  not selected. Implementing any of those values without approval would make
  an architectural decision in code.
- This proposal closes those decision gaps only. It implements no SQL, Rust,
  workflow, image, Helm, or database behavior.

## Current Constraints

- The current assembly prefix is review evidence and is not executable by the
  application. The frozen `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`
  candidate digest covers the pre-lifecycle generated candidate, not the final
  packaged baseline.
- The generated candidate starts with an assembly-only marker and includes
  dump-session statements that set `statement_timeout`, `lock_timeout`, and
  `idle_in_transaction_session_timeout` to zero. Those statements cannot remain
  able to override the bounded initializer deadlines.
- The candidate creates objects in `public`, `revaer_config`, and
  `revaer_runtime`, installs `pgcrypto` and `unaccent`, and currently contains
  no final role-specific grant section.
- PostgreSQL 16.14 candidate generation uses
  `docker.io/library/postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`
  with locale `C`, encoding `UTF8`, data checksums enabled, and timezone `UTC`.
- ADR 541 forbids role creation, ordinary server DDL, raw operator bootstrap,
  nonempty-database adoption, alternate SQL authorities, and long-lived
  bootstrap credentials. Runtime database access remains stored-procedure-only.

## Options

1. **Use a marker in `public` and let the runtime role own the database.** This
   minimizes grants and permits invoker-rights functions, but gives the
   long-lived application principal schema mutation rights and contradicts ADR
   541's credential separation.
2. **Keep the baseline shape, role names, PostgreSQL minor version, pristine
   checks, and deadlines configurable.** This supports more providers, but each
   deployment would have a different safety contract and CI could not prove the
   production classifier or privilege boundary.
3. **Adopt one normalized baseline contract, one externally provisioned owner,
   one separately identified runtime role, one exact PostgreSQL identity, and
   bounded lifecycle behavior.** This is stricter operationally, but it makes
   all supported states reproducible and testable.
4. **Move baseline identity to an external control-plane service.** This avoids
   adding lifecycle state to the application database, but creates another
   authority and availability dependency and cannot atomically bind the SQL
   transaction to the recorded digest.

## Recommendation

- Adopt option 3 with every exact value and boundary below.
- This proposal becomes implementation authority only after decision-specific
  operator approval. Until then, the values below are recommendations, not
  permission to change behavior.

### Normalized Baseline State

- Add one schema named exactly `revaer_system`, owned by the externally
  provisioned database owner used by `database init`.
- Add one table named exactly `revaer_system.database_baseline` with these
  columns in this order:

  | Column | PostgreSQL type | Nullability | Source |
  | --- | --- | --- | --- |
  | `baseline_id` | `smallint` | `NOT NULL` | literal `1` |
  | `contract_version` | `smallint` | `NOT NULL` | initializer constant |
  | `init_sha256` | `bytea` | `NOT NULL` | exact embedded init-byte digest |
  | `postgres_version_num` | `integer` | `NOT NULL` | `current_setting('server_version_num')` |
  | `schema_owner_role` | `name` | `NOT NULL` | `session_user` during initialization |
  | `runtime_role` | `name` | `NOT NULL` | validated initializer input |
  | `sealed_at` | `timestamptz` | `NOT NULL` | `transaction_timestamp()` |

- Apply exactly these constraints:
  - primary key `database_baseline_pkey (baseline_id)`;
  - check `database_baseline_singleton` requiring `baseline_id = 1`;
  - check `database_baseline_contract_v1` requiring
    `contract_version = 1`;
  - check `database_baseline_sha256_length` requiring
    `octet_length(init_sha256) = 32`;
  - check `database_baseline_postgres_16_14` requiring
    `postgres_version_num = 160014`;
  - check `database_baseline_owner_nonempty` requiring
    `length(schema_owner_role::text) BETWEEN 1 AND 63`;
  - check `database_baseline_runtime_nonempty` requiring
    `length(runtime_role::text) BETWEEN 1 AND 63`; and
  - check `database_baseline_distinct_roles` requiring
    `schema_owner_role <> runtime_role`.
- Do not add a surrogate UUID, mutable status, update timestamp, JSON value,
  free-form metadata, environment name, application version, or database OID.
  The singleton identity, exact bytes, admitted PostgreSQL identity, principals,
  and seal time are the complete durable state. Excluding a database OID keeps
  a verified full-database restore possible.
- The baseline contract version is exactly the integer `1`. A future shape or
  semantic change requires a new accepted ADR and a new procedure suffix; it is
  not inferred from the application version or init digest.

### Stored-Procedure Contract

- Add exactly this owner-only sealing function:

  ```sql
  revaer_system.seal_database_baseline_v1(
      contract_version_input smallint,
      init_sha256_input bytea,
      runtime_role_input text
  ) RETURNS TABLE (
      contract_version smallint,
      init_sha256 bytea,
      postgres_version_num integer,
      schema_owner_role name,
      runtime_role name,
      sealed_at timestamptz
  )
  ```

- The sealing function is `VOLATILE`, `SECURITY INVOKER`, and callable only by
  the schema owner. It requires `session_user = current_user`, requires the
  caller to own the current database and `revaer_system`, derives the owner and
  PostgreSQL version instead of accepting them as parameters, rejects an
  existing row, validates the three input values against this ADR, applies the
  approved grants, inserts the singleton, and returns the inserted values.
- The runtime role lookup is an equality lookup against `pg_roles.rolname`
  using a bound `text` value, avoiding the silent truncation semantics of the
  PostgreSQL `name` input type. Rust never concatenates an identifier. The seal
  quotes the resolved role identifier with PostgreSQL's identifier formatter
  only for the grant statements contained in the authoritative init function.
- Add exactly this bounded read function:

  ```sql
  revaer_system.read_database_baseline_v1() RETURNS TABLE (
      contract_version smallint,
      init_sha256 bytea,
      postgres_version_num integer,
      schema_owner_role name,
      runtime_role name,
      sealed_at timestamptz
  )
  ```

- The read function has no parameters, is `STABLE` and `SECURITY DEFINER`, is
  owned by the schema owner, and has the fixed search path
  `pg_catalog, revaer_system`. It returns exactly one row or raises a bounded
  invalid-baseline error; it never returns zero or multiple rows as successful
  absence. It also requires `session_user` to equal either the recorded schema
  owner or recorded runtime role. This admits an idempotent owner-side init
  check and runtime verification while rejecting an unrelated login or
  `SET ROLE` surrogate.
- Revoke all authored application and baseline function execution from `PUBLIC`.
  Grant the runtime role execute only on `read_database_baseline_v1` within
  `revaer_system`; never grant it execute on
  `seal_database_baseline_v1`.
- The application executes the exact tracked init bytes through the existing
  SQLx `raw_sql` API inside one explicit SQLx transaction. It calls the seal
  function with bound parameters in that same transaction. This transport is
  an implementation detail, not a second SQL authority and not permission to
  split, rewrite, or normalize the init script at runtime.

### Owner, Bootstrap, And Runtime Roles

- For v0, the schema owner and one-shot bootstrap login are the same PostgreSQL
  role. The operator creates a dedicated database owned by that role. The role
  must be `LOGIN` only while bootstrap access is needed and must be
  `NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS`.
- `database init` verifies that `session_user = current_user`, that this role is
  the owner of `current_database()`, and that no role substitution is active.
  Database ownership supplies the ability to create in `public`, create the two
  trusted extensions, create the application schemas, and own every object
  without a committed role name or `CREATE ROLE` statement.
- The runtime role is separately created by the operator and supplied to
  `database init` through exactly `REVAER_RUNTIME_DATABASE_ROLE`. The value is a
  UTF-8 PostgreSQL role name of 1 through 63 bytes, is resolved by equality in
  `pg_roles`, and is stored exactly as PostgreSQL resolves it. The role must be
  `LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS`, must
  not be the schema owner, and must not be a direct or indirect member of the
  schema-owner role.
- No application mode creates, alters, drops, renames, or sets a password for a
  role. After initialization, the operator revokes the bootstrap credential or
  changes the owner role to `NOLOGIN`; the durable owner role remains because
  security-definer routines and restored objects require a stable owner.
- Apply this exact privilege matrix after revoking inherited defaults:

  | Capability | `PUBLIC` | schema owner/bootstrap | runtime role |
  | --- | --- | --- | --- |
  | Database `CONNECT` | revoked | implicit as owner | granted |
  | Database `CREATE` | revoked | implicit as owner | none |
  | Database `TEMPORARY` | revoked | implicit as owner | none |
  | Own application schemas and objects | none | all owner rights | none |
  | Schema `USAGE` on `public`, `revaer_config`, `revaer_runtime`, `revaer_system` | none | owner rights | granted |
  | Schema `CREATE` | none | owner rights | none |
  | Tables and sequences | none | owner rights | none |
  | User-defined type `USAGE` in application schemas | none | owner rights | granted |
  | Non-trigger application routine `EXECUTE` | none | owner rights | granted |
  | Baseline read `EXECUTE` | none | owner rights | granted |
  | Baseline seal `EXECUTE` | none | owner rights | none |
  | Extension, grant, revoke, or DDL capability | none | owner rights during explicit init | none |

- Every routine granted to the runtime role must be owned by the schema owner,
  use `SECURITY DEFINER`, and declare a fixed search path containing only the
  minimum of `pg_catalog`, `public`, `revaer_config`, and `revaer_runtime` that
  the routine needs. `pg_temp` is never included. Trigger functions are not
  directly granted. The runtime role receives no table or sequence privilege.
- The final init revokes default `PUBLIC` database, schema, type, and function
  privileges and sets schema-owner default privileges so later direct pre-v1
  init edits cannot restore implicit `PUBLIC` execution or type usage.
- A generated privilege proof may enumerate routine identities for validation,
  but it is not an alternate authored allowlist. The explicit grants and routine
  definitions in `init.sql` remain the sole database authority.

### Final Init Digest Sequence

- Preserve the current assembly digest only as transition evidence. It must
  never be written to `database_baseline`.
- Perform finalization in this exact order:
  1. assemble all 1,624 statement-aligned candidate statements and verify the
     frozen candidate digest;
  2. add the approved `revaer_system` schema, baseline table, lifecycle
     functions, ownership hardening, routine security, and grants to
     `init.sql`;
  3. replace the assembly-only first-line marker with the stable final header
     `-- Revaer pre-v1 packaged database baseline.`;
  4. remove the dump-generated zero values for `statement_timeout`,
     `lock_timeout`, and `idle_in_transaction_session_timeout`, and reject any
     later init statement that changes those three controls;
  5. run transaction-safety and fresh-apply validation; prove two-way parity for
     legacy object definitions, extensions, and seed state; prove that the only
     ownership, routine-security, privilege, and lifecycle deltas are the exact
     additions approved by this ADR; then run full application validation;
  6. compute SHA-256 over the exact final `init.sql` bytes with no newline,
     encoding, or content normalization;
  7. record that final digest as the packaged baseline digest and embed the same
     32 bytes in the application artifact; and
  8. prohibit every post-freeze byte change. A changed init file starts a new
     reviewed baseline and digest; it never edits an already sealed baseline.
- This ordering supersedes only ADR 522's statement that the assembly marker is
  removed during the later transition-machinery cleanup. The marker is removed
  before the first final digest can be frozen or sealed. ADR 522's single-init,
  bounded-stack, parity, cutover, migration-retirement, and final-cleanup rules
  remain unchanged.
- The digest is supplied to the seal function as a bound parameter. It is not
  written into `init.sql`, avoiding a self-referential digest. The lifecycle SQL
  itself is inside the digest.
- The migration corpus remains authoritative only through parity and the atomic
  cutover allowed by ADR 522. After final cleanup, no migration file, runner,
  reference, or compatibility bootstrap remains.

### PostgreSQL Identity And Pristine Catalog

- The initial v0 baseline supports exactly PostgreSQL 16.14:
  - `server_version_num` must equal `160014`;
  - the numeric admission range is `160014 <= server_version_num < 160015`;
  - `server_encoding` must equal `UTF8`;
  - database `lc_collate` and `lc_ctype` must equal `C`;
  - `integer_datetimes` and `standard_conforming_strings` must equal `on`; and
  - data checksums must equal `on` for every admitted database.
- The singleton range is intentional. Accepting another PostgreSQL patch,
  major, locale, encoding, or checksum posture requires a reviewed version-pin
  update, a new pristine snapshot, and the complete compatibility suite. It does
  not permit a mutable image tag or an unchecked provider-reported version.
- Until transition cleanup renames the keys without changing values,
  `.github/build-inputs.env` remains the one version source:
  - `POSTGRES_REBASELINE_VERSION=16.14`; and
  - `POSTGRES_REBASELINE_IMAGE=docker.io/library/postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`.
- PR, Sonar, coverage, feature-matrix, API E2E, UI E2E, local managed database,
  candidate generation, and release validation must all consume that exact
  digest and assert the exact server and client version. Mutable
  `postgres:16-alpine` references are removed. Final transition cleanup may
  rename the two keys to general PostgreSQL validation names only in one atomic
  no-value-change edit with guardrail coverage.
- Commit a mechanically generated pristine catalog snapshot at exactly
  `config/postgres-pristine-16.14.tsv`. Generate it only from the pinned image,
  while connected as a constrained non-superuser database owner matching the
  bootstrap role contract rather than as the image's initial superuser,
  with database and role identities replaced by the fixed tokens
  `<database>` and `<database_owner>`, UTF-8, LF endings, tab-separated fields,
  and bytewise `C` sorting. The implementation task record records its SHA-256.
  The snapshot is evidence derived from the image, not a hand-maintained list.
- The snapshot includes normalized identities and security-relevant attributes
  from these exact catalogs: `pg_namespace`, `pg_extension`, `pg_class`,
  `pg_proc`, `pg_type`, `pg_trigger`, `pg_event_trigger`, `pg_language`,
  `pg_cast`, `pg_collation`, `pg_conversion`, `pg_operator`, `pg_opclass`,
  `pg_opfamily`, `pg_amop`, `pg_amproc`, `pg_ts_config`, `pg_ts_dict`,
  `pg_ts_parser`, `pg_ts_template`, `pg_foreign_data_wrapper`,
  `pg_foreign_server`, `pg_user_mapping`, `pg_policy`, `pg_publication`,
  `pg_publication_rel`, and `pg_subscription`. OIDs, statistics, transaction
  IDs, filesystem locations, and session-local temporary object identities are
  excluded.
- Before executing init SQL, classification occurs under the advisory lock and
  applies these fail-closed predicates in order:
  1. the PostgreSQL identity and database settings above match exactly;
  2. the current login is the database owner and the runtime role passes the
     role constraints above;
  3. a structurally valid `revaer_system.database_baseline` is read and compared;
     a matching row ends classification successfully, while any mismatch or
     malformed lifecycle object fails; only complete absence proceeds to the
     pristine-only predicates below;
  4. when lifecycle state is absent, the only non-temporary schemas are
     `pg_catalog`, `information_schema`,
     `pg_toast`, and an empty `public`; temporary schema names may match only
     `pg_temp_[0-9]+` and `pg_toast_temp_[0-9]+`;
  5. the only extension is `plpgsql` version `1.0` in `pg_catalog`;
  6. `public` contains no relation, sequence, routine, user-defined type,
     trigger, policy, collation, conversion, operator, text-search object,
     foreign-data object, publication, or subscription; and
  7. the normalized system-catalog snapshot equals the committed pinned-image
     snapshot byte-for-byte.
- Any missing query result, duplicate baseline row, malformed column or
  constraint, snapshot difference, extra object, provider decoration,
  unsupported extension, unreadable catalog, or classification race is
  non-pristine and fails before `raw_sql` executes. No broad provider allowlist,
  object deletion, or automatic adoption is permitted.

### Inputs, Lock, And Deadlines

- `database init` reads its connection only from exactly
  `REVAER_DATABASE_BOOTSTRAP_URL`. It requires
  `REVAER_RUNTIME_DATABASE_ROLE` and rejects a present `DATABASE_URL` so the
  one-shot process cannot accidentally receive the long-lived runtime Secret.
- Server and `database verify` read only `DATABASE_URL` and reject a present
  `REVAER_DATABASE_BOOTSTRAP_URL`. Server derives its expected login identity
  from the connection and the read function verifies it against the sealed
  `runtime_role`; no second runtime-role environment value is accepted.
- Never log, emit, persist, hash, or include either URL in an error. Pools created
  for init and verify have exactly one connection and are closed on every
  terminal path.
- Use the one-argument transaction-scoped advisory lock key
  `5896060463355476277`. This is the unsigned big-endian value of the first
  eight bytes `51d303c0b4873935` of SHA-256 over the ASCII domain string
  `revaer.database.baseline.v1`. The derivation makes the namespace reviewable;
  the stored integer is the authority and is not recomputed at runtime.
- Apply these exact mode deadlines:

  | Boundary | `database init` | `database verify` and server startup |
  | --- | ---: | ---: |
  | Connection establishment | 10 seconds | 10 seconds |
  | Pool acquisition | 10 seconds | 10 seconds |
  | Advisory-lock wait | 120 seconds | not applicable |
  | PostgreSQL statement timeout | 120 seconds per top-level statement | 10 seconds |
  | Idle-in-transaction timeout | 30 seconds | not applicable |
  | Whole operation | 600 seconds | 30 seconds |
  | Cancellation cleanup | 10 seconds | 10 seconds |
  | Indeterminate-commit resolution | 30 seconds | not applicable |

- Ten seconds bounds normal connection and read failures without making server
  readiness wait indefinitely. The 120-second lock and statement bounds allow a
  concurrent initializer to finish on hosted validation infrastructure. The
  600-second outer limit contains the complete 1,624-statement baseline with
  fault-detection margin while remaining well below workflow timeouts.
- Do not automatically retry connection, statement, transaction, or seal
  failures within one invocation. A same-artifact rerun is the explicit retry
  and reclassifies under the lock.

### Cancellation And Bounded Results

- SIGINT, SIGTERM, task cancellation, and whole-operation timeout latch a
  cancellation state. After the latch, the initializer starts no new statement,
  requests cancellation of the active PostgreSQL query, rolls back the explicit
  transaction, closes the single-connection pool, and exits nonzero.
- If cancellation arrives before commit is sent, success is impossible. If the
  connection is lost after commit is sent but before its result is known, the
  initializer reconnects once within the 30-second resolution bound and reads
  the baseline. It reports success only for the exact expected row; it reports
  mismatch for a valid different row; otherwise it exits with an indeterminate
  result. It never reruns SQL in the same invocation.
- Process termination that prevents cleanup still relies on PostgreSQL to roll
  back an uncommitted disconnected transaction. A later invocation always
  reclassifies; it never assumes rollback or commit from process-local state.
- Emit exactly one of these reason codes. The set is closed and versioned with
  contract version 1:
  - success: `matching_baseline`, `initialized_baseline`;
  - input and identity: `invalid_configuration`,
    `postgres_identity_unsupported`, `bootstrap_identity_invalid`,
    `bootstrap_privilege_invalid`, `runtime_role_invalid`;
  - connectivity and bounds: `database_connect_timeout`,
    `database_connect_failed`, `pool_acquire_timeout`,
    `baseline_lock_timeout`, `statement_timeout`;
  - classification and verification: `database_not_pristine`,
    `baseline_shape_invalid`, `baseline_contract_unsupported`,
    `baseline_digest_mismatch`, `baseline_runtime_role_mismatch`,
    `baseline_read_denied`;
  - transaction lifecycle: `statement_failed`, `cancelled`,
    `transaction_rollback_failed`, `transaction_commit_failed`,
    `cleanup_failed`, `commit_outcome_unknown`.
- Logs and readiness may include only mode, expected and observed digest in
  lowercase hexadecimal, contract version, PostgreSQL version number, result,
  and one reason code. They may include a PostgreSQL SQLSTATE and a fixed stage
  enum, but never a URL, credential, raw SQL, arbitrary object name, dynamic
  server message, or role name.

## Consequences

- A sealed baseline has one unambiguous relational shape and identifies the
  exact final init bytes, PostgreSQL identity, owner, runtime role, and seal
  instant.
- The long-lived runtime credential cannot mutate schema or baseline state and
  has no direct table or sequence privileges. This requires every runtime-callable
  routine to have an audited security-definer boundary and fixed search path.
- The effective initial PostgreSQL support set is deliberately one exact patch
  release and one pristine catalog. Managed providers with decorations, other
  locales, or automatic minor upgrades are unsupported until reviewed evidence
  is added.
- The bootstrap role remains as a durable no-login owner after its credential is
  revoked. Removing that role would break object ownership and is not part of
  credential cleanup.
- A final baseline digest cannot be known until lifecycle SQL, privilege
  hardening, timeout-preserving cleanup, and header finalization are complete.
  Existing assembly digests remain useful only for transition integrity.
- The strict timeout and cancellation contract may fail under unusually slow
  infrastructure rather than wait indefinitely. Operators rerun after removing
  the cause; agents may not silently increase a value.

## Implementation Boundary

- If accepted, this ADR authorizes only:
  - the exact baseline schema, table, constraints, contract version, and two
    stored-procedure contracts above;
  - the exact externally provisioned owner/bootstrap and runtime role model and
    privilege matrix;
  - final init-byte sequencing and the narrow sequencing correction to ADR 522;
  - PostgreSQL 16.14 admission, one pinned pristine snapshot, and replacement of
    mutable validation image tags with the existing digest-qualified input;
  - the two exact environment names, advisory-lock key, deadlines,
    cancellation behavior, and reason-code set; and
  - the focused implementation and validation needed to prove those contracts
    through the already accepted ADR 522/541 cutover.
- This ADR does not authorize role creation, password management, a provider
  exception, a broader PostgreSQL range, object adoption or deletion, runtime
  DDL, schema upgrade, down migration, a second SQL copy, statement splitting,
  a separate initializer, raw `psql` bootstrap, or a retained migration runner
  after final cleanup.
- This ADR does not relax, remove, skip, rename, or make optional any required
  GitHub check, Sonar property, analyzer, coverage input, quality-gate condition,
  policy guardrail, security scan, or stacked-review line limit.
- The current documentation-only change authorizes no implementation. Any value
  not written above remains undecided and requires a Proposed ADR before code.

## Exact Validation

- **Documentation proposal:** run the policy suite, instruction-drift check,
  documentation index generation, documentation build, full documentation link
  check, and Git diff checks. Do not run implementation gates for this
  documentation-only proposal.
- **Schema shape:** compare `pg_catalog` evidence for the exact schema, table,
  column order/types/nullability, named constraints, function argument and
  result signatures, volatility, security mode, owner, search path, and grants.
  Reject every extra baseline column, row, overload, or privilege.
- **Contract version:** prove only literal version 1 seals and reads; zero,
  negative, null, and every other positive value fail without mutation.
- **Role matrix:** initialize with a database-owner bootstrap login and a
  separate constrained runtime login, revoke or disable bootstrap login, and
  prove runtime init, DDL, table DML, sequence use, seal, grant, extension, and
  owner-role membership all fail. Prove every real application stored-procedure
  workflow still passes with runtime credentials alone.
- **Routine security:** enumerate every runtime-executable routine and prove it
  is owner-owned, security-definer, non-trigger, fixed-search-path, and covered
  by call-path tests. Mutations that add invoker rights, `pg_temp`, table grants,
  default `PUBLIC` execute, or an ungranted caller must fail policy tests.
- **Digest order:** prove the 1,624-statement candidate before finalization,
  prove the stable header and lifecycle SQL precede digest calculation, reject
  timeout-reset statements, independently recompute exact bytes, and fail any
  build or package with stale digest evidence. Prove no byte changes occur
  between final digest generation, application embedding, and baseline seal.
- **Parity boundary:** compare all legacy object definitions, extensions, and
  seed state in both directions. Reject every difference except the exact
  `revaer_system`, owner, routine-security, default-privilege, and runtime-grant
  deltas approved here, and validate each approved delta independently.
- **PostgreSQL pin:** assert server and client 16.14, the exact image digest,
  locale, encoding, required settings, and checksum posture in candidate, PR,
  Sonar, E2E, coverage, local, and release paths. Guardrails must reject mutable
  tags, duplicate pins, or a workflow-local override.
- **Pristine classifier:** test the exact clean snapshot; each allowed temporary
  schema form; and one-at-a-time mutations across every listed catalog. Test an
  extra schema, relation, sequence, routine, type, trigger, extension, role
  substitution, provider object, malformed baseline, duplicate row, and missing
  read function. Every case fails before the first init statement.
- **Atomicity:** through SQLx `raw_sql` in one explicit transaction, inject
  failures before the first statement, between representative extension,
  schema, table, routine, seed, privilege, and lifecycle sections, before seal,
  and before commit. Each leaves no partial user state and permits a clean rerun.
- **Concurrency and bounds:** race same- and different-digest initializers;
  force connect, acquire, advisory-lock, statement, outer, cleanup, and
  indeterminate-commit timeouts; and verify exact elapsed bounds and reason
  codes without credential or dynamic-error leakage.
- **Cancellation:** deliver SIGINT, SIGTERM, task cancellation, connection loss,
  and process termination at each transaction stage. Prove no post-cancellation
  statement starts, rollback or bounded outcome resolution occurs, and a later
  invocation safely reclassifies.
- **Reason-code closure:** compile reason codes as a closed enum, exercise every
  variant, reject arbitrary strings, and snapshot the operator-visible mapping.
- **Cutover and cleanup:** retain migration parity only until the accepted atomic
  cutover, then prove every database-backed local and CI path uses packaged init.
  Final cleanup must prove the migrations directory, migration runner, SQLx
  migration metadata, and alternate bootstrap references are absent.
- **Full implementation gates:** any later accepted implementation must pass
  focused database/package/image/Helm tests, `just ci`, `just ui-e2e`, strict
  Sonar with positive coverage and retained evidence, all required GitHub
  checks, documentation generation and links, and test-media cleanup.

## Operator Questions

1. Do you approve contract version `1`, schema `revaer_system`, the exact
   `database_baseline` shape and constraints, and the exact seal/read signatures?
2. Do you approve making the one-shot bootstrap login the dedicated database
   and schema owner, retaining it as `NOLOGIN` after bootstrap, and supplying the
   separate runtime login through `REVAER_RUNTIME_DATABASE_ROLE` with the exact
   grant matrix above?
3. Do you approve finalizing the header, lifecycle SQL, routine security, grants,
   and timeout-reset removal before computing the first sealable digest, narrowly
   superseding ADR 522's later marker-removal timing?
4. Do you approve the initial effective support set of exactly PostgreSQL 16.14,
   the current immutable image digest for every PR/Sonar/local validation path,
   and fail-closed admission against a generated committed pristine snapshot?
5. Do you approve `REVAER_DATABASE_BOOTSTRAP_URL`, the advisory-lock key
   `5896060463355476277`, every listed deadline, cancellation behavior, and the
   closed reason-code set?
6. Do you approve the implementation boundary that preserves all required
   checks and Sonar strictness and requires complete migration removal after the
   ADR 522 cutover?

## Follow-up

- Present this Proposed ADR to the operator. Do not implement any recommended
  value until the operator provides explicit decision-specific approval.
- If accepted, implement the work in bounded outside-in stack slices: final-init
  proof, verified pool boundary, baseline lifecycle and privilege tests,
  packaged command modes, Docker/Helm and CI convergence, atomic cutover, then
  migration retirement and transition cleanup.
- Record each implementation slice in a separate task ADR with exact evidence.
  A requested change to any value in this proposal requires updating the
  proposal before approval or a later superseding Proposed ADR.

## Task Record

- Motivation:
  - Close the concrete baseline, role, digest, PostgreSQL, classifier, and
    lifecycle choices that ADRs 522 and 541 intentionally left for operator
    approval before implementation.
- Design notes:
  - The recommendation uses one exact database identity and catalog snapshot so
    pristine classification is evidence-based rather than heuristic.
  - The digest is finalized only after all executable bytes are stable and is
    passed to the seal function, avoiding both stale identity and self-reference.
  - Keeping the bootstrap login as the durable no-login owner avoids role
    creation and `SET ROLE` complexity while preserving least-privilege runtime
    access through audited security-definer procedures.
- Test coverage summary:
  - This change is documentation-only. It adds no SQL, Rust, workflow, image,
    Helm, database, unit, integration, coverage, or E2E behavior.
  - Applicable validation is limited to policy, instruction drift,
    documentation indexing/build/links, and Git diff checks.
- Observability updates:
  - No runtime telemetry changes are made.
  - The proposal defines a future closed reason-code surface and explicitly
    excludes URLs, credentials, raw SQL, arbitrary catalog names, dynamic server
    messages, and role names from logs and readiness evidence.
- Status-doc validation:
  - Reviewed accepted ADRs 522 and 541, ADR 528, the current rebaseline config,
    pinned build inputs, init assembly header, application bootstrap, database
    recipes, PR/Sonar services, Helm database values, and scoped instructions.
  - Product and operator guides remain unchanged because no behavior is
    implemented. The ADR index, mdBook summary, and generated LLM index expose
    the proposal.
- Risk & rollback plan:
  - The proposal changes no runtime behavior. Rollback is removal or revision of
    this unapproved ADR and its catalogue entries.
  - If later accepted implementation fails before cutover, revert the bounded
    implementation slices. After a committed v0 baseline, rollback remains only
    verified full-database restore or disposable recreation with a same-digest
    artifact; no in-place downgrade is authorized.
- Dependency rationale:
  - No dependency is added. The recommendation uses existing SQLx `raw_sql`,
    SQLx transactions and pools, PostgreSQL catalogs/advisory locks/procedures,
    repository SHA-256 tooling, and canonical `just` recipes.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No stale-policy contradiction was found for a documentation-only proposal.
    No required check, Sonar criterion, stored-procedure rule, migration freeze,
    dependency rule, or review-size limit is relaxed.
