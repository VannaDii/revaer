# Ingestion existing-data evidence

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Conditional D3 in ADR 569 requires independent semantic evidence. ADR 580
    covers initial cold calls and pure helper-first compilation, but not cold
    calls against data previously created by real ingestion.
- Decision:
  - Add bounded existing-data proof cases only. Do not modify the frozen
    migrations, final init, production routines, privileges, caller compilation
    settings or the existing warm-committed test. D4 remains unapproved.
- Consequences:
  - The proof now observes real fixture ingestion on one recorded backend and
    the tested call on a distinct fresh backend. These cases do not demonstrate
    warm-cache safety or fulfill the complete conditional D3 contract.
  - A further shared external-ID upsert failure is retained as a required
    failure, not accepted as parity success or repaired in production SQL.
- Follow-up:
  - Parent integration owns instruction/index/catalogue changes, stack review
    and full CI/UI/Sonar gates. Any external-ID routine/index correction needs
    an exact operator-reviewed proposal before implementation. Complete the
    remaining D3 scope and obtain a D4 decision before cutover.

## Task Record

- Motivation:
  - Exercise persisted source reuse, canonical updates, conflicts and rollback
    without using the session-repair workaround present in older Rust tests.
- Design notes:
  - Eight cases use fresh database clones and two independently recorded client
    connections each. The setup SQL creates only the existing request/indexer
    scope; actual application state comes from the real ingestion procedure.
    Both fixture and tested calls connect directly as their respective reference
    or constrained final role. No routine is replaced or precompiled by a test
    substitute, and no session is repaired within the warm test.
  - Each stage retains SQL, raw stdout/stderr, SQLSTATEs, results, exact roles,
    unchanged caller variable-conflict settings, backend PID, transaction clock
    and all 18 tables before and after. Fixture-after must equal tested-before.
    Exact variant-specific role expectations are checked before comparison;
    their differing authority is explicitly labelled, not treated as equal raw
    privilege state. The raw observed role names and capabilities are retained.
  - The new comparator collects generated public identities from all four table
    images, including before-only rows, and requires stable one-to-one numeric
    row associations. Each stage's returned identities must exist in that
    stage's committed after-image. Numeric primary and relationship IDs remain
    exact. Only the two generated public-ID columns and corresponding result
    fields normalize; arbitrary UUID-shaped text does not.
  - Captured fixture and tested transaction times retain distinct provenance.
    Only explicitly enumerated timestamp columns whose frozen defaults or
    ingestion assignments use `now()` normalize when exactly equal to a
    captured clock. Caller observation/publication times, arbitrary strings and
    other columns remain literal. The fixture clock is retained when referenced
    by tested before/after images; no timestamp-column removal occurs.
  - Strict diagnostic framing is reused unchanged, including exact frozen
    signature/body validation and only the independently verified D3 one-line
    diagnostic offset. Shared failures accumulate in the canonical checks;
    execution continues to the unchanged mandatory warm counterexample.
- Test coverage summary:
  - Preserve the original 100 harness assertions and add rejection coverage for
    stage continuity, backend reuse, missing clocks, seed-clock provenance,
    changed roles, lost seed results, before-only identities, unstable IDs,
    arbitrary clock/UUID-shaped values, relationship/table mutations, leaked
    rollback writes and shared external-ID failures. Focused harness and live
    results are recorded in the validation checkpoint below.
- Observability updates:
  - Add fixture/tested-stage evidence and explicit expected SQLSTATEs to the
    existing private proof report. No production telemetry changes.
- Status-doc validation:
  - Reviewed ADRs 569, 579 and 580 against the current finalization and goal.
    D3 stays incomplete, D4 stays Proposed, and the final init remains inert.
    This worker does not edit parent-owned indexes or status catalogues.
- Risk & rollback plan:
  - Equality against the frozen reference is weaker than a usable service: both
    can fail identically. Independent bounded outcomes therefore remain
    mandatory, and neither current shared failure can become accepted success.
    Revert these proof-only files to remove the additional evidence; no runtime
    or persisted application changes require rollback. The canonical proof
    owns and removes its unique containers and anonymous volumes.
- Dependency rationale:
  - No dependency added. Reuse Ruby's JSON facilities, existing proof transport,
    exact pinned PostgreSQL image and canonical Just recipes.
- Stale-policy check:
  - Reviewed root AGENTS, scoped data/devops instructions, the task template,
    ADRs 569/579/580, the frozen ingestion body and existing Rust ingestion
    tests. No architecture, privilege, quality criterion or included feature is
    changed. Parent owns the corresponding instruction/index/catalogue review.

## Validation Checkpoint

- Focused ingestion harness: 165 assertions pass, including all original 100.
  Exact-final-delta harness: 18 assertions pass. Candidate/freeze harness: 45
  assertions pass. The canonical freeze guard confirms the unchanged 167-file
  migration corpus and pinned digest. Whitespace checks pass.
- Replayed all 32 new raw stage transcripts against their saved JSON and all
  16 reference/final stage pairs against the final report; no mismatch found.
  This is a deterministic recheck of the retained run, not a second independent
  live implementation. Local Ruby instrumentation for the final live run and
  final rejection harness covers 159/163 executable lines in the new helper and
  200/206 in the existing ingestion proof. No coverage was uploaded.
- Canonical `just db-init-final-proof` on pinned PostgreSQL 16.14 records
  149 passing checks out of 151. All eight new reference/final comparisons
  agree, but only seven meet their independently required outcome.
- Successful cases cover newer-source/title refresh, monotonic last-seen data
  for an older observation, guid-less hash reuse, conflicting hash without
  overwriting the durable identity, tracker attribute reuse, tracker attribute
  conflicts preserving durable values, and failed-attribute rollback over
  existing data. Conflict rows, linked audit records and health events survive
  comparison; exact expected conflict values are asserted.
- `existing-external-id` requires success but returns `42P10` in both variants.
  An initial attempted IMDb fixture also failed before committing. The retained
  final case uses a successful tracker fixture, then calls real ingestion with
  IMDb data on a fresh backend. Both variants roll back the failing call. Its
  diagnostic identifies the text external-ID `ON CONFLICT` clause. The frozen
  unique index is partial (`WHERE id_value_text IS NOT NULL`), while this clause
  does not name a predicate. No index/body fix or parity exception is selected.
  Integer external-ID upserts have analogous source structure but were not
  exercised here; do not claim they were independently reproduced.
- The original warm-committed case still returns `00000,42P07`, not the required
  `00000,00000`. The canonical command exits nonzero; both reports retain
  incomplete/failed status. No backend reset, reconnect or acceptance of that
  error is introduced.
- Final init digest remains
  `1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.
- Private evidence is retained under this worker's ignored
  `target/database-rebaseline/` plus `target/ingestion-existing-*` logs and
  instrumentation. The last run's exact container and its associated anonymous
  volume have destroy events and are absent on read-back. The harness also
  asserts cleanup after fixture failure. No broad Docker cleanup occurred.
- This is bounded fixture evidence, not complete D3. Mutating helper-first
  compilation, complete branch/policy coverage, successful warm paths, in-call
  compiler settings and native/trigger/dynamic closure remain unproved. No full
  CI, UI, Sonar upload, package validation or test-media acquisition was run by
  this worker. No production-readiness or merge claim is made.

## Parent Integration

The parent reviewed and integrated `4c16d775` at checkpoint `f0a17970`, added
the matching instruction/index/catalogue updates, and independently reran the
165-assertion harness. Full integration CI passes with the retained shutdown
warnings; full UI E2E remains failed at the unchanged profile-creation boundary.
Exact results and cleanup are recorded in ADR 581. The worker's database
evidence, final logs and separate Ruby execution records were copied to the
private `revaer-stack-audit-evidence-20260910` directory before its clean
worktree was removed. Proposed D5 in ADR 583 remains unimplemented; D4 and all
other existing holds are unchanged.
