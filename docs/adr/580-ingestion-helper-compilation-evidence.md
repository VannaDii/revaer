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
  - Full repository gates and independent proof review are recorded below when
    their current executions complete; no earlier green result is substituted.
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

Pending the current repository gate executions. This record does not claim
handoff completion while `just ci`, `just ui-e2e` or required remote gates lack
successful current evidence.
