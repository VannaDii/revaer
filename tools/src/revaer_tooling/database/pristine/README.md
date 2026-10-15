# Pristine PostgreSQL catalog proofs

This package preserves the catalog reference used by Revaer's database
initializer. It reads a fresh PostgreSQL 18.6 database as its constrained owner
and compares the complete result with the reviewed snapshot. It requires Docker
and the image pinned in `.github/build-inputs.env`.

## Commands

| Command | Result |
| --- | --- |
| `rv db-pristine-catalog-generate` | Generate the complete snapshot and provenance in `target/postgres-pristine/`. |
| `rv db-pristine-catalog-validate` | Generate fresh evidence and require an exact byte match with `config/postgres-pristine-18.6.tsv`. |
| `rv db-pristine-catalog-test` | Exercise all 27 catalog classes, normalization and privilege boundaries against the pinned server. |

Generation leaves the committed reference untouched. A differing reference fails
validation and retains the newly observed snapshot for review. A result above
9,999 lines is reported; the generator never truncates a catalog to fit a diff.
These commands need only the PostgreSQL pin, so they also work in the tooling
foundation before the application database transition is rebased onto it.

## How the proof works

1. Verify the server, `psql` and `pg_dump` versions in the digest-pinned image.
2. Start a private, network-isolated container with checksums, logical WAL, UTC
   and a fresh anonymous data volume. Record resource ownership before startup.
3. Create a `template0` database with UTF-8 encoding and C collation. Its direct
   login owner has no superuser, role creation, database creation, replication,
   row-security bypass or membership privileges.
4. Compare every catalog column with `spec.json`. Read identity and catalog rows
   in one repeatable-read, read-only transaction with fixed timeouts/search path.
5. Resolve object references, normalize physical identities, and serialize every
   row into sorted UTF-8 evidence. Reject missing, duplicate or unresolved records.
6. Remove the owned container and volume, verify removal, then publish evidence.

`pg_subscription` and `pg_user_mapping` contain credential-bearing fields that
the owner cannot read. Empty results require a separate proof of emptiness using
readable identities. A populated catalog attempts the complete projection and
must fail with PostgreSQL's permission error; it is never silently omitted.

## Package structure

| File | Responsibility |
| --- | --- |
| `spec.json`, `spec.py` | Reviewed columns, reference targets, expression deparsers and explicit exclusions; immutable validated input. |
| `query.py` | PostgreSQL catalog queries, ACL/reference resolution, TOAST identities and definition projections. |
| `snapshot.py` | Complete inventory, transaction, strict JSON transport, row validation and stable serialization. |
| `workspace.py` | Selected-checkout inputs, constrained fixture provisioning and provenance publication. |
| `../../tasks/pristine.py` | Static command methods, operation lock and generate/validate orchestration. |

The seven existing excluded `pg_class` fields are physical allocation,
statistics or transaction-maintenance values. Their names and rationale remain
in `spec.json` and the generated provenance. Object OIDs are resolved to names;
TOAST relation names derive from their parent relation. ACLs, function bodies,
security properties, view/index/constraint definitions and text-search mappings
remain part of the comparison.

## Evidence and failures

The generated TSV and `provenance.json` have mode `0600`. Provenance records the
image identity, server release, complete typed column inventory, exclusions,
reader and transaction contract, generation time, and measured hash/size/count.
The shared [proof lifecycle](../README.md) retains private ownership and diagnostic
records after failures. There is no connection option for an operator database.

Tests mutate each catalog class and compare fresh reads. Additional cases cover
different owner/database names, changed statistics, temporary objects,
reallocated object/TOAST OIDs, function security and grants, row security,
column changes, incomplete inventories, duplicate JSON keys and nonfinite values.
The deliberately disconnected subscription fixture asserts its exact expected
warning. Normal SQL proof operations continue to fail on warnings.

The initial native Python generation matched the existing Ruby snapshot exactly:
8,456 lines, 4,354,567 bytes, SHA-256
`0ba173f3caa88da40a4391e9bd34ac88416a2f7c41f19be47043bfa54a2cbf05`.
This proves the pristine reference port; the complete application initializer
proof is a separate command and acceptance requirement.
