# Pre-v1 single init-script transition

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- The product has not shipped a stable v1, so preserving a long incremental
  migration history adds review and bootstrap complexity without a supported
  installed-database upgrade obligation.
- The current migration corpus is much larger than the 10,000 changed-line limit
  for one pull request. Adding a complete rebaseline and deleting the corpus in
  one change would violate the stack review contract even if Git classified some
  files as renames.
- Every intermediate pull request must remain reviewable, pass all required
  checks, and avoid making an incomplete init script the runtime bootstrap.

## Options

1. **Replace all migrations in one pull request.** This is direct but exceeds the
   review-size limit and prevents focused stacked review.
2. **Assemble one final-state init script and retire migrations in bounded stack
   slices.** Keep migrations authoritative during statement-aligned assembly,
   prove parity, switch bootstrap once, then delete inert history in bounded
   slices.
3. **Keep migrations until v1.** This avoids transition work but contradicts the
   selected pre-release operating model and preserves redundant historical DDL.

## Recommendation

- Adopt option 2.
- The final authority is exactly `crates/revaer-data/init.sql`. It contains the
  final extension setup, schemas, tables, constraints, indexes, routines,
  triggers, seed rows, privileges, and comments needed to create a fresh v1
  candidate database. It is a final-state rebaseline, not a concatenation of
  historical create/alter/drop transitions.
- No production or development database is upgraded in place during this
  transition. Because no stable v1 has shipped, existing local and CI databases
  are recreated from scratch. Any environment requiring retained data remains
  blocked until an operator approves a separate export/import plan.

### Deterministic Rebaseline

- Build a disposable database by applying the frozen migration corpus through
  the existing canonical recipe. Export a normalized final schema with the
  repository-pinned PostgreSQL tools, then add reviewed extension setup, seed
  data, ownership, and grants in deterministic sections.
- Strip owner-specific, database-name, session-noise, and environment-specific
  output. Preserve security-definer search paths, constraint validation,
  privileges, comments used by tooling, and every normalized seed identity.
- Produce one local full candidate and a statement-boundary map. The candidate
  must parse and apply from an empty database before any bytes are proposed for
  the stack. No generated candidate or database dump is pushed outside the
  reviewed `init.sql` slices.
- Record a SHA-256 digest and statement count for the complete candidate in the
  transition task record when implemented. Every assembly slice proves that the
  tracked file is the exact prefix of that reviewed candidate and ends after a
  complete SQL statement.

### Bounded Stack Sequence

1. **Freeze and guard.** Reject new migration files, pin the rebaseline tool
   versions, add fresh-database and changed-line checks, and record the complete
   candidate digest. Existing migrations remain the only bootstrap source.
2. **Assemble `init.sql`.** Append statement-aligned prefixes across as many
   stacked pull requests as required. Each pull request changes at most 8,500
   text lines against its immediate parent, leaving review budget for ADR,
   catalogue, and test updates. The partial prefix is marked assembly-only and
   is never selected by runtime or ordinary tests.
3. **Prove parity and cut over.** After the final statement is present, create
   two empty databases: one from frozen migrations and one from `init.sql`.
   Compare normalized catalog objects, routine definitions, constraints,
   indexes, triggers, extension versions, seed rows, and grants. Run the complete
   database and application suites against the init-created database, then make
   it the sole bootstrap source in one bounded pull request.
4. **Retire inert history.** Delete migration files in statement/file groups of
   at most 8,500 changed lines per stacked pull request. Bootstrap continues to
   use the unchanged init script, and every required check still runs.
5. **Remove transition machinery.** Delete the migration runner, embedded
   migration metadata, temporary parity tooling, and assembly marker. Add a
   guard that requires `init.sql`, rejects a migrations directory, and verifies
   fresh bootstrap through the canonical `just` recipes.
- Each pull request is based on the immediately preceding stack branch. The
  changed-line gate uses GitHub-equivalent additions plus deletions from that
  base and fails at 10,000; binary or uncountable entries fail closed. Rename
  detection is never used to evade the limit.
- Required workflows are not skipped during assembly or deletion. Before
  cutover, they exercise migrations plus the explicit init-prefix check. At and
  after cutover, every database-backed check starts from `init.sql`.

### Final Operating Contract

- Before the first stable v1 release, every accepted schema change edits
  `init.sql` directly and proves fresh-database bootstrap. No numbered migration
  is added or restored.
- The v1 release records the exact init digest as the baseline. Only after that
  release may a separately approved forward-migration system begin with changes
  newer than the baseline; it never reconstructs the deleted pre-v1 history.
- CI, local tests, development startup, containers, and release validation use
  the same init entry point through `just`. No workflow carries a second SQL
  bootstrap implementation.

## Consequences

- The final repository has one reviewable fresh-install database definition and
  no pre-release migration archaeology in the runtime path.
- The transition requires many mechanical stack slices, but each stays below the
  review limit and remains independently green.
- Temporary duplication exists after init cutover while inert migration files
  are deleted. One explicit guard ensures only init is executable during that
  interval.
- Existing unpublished databases are disposable under this decision. Retaining
  one would require a separately approved data transition.

## Implementation Boundary

- This accepted ADR authorizes only the final `init.sql` authority, deterministic
  rebaseline, bounded assembly/cutover/deletion sequence, parity proof, fresh-
  database policy, and final guards described above.
- This ADR does not authorize skipping checks, reducing Sonar scope, hiding
  SQL from analysis, exceeding 10,000 changed lines, preserving an alternate
  bootstrap, deleting user data, or creating a post-v1 migration policy.
- The exact schema content remains governed by separately accepted media ADRs.
  Accepted ADRs 507-521 and 523-525 authorize only SQL within their recorded
  scopes; unresolved exact values in ADRs 515 and 516 authorize no concrete
  defaults.
- SQL, script, workflow, dependency, and bootstrap changes must remain limited to
  the accepted transition and media contracts.

## Validation

- Decision validation to date is documentation-only.
- During implementation, every assembly prefix must pass SQL parsing, empty-database
  prefix application, candidate-prefix digest, instruction drift, docs links,
  and changed-line enforcement.
- Cutover must compare normalized catalogs and seed state in both directions and
  run all stored-procedure, schema, API, coverage, and integration suites against
  the init-created database.
- Every deletion slice must prove init bytes and digest are unchanged and the
  removed migrations are not referenced by source, tests, packages, or docs.
- The final slice must prove no migration file or runner remains and that
  `just ci` and `just ui-e2e` pass from a clean fresh database.

## Follow-up

- Generate and review the full local candidate before opening the first assembly
  slice; do not use incremental design decisions while appending bytes.
- Place schema work for accepted ADRs before candidate freeze or defer it to a
  direct pre-v1 init edit after the transition, never into a new migration.

## Task Record

- Motivation:
  - Reconcile the pre-v1 single-init requirement with bounded stacked review.
- Design notes:
  - Assembly and deletion are deliberately separate so no incomplete init script
    becomes authoritative and no pull request exceeds the size limit.
- Test coverage summary:
  - The ADR-only change added no SQL, bootstrap, workflow, or database test.
- Observability updates:
  - No runtime telemetry is required. CI evidence later records bounded counts,
    candidate digest, parity result, and bootstrap source without database data.
- Status-doc validation:
  - Reviewed the current migration corpus, `MEDIA_TRANSCODING.md`, accepted ADRs
    446-451, 484, 500, 501, and 507-521.
- Risk & rollback plan:
  - Before cutover, revert the affected leaf slices. After cutover, rollback
    means rebuilding an unpublished database from the prior stack commit; no
    in-place downgrade is claimed.
- Dependency rationale:
  - No new dependency is required. Existing PostgreSQL tools, SHA-256 support, Git
    diff accounting, and canonical `just` recipes are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-data.instructions.md`,
    and `.github/instructions/devops.instructions.md`; no quality or review rule
    is relaxed.
