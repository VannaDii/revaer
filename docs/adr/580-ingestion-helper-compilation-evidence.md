# Ingestion helper compilation evidence

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Conditional D3 in ADR 569 requires independent compilation-context evidence,
    not just equality against a rewritten reference. The existing six cold
    cases did not call helpers before ingestion.
- Decision:
  - Extend the disposable proof within the already-approved D3 investigation.
    Preserve the frozen reference, candidate digest, constrained direct runtime
    connection, caller settings and required successful second committed call.
  - Do not modify production routines, grant privileges, activate the init
    candidate or implement Proposed D4 in ADR 579.
- Consequences:
  - Six cases now execute after nine pure helpers have been called directly in
    the same backend at the caller's `plpgsql.variable_conflict=error` setting.
    Forty-two known answers cover normalization, hashing, policy matching and
    action mapping. The helper outputs are checked independently against exact
    expected values and retained in the reference/final evidence.
  - This does not prove every helper branch, mutating-helper compilation,
    trigger/native/dynamic closure, in-call settings or the complete warm-cache
    contract. Those requirements remain open; D3 is not certified.
- Follow-up:
  - Obtain the operator's D4 decision before changing temporary-table lifetime.
    Complete the remaining D3 matrix and all release acceptance gates.

## Task Record

- Motivation:
  - Advance the single-init cutover proof without bypassing the held automatic
    discovery behavior behind the existing end-to-end failure.
- Design notes:
  - A test-only SQL fixture invokes the existing helpers before the ingestion
    savepoint. The parser requires the exact one helper record between the
    caller-setting observation and ingestion result, and rejects missing,
    duplicate, unexpected or changed answers. Cold cases contain no helper
    pre-compilation. No session repair, role substitution or setting change is
    introduced.
  - Each case still uses fresh cloned databases and compares SQLSTATE, results,
    errors, caller settings and all 18 before/after table snapshots. The frozen
    function retains its GUC; only the final candidate has D3's local directive.
- Test coverage summary:
  - Focused Ruby harness: 100 assertions passed, including mutations of every
    helper's answers, malformed records and incorrect transcript ordering.
    Independent review prompted complete diagnostic framing and retention,
    helper-before-result ordering, exact inventoried signature matching, source
    line bounds and rejection of empty or foreign diagnostic SQL. The only
    diagnostic normalization is D3's independently proved one-line directive
    offset in the final function; original stderr remains unchanged on disk.
  - `just db-init-final-proof`: 134 of 135 checks passed on the pinned PostgreSQL
    16.14 image. All six added helper-first cases passed. The unchanged
    `warm-committed` case still returns `00000,42P07` in both databases rather
    than the required `00000,00000`. The command exits nonzero and reports
    `complete: false` and `passed: false`.
  - The final init digest remains
    `1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.
  - Independent review of the unchanged broker codec found no concrete contract
    violation; `just test-media-broker-codec` passed 20 tests including 25
    independent vectors. That is codec evidence, not lifecycle or containment
    validation.
  - Final independent review found no remaining issue within this bounded
    change, reparsed all 26 saved transcripts and retained the 13 matching
    reference/final comparisons, including the required warm failure.
- Observability updates:
  - Proof JSON records whether helpers compiled first and retains all helper
    answers, complete parsed error context and native error location. Raw
    SQL/stdout/stderr and table evidence remain available. Runtime telemetry and
    quality criteria are unchanged.
- Status-doc validation:
  - Reviewed the completion ledger and database approval records. No operator
    capability or production-readiness claim changes. Update the ADR index,
    book summary and generated document catalogue alongside this record.
- Risk & rollback plan:
  - Exact known answers could be mistaken for complete call-path coverage. The
    explicit remaining scope and hard failure prevent that interpretation.
    Revert this proof-only change to return to the smaller prior evidence set;
    no production state needs rollback.
- Dependency rationale:
  - No dependency added. Use the existing Ruby JSON/Digest facilities, proof
    transport and pinned disposable PostgreSQL image.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust, data, UI, devops and Sonar scoped instructions,
    ADRs 559 and 569, and the task template. No policy contradiction was removed
    by this bounded proof change. No migration, workflow, scanner setting,
    timeout, coverage threshold, API assertion or approval hold is relaxed.

## Integrated Validation

- Code checkpoint: `0eb38ecf`. `just ci` exited zero with all-feature and minimal
  tests, Clippy, policy, audits, all 18 package coverage gates and release build.
  Rust source did not change during the run. The proof review fixes were in
  place before CI's final script-coverage phase, which ran all 100 assertions;
  a separate full `just policy` rerun on the committed checkpoint also passed.
  Eight held-S2 configuration-watcher WARN events remain, so the output is not
  warning-free.
- `just ui-e2e` exited one: 46 passed, one failed, 61 not run. Profile creation
  at `tests/specs/api/media.spec.ts:130` still returns 400 rather than 201.
  Teardown still rejects missing job-phases and profile-readiness GET coverage.
  The API log retains the missing final-image compliance bundle, slow queries,
  missing library mount and synthetic tracker/session degradation diagnostics.
  No assertion or included automatic-discovery feature was removed or disabled.
- `just sonar-compile-db` and
  `just js-release-coverage js-coverage-merge sonar-verify-inputs` passed. Rust
  LCOV contains 238 source records, 101,422 line records and 94,365 covered
  lines. JavaScript LCOV contains 60 sources, 5,276 lines and 3,924 covered lines.
  Merging the final live PostgreSQL run and rejection tests through the existing
  generic converter retains 197/203 executable ingestion-proof lines covered.
  No uncovered line was removed; these are local inputs, not published metrics.
- No new source analysis or authoritative Sonar scan was completed for this
  patch. The preceding approved `sonar verify` attempt for `final_sql.rb` received
  the organization entitlement 403; its successful MCP file analysis does not
  cover these changed scripts. The full local scanner lacks `SONAR_TOKEN`.
- Documentation indexing and instruction checks pass. The initial link check
  passed 1,116 links; the book build exits zero with the existing large-index
  WARN. Final closeout-only documentation is reindexed and checked separately.
- The local code change is 380 added-plus-deleted lines against `2fb206a7`.
  This is not a verified remote PR-size or stack-completion claim.

## GitHub Metadata Follow-Through

CLI/API inspection found 104 open PRs whose heads start with `stack/media3-`;
all 104 were already assigned to VannaDii. The parent requested Copilot review
on PR 194 and the independent worker requested it on the other 103, once each.
All commands exited zero, but read-back showed no pending Copilot request or
submitted Copilot review. PR 194's direct REST requested-reviewers list was
also empty. These are submitted requests, not confirmed review fulfillment;
do not repeat them indefinitely or claim review completion.

The live API identifies the authenticated account as VannaDii while the local
CLI retains the old GioCirque label. No account switch or credential change
occurred. Current ruleset 12202805 remains active for `refs/heads/stack/**/*`
and requires all 21 recorded checks. PR 194 remains at `cfed91f8`; its workflow
does not define Supply Chain Checks. An unchanged rerun cannot emit that missing
context. No PR content, head/base/title, stack topology or criteria was changed,
and no source commit was pushed or merged. Stack repair and exact-revision
checks remain required.
