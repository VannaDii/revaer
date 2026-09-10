# PostgreSQL pristine catalog evidence

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record. Implements
  the accepted [551](551-packaged-database-baseline-contract.md) catalog
  contract, within [541](541-packaged-init-bootstrap-lifecycle.md). The parent
  agent coordinated this isolated slice and reserved this record; that assignment
  is not operator approval. Runtime classification and init finalization are
  outside this slice.

## Context And Scope

The packaged initializer needs mechanically reproducible pristine evidence,
not a hand-maintained provider object list. This slice adds only the fixture
extractor, exact generated catalog, canonical commands, and focused tests. It
does not change `init.sql`, the frozen migrations, runtime database access,
roles in an operator database, or any GitHub state. No old prototype supplies
approval, schema definitions, or provenance.

## Design Notes

- [The catalog](../../config/postgres-pristine-16.14.tsv) contains the exact 27
  named catalogs from ADR 551. An explicit complete column inventory is checked
  against live `pg_attribute`; missing, additional, unsupported, or unaccounted
  columns fail. This is a serialization contract, not an object allowlist.
- [The extractor](../../scripts/postgres_pristine/query.rb) resolves OID
  references to stable object identities, normalizes the current database and
  owner to `<database>` and `<database_owner>`, and preserves distinct built-in
  roles. ACLs include grantor, grantee, privilege, and grant option. Function
  signatures, bodies, execution/security settings, defaults, operator links,
  language trust, RLS, trigger/policy expressions, publication settings, and
  text-search mappings remain visible. Relation evidence also records columns,
  column ACLs, constraints, view/rule definitions, and index semantics.
- OID row keys, physical relation/tablespace locations, planner statistics,
  frozen transaction IDs, and temporary object identities are excluded as
  required by ADR 551. TOAST names derive from their parent relation instead
  of leaking numeric OIDs. Symbolic `$libdir` module references remain semantic
  identities; an absolute or otherwise unrepresentable binary-library location
  fails extraction rather than being silently dropped.
- Snapshot rows use UTF-8, LF endings, escaped JSON scalar/array values in
  tab-separated named fields, and bytewise sorting. JSON is transport/evidence
  only, never application persistence or an alternate authored SQL schema.
- Every read uses a direct connection as a newly provisioned, constrained
  non-superuser database owner. No `SET ROLE`, elevated catalog grants, or
  superuser extraction is used. Fixed `pg_catalog` search path and one
  repeatable-read, read-only transaction cover identity and all catalog rows.
- The identity query additionally verifies that this deliberately isolated
  fixture owner has no direct memberships. That is a fixture provisioning
  invariant, not an approved runtime admissibility rule. ADR 551 does not ban
  every benign owner membership; its membership restriction concerns the
  runtime role's direct or indirect membership in the schema-owner role.
  Runtime admission must implement the approved contract, not copy this
  fixture-only restriction.
- PostgreSQL restricts `pg_user_mapping` and subscription connection options.
  Emptiness is proven using the built-in unfiltered `pg_user_mappings` identity
  view and readable subscription OIDs in that same snapshot. A populated
  catalog attempts its complete projection and fails with SQLSTATE `42501`;
  masked options are never mistaken for empty data. The built-in view definition
  is itself included in relation evidence. See PostgreSQL's
  [mapping view](https://www.postgresql.org/docs/16/view-pg-user-mappings.html)
  and [subscription catalog](https://www.postgresql.org/docs/16/catalog-pg-subscription.html).
- The container has a unique name, no exposed network or port, 1 GiB shared
  memory, and an ephemeral private init credential file. Success and failure
  remove the task-owned container and anonymous volumes. SQL failures expose
  only a fixed stage, SQLSTATE, and record count, not server messages, SQL,
  URLs, or credentials. Port 5544 is not needed.

## Provenance And Commands

The version and image come from the existing [build-input manifest](../../.github/build-inputs.env):
PostgreSQL server and client `16.14`, `server_version_num=160014`, locale `C`,
encoding `UTF8`, integer datetimes and standard-conforming strings on, checksums
on, timezone UTC. This run used the pinned image's Linux arm64 variant.

```text
image: docker.io/library/postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777
image config: sha256:7e7dbab8d3b431a20793a6d99cb5a6bc84e44914309917f1bf5589a7568cdefd
snapshot SHA-256: 0ba173f3caa88da40a4391e9bd34ac88416a2f7c41f19be47043bfa54a2cbf05
snapshot bytes: 4354567
snapshot lines: 8456
catalog classes: 27
```

- `just db-pristine-catalog-generate` writes the complete generated artifact
  and detailed provenance under ignored `target/postgres-pristine/`. Promotion
  to the exact committed path is a byte-preserving copy, never manual editing.
- `just db-pristine-catalog-validate` independently provisions a fresh pinned
  fixture and requires byte equality with the committed snapshot; missing or
  different evidence fails. It never rewrites the committed file.
- `just db-pristine-catalog-test` runs all catalog and normalization mutations.
  Fixture DDL and privileged mutations exist only in disposable test setup.
- The 8,456-line snapshot fits the 9,999-line repository ceiling intact. No
  partial snapshot is committed or considered valid. Keep helper/evidence work
  and the recipe/devops update in separate commits for integration coordination;
  apply both before invoking the recipes. If a later complete snapshot alone
  exceeds the cap, retain it only as ignored evidence and obtain an explicit
  record-aligned assembly plan; never hide lines or weaken the cap.

## Validation And Remaining Work

- Generation and independent fresh-image byte comparison passed.
- Focused tests passed 131 assertions, including one mutation for each of the
  27 catalog classes, two different database/owner identities, changed OIDs,
  temporary and TOAST objects, statistics independence, wrong-owner rejection,
  column inventory drift, ACLs, search paths, leakproof/security-definer flags,
  SQL bodies/defaults, column constraints, forced RLS, and text-search mappings.
- Ruby line/branch execution records and command logs are retained under the
  ignored evidence directory. These are focused evidence, not a repository
  coverage or published Sonar claim.
- Executed Ruby line coverage was 90.00% for the command entry point, 94.12%
  for container lifecycle, 96.88% for serialization, and 100% for the column
  inventory and query builder. The authored test file reached 96.59%. No source
  or test classification, coverage instrumentation, or threshold was changed.
- `just policy`, `just instruction-drift`, documentation generation, and all
  1,053 documentation links passed. `just docs-build` completed with the
  large-search-index warning; it was not warning-free. Git diff hygiene passed.
- Full-file `analyze_code_snippet` guidance ran for all six new Ruby files in
  `MAIN` scope against `VannaDii_Revaer`, with zero reported issues. It does not
  analyze the complete repository, the generated TSV, or publish a quality gate.
- Runtime pristine classification, advisory-lock races, final baseline sealing,
  parity, packaged initialization, amd64 package validation, final CI wiring,
  and the parent stack's `just ci`, `just ui-e2e`, full Sonar and GitHub checks
  remain outside this slice. This fixture tool is not an operator initializer.

## Task Record

- Motivation: supply approved, reproducible pristine evidence independently of
  stack repair and final-init work without inventing a second schema authority.
- Test coverage summary: the focused suite above tests actual PostgreSQL 16.14
  behavior; it does not stand in for the full application or release gates.
- Observability updates: fixed local generation/count/hash and failure-stage
  evidence only; no production telemetry or credential-bearing diagnostics.
- Status-doc validation: reviewed ADRs 541, 551, and the completion ledger;
  operator/product docs remain unchanged because bootstrap is not implemented.
  The ADR index, book summary, and generated documentation catalog are updated.
- Risk and rollback plan: an extraction defect could reject a valid pristine
  database or miss drift when eventually consumed. Retain per-class mutations
  and byte comparison; review the extractor before wiring runtime classification.
  Revert these isolated files to roll back; no application database is changed.
- Dependency rationale: no new dependencies; Ruby standard library, the existing
  Docker/PostgreSQL input, and canonical `just` entry points are sufficient.
- Stale-policy check: reviewed `AGENTS.md`, the data, Rust, devops, and Sonar
  scoped instructions. Added only a pristine-specific devops rule alongside
  the recipes. No global rule, migration freeze, quality threshold, analyzer,
  required check, or architectural approval was changed or relaxed.
