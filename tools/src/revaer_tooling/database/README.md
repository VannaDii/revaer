# Database transition tooling

This package supports Revaer's reviewed database rebaseline. It is operational
tooling for the selected checkout; it does not initialize an operator's database
or change application database access rules.

## Commands

Run these from the checkout whose reviewed transition inputs you intend to use.
The installed launcher selects that checkout's locked implementation.

| Command | Behavior |
| --- | --- |
| `rv db-rebaseline-freeze` | Verify migration filenames/bytes, pinned inputs and the initializer's phase constraints. |
| `rv db-rebaseline-candidate` | Replay migrations with SQLx, normalize through PostgreSQL, replay the candidate and compare its schema. |
| `rv db-init-prefix-check` | Check an assembly prefix, or the exact reviewed final initializer during finalization. |
| `rv db-init-finalize` | Regenerate the already-defined routine deltas/security/grants around the authored lifecycle section. |

The finalization generator requires an existing lifecycle section with unique
markers. It leaves reviewed hash pins for human review. It does not invent a new
lifecycle, grant policy or accepted initializer hash.

These commands require the checkout's transition manifest. The media integration
supplies it; the earlier foundation does not have that transition. The
[pristine catalog commands](pristine/README.md) need only the pinned PostgreSQL
image and work in either checkout. Complete final-runtime proof commands remain
tracked in the [migration inventory](../../../migration.md). Accepted media ADR 591
supersedes legacy-equivalence acceptance for v0. Do not finish historical D3
qualification as a migration prerequisite: integrate the retained baseline,
extension and reset checks with fresh-init, security, recovery and actual
application qualification instead. Existing ingestion components are historical
work, not a complete public command or evidence of v0 readiness.

## Feature development

The media checkout uses `TRANSITION_PHASE=feature-development`. Freeze validation
continues to enforce the exact historical migration corpus and reviewed input
formats. It requires a regular initializer with scannable SQL statements, while
allowing feature edits beyond the historical finalization hash. It never updates
that hash. Candidate/prefix/finalization commands retain their separate exact-byte
contracts; a successful freeze is not proof of runtime behavior or legacy parity.

## Changed-line limits

`rv stack-changed-lines --base REF --head REF` and
`rv db-init-assembly-changed-lines --base REF --head REF` resolve both inputs to
commits once, then count the entire native Git diff. They preserve the reviewed
limits in `config/database-rebaseline.env`. Both additions and deletions count;
renames are counted as deletion plus addition. External diff drivers and text
conversion are disabled, and NUL-delimited paths preserve tabs and newlines.

Uncountable binary entries fail. The stack command retains exactly the existing
ADR 588 ASSET-1 exception: all 213 approved deletions, original modes, Git blob
identities, byte counts and SHA-256 digests must match. The complete inventory's
compact JSON hash and order remain fixed. Text must still fit its normal limit.
The assembly command has no binary exception.

ASSET-1 also requires the exact PR 130 repository/branch/base/head identities and
fresh, complete, unfiltered GitHub timeline pages. Closing, merging or reopening
permanently expires it. Provider errors, duplicate JSON keys/events, changing
identity/counts and incomplete/repeated pages fail. No prior receipt is reused.
The checked head must contain the current guard, dispatch, transport, decision,
inventory and instruction bytes listed in `assets.py`. The Python path list
replaces the old Ruby implementation paths; it does not extend the exception's
scope, PR identity or lifetime.

| Module | Responsibility |
| --- | --- |
| `changed_lines.py` | Complete text counts, exact revisions and limit ordering. |
| `assets.py` | Approved inventory, raw Git metadata, original blobs and committed implementation. |
| `asset_history.py` | Fresh provider identity and full expiration history. |
| `../external/git.py`, `../external/github.py` | Typed native Git/gh invocations. |

The shared process runner captures arbitrary Git blob bytes without decoding or
printing them. Diagnostics retain the usual separate redacted text path. The
native regression reconstructs the exact historical deletion set in a disposable
repository; its provider responses are synthetic. A passing fixture is not
canonical acceptance for a real PR.

## Rust application qualifications

`rv test-database-baseline-read` runs the existing `baseline::` test selection
across the entire workspace with all features and warnings denied. It requires
the caller's explicit disposable test database, like the other Rust test tasks.

The final initializer proof also invokes two exact Rust library tests:

| Command | Required input environment variable |
| --- | --- |
| `rv db-init-pool-probe` | `REVAER_INGESTION_POOL_PROOF` |
| `rv db-init-cancellation-probe` | `REVAER_INGESTION_CANCELLATION_PROOF` |

Each value names the private input file produced by the owning proof process.
The CLI supplies no database default. Rust continues to validate its input
schema, literal loopback endpoint, database/role identity and fresh report paths.
rv uses the exact qualified test name, `--lib`, `--all-features`, `--exact` and
`--show-output`, and rejects Cargo's otherwise-successful zero-test result.
Missing or failed qualifications fail the command. These entry points preserve
the Rust application checks; their complete database producer is still being ported.

## Authority and immutable inputs

`config/database-rebaseline.env` pins the migration corpus, transition phase,
candidate hash and statement count, final initializer hash and changed-line
limits. `.github/build-inputs.env` pins the PostgreSQL image and release.
Validation never updates those inputs to make a failed proof pass.

Corpus hashing includes each sorted repository-relative filename, a NUL, its
exact file bytes and another NUL. No SQL formatter, newline conversion or parser
serialization participates in that hash.

## Statement boundaries

`statements.py` uses the locked `pglast` 6.16 PostgreSQL scanner. PostgreSQL owns
recognition of strings, dollar quotes and nested comments. The adapter converts
documented Unicode token indexes to UTF-8 offsets and retains the existing rule:
a terminating semicolon includes following tabs, spaces or CR and the next LF
only when nothing else occurs before that LF. Line numbers identify the
semicolon, starting at one.

The scanner is intentionally separate from execution. It can identify a complete
lexical boundary in syntactically invalid SQL; the pinned PostgreSQL 16.14 server
must still accept candidate and final SQL in their disposable proof databases.
No generated SQL is trusted merely because it has the expected statement count.

The dependency loads only when database tooling needs it. Container build/runtime
commands can continue to use the core uv dependency group without a SQL parser.

## Validation

The initial read-only migration comparison matched 5,267 boundaries across 169
files against the previous Ruby implementation, including UTF-8 byte counts and
line numbers. The comparison artifact records the hash of every source file.
Focused tests cover quoted/commented semicolons, Unicode, incomplete tokens and
exact prefix boundaries. This comparison does not replace native database proofs.

The final SQL comparison also matched the Ruby output for all 532 routines:
approved legacy deltas, security statements, grants and complete regenerated
initializer bytes. The reviewed initializer SHA-256 remains
`c16d7edc64765393bf0014556ed1da00c839fdc712ab72a13c412d84b62335a8`.

A native run on a disposable copy of all 167 frozen migrations reproduced the
candidate, statement map and evidence file byte for byte. SQLx 0.8.6 performed
migration replay; the pinned PostgreSQL 16.14 image performed both subsequent
transactional applications and schema dumps. Its container and data volume were
removed and checked after the run. This proves candidate generation; the complete
final initializer runtime/ingestion proof remains a separate migration requirement.

## Final proof baseline stage

`baseline.py` receives the owned connection, constrained owner/runtime principals,
exact initializer SHA-256 and check collector. Its original 53 lifecycle checks cover invalid seal
inputs, empty/repeated seals, exact baseline shape and digest, seven malformed
read cases, direct runtime privilege denials, surrogate-role rejection, stored
procedure call paths, post-seal transaction rollback, initializer timeout
preservation and runtime reads after bootstrap login is disabled.

Negative checks require a nonzero result, the actual PostgreSQL error SQLSTATE,
and the exact expected DETAIL when specified. A matching word in unrelated
diagnostics does not count. Warnings cannot satisfy an expected denial. The
unauthorized schema GRANT is a distinct PostgreSQL warning case: its diagnostic
and unchanged ACL are both required. Temporary outsider membership is revoked
in a finally block. Failed malformed-read transactions must leave the baseline
byte-for-byte unchanged when read back as JSON.

The baseline, extension and reset stages have passed together on the current
media initializer in disposable storage (95 checks including repeated runtime
calls after bootstrap login removal). This does not complete the final proof:
native ingestion stages still require porting; schema/seed parity and the
routine privilege inventory described below still need full media integration. No initializer hash pin was changed by
this validation. The evidence records the exact tested source hash.

### Routine-permission inventory

`BaselineProof.routine_permissions` adds six checks using the independently
classified `Routine` records from candidate generation. It compares every
runtime-accessible authored function with its expected identity, owner, definer
mode, trigger status and exact ordered settings. It preserves the reset's
five-second lock setting and the baseline reader's dedicated search path.
The full native inventory, including extension routines, is retained privately
as `final-runtime-routines.json`; extension membership is validated before those
routines are excluded from authored-function assertions.

The stage also checks PUBLIC execution grants, direct table/sequence access and
the runtime database privilege matrix. Native tests change eleven independent
properties, including replacing a function name without changing the grant count.
Every mutation must fail its specific assertion, and restoration must reproduce
the exact original inventory. These tests use a small pinned PostgreSQL fixture;
full media validation must supply the frozen candidate's actual routine inventory.
They do not replace that remaining integration requirement.

## Final proof schema and seed comparison

`parity.py` compares native dumps of the privileged reference replay and the
constrained-owner final database. It reuses the exact approved ingestion/reset
transformations. Normalization removes randomized dump restrict markers, the
known public-schema comment, and the routine security headers independently
checked by the permission inventory. It retains routine bodies, other settings,
all application schemas and extension definitions. The lifecycle schema remains
outside this specific comparison, as in the original proof; baseline stages
validate it separately.

Seed comparison inventories every ordinary application table from PostgreSQL,
including empty tables. Only timestamp columns and the two approved generated
rate-limit UUIDv4 identities are normalized. Invalid, repeated or missing seed
identities fail. Other row values and ordering remain significant. Catalog
inventory travels as JSON so delimiter characters cannot split the metadata.

Each run removes previous paired comparison files before native work. Both
schema inputs and both normalized seed inputs are private evidence. Invalid
seed input cannot leave a prior successful seed pair behind. A schema mismatch
records a failed check even if the seed comparison also completes.

Native tests exercise the approved SQL transformations on both databases, then
introduce table, seed and UUID drift. They also prove that security normalization
does not erase function body text or unrelated settings. The fixture retains the
real ingestion body's nested dollar-quote envelope because pg_dump chooses its
delimiter from body content. Complete media parity still requires replay of the
reviewed frozen migration corpus; a small native fixture does not establish it.

## Final proof extension stage

`extensions.py` preserves the final initializer proof's extension boundary. It
receives the owning PostgreSQL connection, constrained owner, runtime role and
check collector. It neither discovers a database nor creates another container.
The full final-proof command is still pending; this tested stage is composed by
that proof, not offered as a separate user command.

It compares the target with a fresh stock pgcrypto/unaccent installation in the
same pinned server. The snapshot includes every extension and member, routine
body and owner, security mode, inherited execution ACL, dictionary and template.
All eleven original mutation cases must differ from stock and roll back exactly.
Runtime digest and unaccent calls prove the preserved primitive behavior. The
stock inventory must still contain forty routines and two internal callbacks.

Native tests exercise all 27 checks, detect a changed target through the verifier,
and prove rollback when a mutation query fails before its explicit rollback.
The existing owned-resource lifecycle verifies container and volume removal.
`final-stock-extensions.json` and `final-observed-extensions.json` retain private
catalog evidence; neither file by itself asserts complete final-proof success.

## Final proof reset-timeout stage

`reset_timeout.py` exercises the actual reset routine using temporary triggers
in the owned proof database. It verifies owner/runtime success, nested timeout
restoration, division-by-zero, real query cancellation and real table-lock
contention. The reset must retain its five-second lock bound and restore the
caller's outer transaction and session settings.

The stage receives an executor, monotonic clock, sleeper and fresh-token source.
Worker discovery requires the unique application identity, current database and
PostgreSQL sleep event. Cancellation rechecks that identity and the observed PID.
Workers finish before observer cleanup; an independent server timeout bounds the
cancellation case, and the lock holder has a bounded sleep plus explicit release.
Unexpected warnings and lock-holder diagnostics fail. Observer routines are
removed and `final-reset-timeout-transcript.txt` is written privately on failure
as well as success. This transcript is stage evidence, not whole-proof approval.

The native regression uses a small PostgreSQL reset fixture by default. Setting
`REVAER_TEST_INITIALIZER` to a reviewed initializer path runs the same checks on
that SQL, transactionally applied and sealed in a disposable database. A mutation
removing the reset's lock bound must fail observation. This override never points
the test at an existing database; the existing proof lifecycle owns all storage.

## Ingestion qualification port

The [ingestion evidence package](ingestion/README.md) validates strict session,
role, diagnostic and committed-identity records. Its native parser regressions
and verified routine inventory/session transaction controls are complete; isolated execution and exact approved correction comparisons are implemented; application/native qualification cases
and complete proof composition are still being ported. The thirteen-case
cold/helper-first/warm matrix is composed with phase-only evidence. The source proof explicitly leaves complete D3 scope unproven.
Individual passing cases must not be reported as complete qualification.

## Native resources and failure handling

`ProofDatabase` receives its Docker adapter, filesystem, sleeper and token source
from CLI wiring. `PostgresDocker` owns native argument construction for container
creation, version checks, readiness, SQL input, dump collection and removal.
`QueryArgs`, `DumpArgs` and `ProofContainerArgs` make those operations explicit.
SQL goes through stdin. Credentials go through a private temporary environment
file and are excluded from object representations and proof receipts.

Docker creates a fresh anonymous data volume for each proof container. Removal
uses the exact created container ID with Docker's volume-removal option, then
checks that the recorded volume is absent. This avoids host bind-mount ownership
changes. The command never prunes Docker storage or adopts a development database.
Native candidate parity verifies that this storage change preserves generated SQL.

Each run records a fresh ownership token before creation, then the returned
container and volume IDs. An uncertain create is recovered using the exact name
and ownership label. Existing receipt tokens cannot be reused. Failed startup,
SQL errors, PostgreSQL warnings and interruption all trigger owned cleanup.
Cleanup failure retains both the earlier error and the private receipt.

Candidate generation holds `target/database-rebaseline/.operation.lock` across
the complete operation. It invalidates previous generated candidate/map/success
files before native work. The successful evidence file is published only after
all proof stages and resource cleanup finish.

| Artifact under `target/database-rebaseline` | Meaning |
| --- | --- |
| `init-candidate.sql` | Exact reviewed candidate bytes. |
| `statement-boundaries.tsv` | One-based ordinal, UTF-8 byte count and semicolon line. |
| `evidence.env` | Candidate/corpus/schema hashes, native pins and successful replay results. |
| `postgres-<token>.json` | Resource ownership, completion and storage-removal evidence. |
| `migration-error.txt`, `query-error.txt` | Retained diagnostics when the corresponding stage fails. |
| `expected-schema.sql`, `observed-schema.sql` | Both inputs when schema comparison fails. |

Artifacts are private (0600; directory 0700). Validate `completed` and
`storage_removed` together when inspecting a lifecycle receipt. An unfinished run
or cleanup failure is never a successful proof.

## Architecture

- `contract.py`: immutable reviewed inputs, corpus hashing and phase checks.
- `statements.py`: narrow PostgreSQL scanner boundary and byte mapping.
- `final_sql.py`: exact approved transformations and routine classifications.
- `postgres.py`: injected disposable resource lifecycle and SQL failure handling.
- `candidate.py`: migration/replay/schema comparison and final evidence publication.
- `../external/postgres.py`: typed native tool operations.
- `../tasks/database_rebaseline.py`: static entry points and operation locking.
