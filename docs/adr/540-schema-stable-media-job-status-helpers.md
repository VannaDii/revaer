# Schema-stable media job status helpers

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The accepted ADR 522 rebaseline applies a schema-only PostgreSQL dump with
    the dump's intentionally empty search path before selecting `init.sql`.
  - Six immutable media-job status helpers returned the public enum but cast
    literals through the unqualified `media_job_status` name in their SQL
    bodies. PostgreSQL could create the dump with function-body checks disabled,
    then failed while validating a dependent constraint because the empty
    search path could not resolve that body reference.
- Decision:
  - Schema-qualify both the return type and literal cast in all six status
    helpers as `public.media_job_status`.
  - Preserve the function names, volatility, arguments, return values, and all
    caller contracts. No runtime or persistence behavior changes.
- Consequences:
  - The final schema can be restored and validated without ambient search-path
    assumptions.
  - The frozen migration-corpus digest must be recalculated before ADR 522's
    freeze guard is committed.
- Follow-up:
  - Regenerate the complete rebaseline candidate and require fresh-database
    apply plus byte-identical normalized schema re-dump evidence.

## Task Record

- Motivation:
  - Remove a deterministic fresh-bootstrap failure discovered by applying the
    complete current schema candidate.
- Design notes:
  - Qualification is deliberately limited to the enum type references inside
    the six zero-argument helper functions. It does not add a function-level
    search path or change privilege behavior.
- Test coverage summary:
  - The final validation must rerun the full migration-backed database tests and
    the ADR 522 candidate generation, fresh apply, and normalized re-dump gates.
- Observability updates:
  - None. This is bootstrap schema correctness with no runtime signal change.
- Status-doc validation:
  - Added this record to the ADR index and mdBook summary. User-facing and
    operator-facing runtime behavior is unchanged.
- Risk & rollback plan:
  - A regression would affect clean schema creation only. Reverting this commit
    restores the prior helper definitions, but would also restore the proven
    empty-search-path bootstrap failure.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md`,
    `.github/instructions/revaer-data.instructions.md`, and accepted ADR 522.
    No policy drift or stale reference was found, and no quality criterion was
    relaxed.
