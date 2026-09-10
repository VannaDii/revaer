# Missing init candidate diagnostic

- Status: Recorded
- Date: 2026-09-09
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Running `just db-init-prefix-check` in a clean worktree before generating
    candidate evidence raised `NoMethodError`: `Errno::ENOENT` does not provide
    the `path` method used by the error handler.
- Decision:
  - Report the existing candidate-missing failure using the method's known path,
    rendered relative to the repository. Preserve failure status and all digest,
    statement-count, frozen-corpus, and prefix checks.
- Consequences:
  - Missing evidence now produces a useful operational diagnostic, not an
    exception while reporting the original failure. It never counts as a pass.
- Follow-up:
  - Generate candidate evidence through `just db-rebaseline-candidate` before
    checking the assembly prefix. This is not the final database cutover.

## Task Record

- Motivation:
  - Repair a reproducible failure path encountered during fresh init validation.
- Design notes:
  - Use the existing `relative` helper and known method argument. Do not inspect
    exception internals, invent a fallback candidate, or create directories in
    the validation path.
- Test coverage summary:
  - Added exact-message regressions for the missing default candidate and a
    missing explicitly supplied candidate, both without candidate directories.
  - `just policy` passes, including 43 rebaseline assertions; instruction drift
    and diff checks pass. Full CI and UI results remain separate release gates.
  - `just db-rebaseline-candidate` passes against pinned PostgreSQL 16.14:
    normalization apply, fresh candidate apply, and normalized schema re-dump
    comparison all pass. The unchanged candidate has 1,624 statements and
    SHA-256 `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`.
  - `just db-init-prefix-check` then passes for all 1,593,023 candidate bytes.
    No runtime cutover or final grants are established by those results.
- Observability updates:
  - The CLI reports `candidate is missing` with the repository-relative path;
    the existing nonzero exit behavior is retained.
- Status-doc validation:
  - No runtime or database authority changes. The existing init candidate remains
    inert assembly evidence; the operator-facing release state is unchanged.
- Risk & rollback plan:
  - The change affects only missing-file error translation. Existing valid,
    corrupt, divergent, and partial-candidate tests remain mandatory.
  - A regression can revert this narrow change without touching SQL or stored
    data; missing evidence must still fail closed.
- Dependency rationale:
  - No dependencies are introduced or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, devops/data scoped instructions, the ADR template,
    the rebaseline helper, and existing policy-suite tests. The devops guidance
    now records the failure-path obligation. No quality criterion or database
    transition boundary is relaxed.
