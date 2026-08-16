# Pre-v1 init freeze and candidate guard

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Accepted ADR 522 requires a bounded freeze-and-guard step before any
    `init.sql` assembly. Existing numbered migrations must remain the sole
    bootstrap authority during this step.
  - A complete candidate must exist before assembly begins, but committing the
    44,630-line candidate in this change would exceed the bounded delivery and
    would expose an incomplete tracked init transition.
- Decision:
  - Record implementation of ADR 522 step 1 only. No new architecture is
    introduced and no runtime, development, test, or release bootstrap path is
    switched.
  - Freeze all 167 files through
    `crates/revaer-data/migrations/0188_media_workspace_retention.sql` with
    aggregate SHA-256
    `966d3a286c7fb6f221987fd4906e9fc25cd19fe4729afd70405dee6b72a6bbac`.
    The policy gate rejects additions, edits, renames, removals, symlinks, and
    non-numbered entries in that directory.
  - Pin PostgreSQL server, `pg_dump`, and `psql` 16.14 to
    `docker.io/library/postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`
    in the existing build-input manifest. The generator rejects floating image
    references and version drift.
  - Generate only
    `target/database-rebaseline/init-candidate.sql`, which remains ignored and
    untracked. The deterministic candidate SHA-256 is
    `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`;
    it contains 1,624 complete SQL statements, 44,630 lines, and 1,593,023
    bytes.
  - Derive the candidate from a schema-only dump of a fresh migrated database,
    excluding the SQLx bookkeeping table. Normalize PostgreSQL's dump output
    through one empty-database apply/re-dump pass, then require the final
    candidate to apply to another empty database and produce a byte-identical
    normalized schema dump.
  - Reuse the existing `revaer_config.factory_reset()` contract for default
    state instead of materializing generated UUIDs or timestamps. The candidate
    sets the established schema search path only for that invocation and resets
    it afterward. No seed values or schema semantics are invented.
  - Keep `TRANSITION_PHASE=freeze`, which fails if
    `crates/revaer-data/init.sql` exists. Later assembly must explicitly change
    phase, provide an exact candidate prefix ending at a parser-proven statement
    boundary, and prove that prefix applies to an empty database.
  - Count diffs with Git rename detection disabled. Binary or uncountable
    entries fail closed. The general stack maximum is 9,999 changed lines so a
    10,000-line diff fails; init assembly has the stricter 8,500-line maximum.
- Consequences:
  - Schema work cannot enter another numbered migration after this freeze.
  - A complete, reproducible candidate is available locally for bounded
    statement-aligned assembly without making any partial file authoritative.
  - Candidate generation requires Docker and the existing exact sqlx-cli 0.8.6
    tool, but no package dependency was added.
- Follow-up:
  - Assemble exact candidate prefixes in separately reviewed stack slices only
    after this guard is integrated.
  - Do not delete migrations or select `init.sql` until ADR 522's parity and
    cutover step is complete.

## Task Record

- Motivation:
  - Establish the immutable input and reproducible full output required to
    begin bounded pre-v1 init assembly safely.
- Design notes:
  - `config/database-rebaseline.env` is the small phase and evidence contract;
    `.github/build-inputs.env` remains the single tool-version source.
  - The SQL boundary parser handles comments, quoted identifiers and strings,
    nested block comments, and dollar-quoted procedure bodies. A prefix must be
    byte-exact and end at one of its top-level semicolon boundaries.
  - Local evidence records migrated and normalized schema digests, candidate
    digest and count, tool identity, and both successful fresh applications.
- Test coverage summary:
  - Added focused mutation coverage for migration additions, modifications,
    removals, unexpected entries, phase misuse, floating tools, version drift,
    duplicate contract keys, SQL lexical edge cases, candidate digest drift,
    divergent and mid-statement prefixes, rename accounting, binary diffs,
    invalid refs, schema re-dump drift, tool drift, canonical-path confinement,
    unpinned-artifact rejection, fresh-apply failure, cleanup failure, and
    combined primary-and-cleanup failure evidence.
  - Generated the candidate twice in independent disposable PostgreSQL
    containers after correcting the schema-qualified enum dependency. Both
    runs produced the same digest and 1,624-statement count; the pinned run
    emitted passing normalization, fresh-apply, and schema re-dump evidence.
  - The focused 41-assertion rebaseline regression suite and the complete
    policy suite passed. Workflow mutation coverage proves the Feature Matrix
    candidate step cannot be removed or moved after migration-backed tests.
  - `just db-rebaseline-candidate` passed through the exact command now used by
    PR CI, reproducing the pinned candidate digest and 1,624-statement count.
    `just test`, `just check`, `just instruction-drift`, and the 939-link
    documentation check also passed on the final implementation.
- Observability updates:
  - `target/database-rebaseline/evidence.env` and
    `statement-boundaries.tsv` provide local machine-readable evidence without
    credentials, database contents, or tracked generated SQL.
- Status-doc validation:
  - Updated the ADR index and mdBook summary. Runtime and operator bootstrap
    documentation remains accurate because migrations are still authoritative.
- Risk & rollback plan:
  - Revert this guard slice to unfreeze the unchanged migration corpus. No
    database or runtime rollback is required because bootstrap did not change.
- Dependency rationale:
  - No dependency was added. The implementation uses Ruby's standard library,
    existing Docker, exact sqlx-cli 0.8.6, and the digest-pinned PostgreSQL image.
- Stale-policy check:
  - Reviewed `AGENTS.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/devops.instructions.md`, ADR 522, the root Justfile,
    all Just modules, migration/bootstrap callers, and stack check controls.
  - Drift was found in the prior rule requiring every persisted-state change to
    add a migration. The data instruction now reflects the accepted freeze and
    later direct-init model; DevOps policy now records the exact rebaseline and
    changed-line contracts. No Sonar, test, security, or required-check
    criterion was relaxed.
