# Ingestion policy temporary table lifetime

- Status: Accepted
- Date: 2026-09-10
- Operator approval: 2026-09-11, explicit D4 approval through
  [ADR 588](588-first-release-decision-package.md#approval-resolution).
- Context: Independent D3 testing reproduces the same warm-session ingestion
  failure in the unmodified frozen reference and the final candidate.
- Decision: D4 is accepted as the exact final-init parity exception below.
  Retain the frozen counterexample; do not activate cutover before full proof.
- Consequences: The proposed change would make a transaction-scoped policy
  work table disappear at commit instead of obstructing the next transaction
  on the same pooled connection. This is a behavior correction, not proven
  equivalence to the legacy error.
- Follow-up: Implement only the accepted delta and
  independently validate its behavior alongside the still-conditional D3.

## Approval Resolution

The operator approved ADR 588 choice 1 at reviewed commit `9575c077`.
This selects D4 and the separately enumerated D5 ingestion-family changes;
it does not expand temporary-table lifetime or same-transaction semantics.
The historical evidence and recommendation wording below remain unchanged.

## Evidence

The independent proof at `19ca4e65` uses the exact pinned PostgreSQL 16.14
image and frozen candidate. Six cold scenarios have matching observed results,
data, errors and caller settings. The seventh commits a successful ingestion
and calls `public.search_result_ingest_v1` again on the same backend. Both
the frozen reference and constrained final candidate fail the second call
with `42P07: relation "tmp_policy_rules" already exists`.

Parent integration reproduced that exact outcome on final digest
`1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.
The canonical final proof records 122 passing assertions and the one failing
warm required-outcome assertion. Its report remains incomplete and failed;
the 105 D1/D2 and preceding baseline assertions pass again on this revision.

The function creates `tmp_policy_rules` with `CREATE TEMP TABLE ... AS` and
no `ON COMMIT DROP`. Its other two scratch tables already use `ON COMMIT DROP`.
The production Rust wrapper accepts a `PgPool`, performs one stored-procedure
call and has no session-reset hook. Existing size-rollup and paging tests
explicitly execute `DISCARD TEMP` between calls, which is absent from the
production path and prevents those tests from exposing this retained-table
failure. The new independent proof does not repair or reconnect the backend.

This is a shared legacy defect, not an observed regression caused only by
D3's compiler directive. Complete branch/helper closure and in-call compiler
setting evidence remain unproven even if the lifetime defect is corrected.

## D4: Exact Transaction-Scoped Policy Work Table

Recommendation, not authorization: in the final init's exact
`public.search_result_ingest_v1` statement only, replace:

```sql
CREATE TEMP TABLE tmp_policy_rules AS
```

with:

```sql
CREATE TEMP TABLE tmp_policy_rules ON COMMIT DROP AS
```

No frozen migration, input/output signature, source/policy decision, table
contents, runtime grant, timeout, dependency, or other routine body changes.
The table continues to exist for the rest of its creating transaction; its
rows are not reused in a later transaction. This is an explicit exception to
ADR 551's legacy-definition parity, additional to ADR 569 D3.

This proposal does not solve or promise multiple ingestion calls inside a
single explicit transaction. The other scratch tables already have that
transaction-scoped limitation, and the current Rust API takes a pool rather
than a transaction executor. No new public restriction, batched transaction
API, caller-owned temporary-table deletion or reuse protocol is selected here.
If required application paths need those semantics, present that distinct
contract before expanding this change.

Approval is limited to this exact statement under the pinned PostgreSQL
identity. A different temporary-table lifetime, reuse mechanism, caller
transaction contract, other routine delta or PostgreSQL identity needs renewed
review. Never add `IF NOT EXISTS`, reconnect or discard session state, remove
the repeated-call assertion, or accept `42P07` to manufacture a proof pass.

## Alternatives

- Retain legacy bytes and the failing proof. This remains the default pending
  approval; it does not establish a usable pooled ingestion path.
- Drop/recreate scratch tables on every function call or eliminate the scratch
  tables. Either changes a larger execution and ownership contract, and is not
  included in this recommendation.
- Reset or reconnect pooled sessions between results. This moves SQL lifecycle
  responsibility into callers, does not fix the procedure, and is not included.

## Required Evidence After Approval

- Keep the frozen reference untouched and retain its original counterexample.
  Explicitly assert the approved legacy-error-to-success change; do not hide it
  behind a normalized-reference comparison.
- Prove repeat calls across commits on one backend through both the versioned
  function and the production wrapper, with empty/nonempty and changed policy
  snapshots, attributes, pagination and source updates. Assert exact returned
  identities/flags and stored relationships without `DISCARD TEMP` or reconnect.
- Prove the policy work table exists within its transaction and is absent
  after commit and rollback. Exercise failed/cancelled calls followed by retry
  on that same backend, with no retained stale policy rows or privilege change.
- Correct the existing tests' caller-only temporary-table repair when the
  authoritative final init is used, retaining all domain assertions. Do not
  edit the frozen migration corpus to make legacy-path tests pass.
- Separately complete D3's independent cold/warm and reachable-helper proof.
  If an authorized control variant is used to isolate D3 from D4, retain both
  the unmodified frozen observations and the exact control delta explicitly.
- Regenerate and independently review the final digest, run full database
  conformance, `just ci`, `just ui-e2e`, strict published Sonar coverage and
  applicable GitHub checks before any cutover or merge claim.

## Task Record

### Later Isolated Evidence

Under the operator's 2026-09-11 research-only goal, the
[consolidated package](support/588-decision-details.md#database-reproduced-defect-family)
tests D4 on disposable database clones. Committed repeated calls and changed
policy snapshots succeed with the proposed lifetime, and the table is absent
after commit/rollback. Same-transaction repetition still fails, including in
the candidate; that limitation remains explicit. No committed init bytes,
guard, ordinary bootstrap path, or approval status changed. These cases do
not establish the complete conditional D3 certificate.

- Motivation: Present the exact newly reproduced obstacle without inventing
  further approval or treating a shared legacy error as acceptable behavior.
- Design notes: This record proposes only D4. The retained independent proof
  is wired into the final-init gate and fails closed; no D4 SQL is implemented.
- Test coverage summary: Worker evidence includes 33 proof-harness assertions,
  six matching cold scenarios and the failing warm case. Parent integration
  reproduces the counterexample on the D1/D2 digest as recorded above; neither
  run is a full D3 certificate. After independent review, 44 harness assertions
  and three live controls validate successful second-write persistence,
  mutation visibility, exact SQLSTATE/diagnostic handling and failure rollback.
  The proof no longer discards successful second-call writes before comparison.
  Candidate-cleanup tests pass 45 assertions,
  including exact owned-container and anonymous-volume removal.
- Observability updates: Retain raw SQL, output, errors, role/setting evidence,
  table snapshots and the explicit incomplete report. No production logs change.
- Status-doc validation: ADR 569 and the completion ledger must continue to
  describe D3 as conditional and the final init as inert; no readiness claim.
- Risk & rollback plan: A transaction-lifetime fix is narrower than function-
  local scratch-table redesign but does not provide that broader behavior.
  Keep the current runtime unchanged until approval and required proof; revert
  only the local candidate delta if validation rejects it.
- Dependency rationale: No dependency added or proposed.
- Stale-policy check: Reviewed root, data/devops/Rust instructions, ADRs 551,
  559 and 569, final SQL, Rust ingestion calls and temporary-table test cleanup.
  No accepted constraint was silently superseded; indexes are updated. The
  proof's disposable Docker cleanup now includes owned anonymous volumes in
  both candidate and final harnesses, without pruning unrelated resources.
