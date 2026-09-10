# Runtime shutdown event classification

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - ADR 576's full CI run emitted warnings for normal bootstrap-owned aborts.
    PR 194 contains a related corrective change, absent from local integration.
- Decision:
  - Reconcile the runtime correction only. Log a cancelled join at info when
    this stop operation requested cancellation; retain warning severity for
    every other join failure and for graceful-shutdown deadline expiry.
  - Reuse the existing stop helper for filesystem and configuration tasks, and
    observe the configuration task's joined result after aborting it.
  - Do not port PR 194's separate UI assertions or historical approval status.
- Consequences:
  - Successful requested shutdown is still observable without a false warning.
    Unexpected cancellation, panics and deadlines remain visible failures.
  - No cancellation deadline, task startup, media readiness, broker policy,
    native containment, persisted state, or included feature changes.
- Follow-up:
  - Complete focused event-level regressions, independent review, full CI/UI,
    Sonar checks, and cleanup. Keep ADR 569 and E1 holds unchanged.

## Task Record

- Motivation: Fix the demonstrated mismatch between an explicit abort request
  and warning-level join classification while reconciling the divergent stack.
- Design notes: Adapt only PR 194's runtime intent from
  `cfed91f82a07a909ac1b96d9129e9eabeaddbf1c`. Share the existing stop helper
  instead of duplicating cancellation handling. The explicit request flag
  records that this shutdown operation requested an abort; it does not prove
  exclusive causality if another caller races with it. Already-finished and
  graceful unrequested cancellation must not receive this classification.
- Test coverage summary: Focused event-level tests and exact-revision integrated
  results are pending and must be attached before handoff. Existing full CI/UI
  criteria are unchanged; a focused pass cannot establish feature completion.
- Observability updates: Requested cancellation emits one bounded info event.
  Other join errors retain their warning and error field. Grace expiry retains
  its existing warning in addition to observing the subsequent join result.
- Status-doc validation: Update the ADR indexes and generated catalog, with
  integrated evidence recorded here. No release-readiness claim is added.
- Risk & rollback plan: Misclassification could conceal a real failure. Test
  both request states with real cancelled and panicked joins and actual stop
  helpers, including grace expiry and joined cleanup. Revert this bounded
  correction if contradicted; do not weaken tests to retain it.
- Dependency rationale: No new dependency. Existing Tokio, tracing, and test
  support provide task and event evidence.
- Stale-policy check: Reviewed root, Rust and devops instructions, current
  bootstrap, PR 194's runtime/UI diff, and ADR 576 evidence. The new focused
  recipe is documented in the scoped instructions and supplements full gates.
  No Sonar, fixture, coverage, review, architecture-approval or GitHub criterion
  is relaxed. Historical ADR 454's status is not adopted as approval evidence.
