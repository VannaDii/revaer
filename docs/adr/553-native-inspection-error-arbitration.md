# Native inspection error arbitration

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural security remediation
- Context:
  - The injected native-process supervisor returns bounded secondary evidence
    when cancellation or failure leaves a process group, leader, or inherited
    output pipe in an unexpected state.
  - Media inspection runs concurrently with a database-backed job-control
    monitor. The runtime previously propagated a monitor storage error before
    examining the already-completed inspection result.
  - Independent security diff scan
    `2fb8e96a-9aba-4d34-9b99-9287eb20c181` validated that ordering as finding
    `csf_e518f81356798bf4a0709ed4`: simultaneous monitor and inspection failures
    could omit the native cleanup evidence from the selected runtime error.
- Decision:
  - Resolve the monitor task and inspection task joins independently, then
    arbitrate their inner results together.
  - Preserve the existing storage error code and category when only the
    job-control monitor fails.
  - When both monitor and inspection fail, return one typed composite error
    whose source remains the `DataError` and whose rendered detail includes the
    complete bounded `InspectError`, including native cleanup evidence.
  - Preserve the existing inspection and cancellation classification whenever
    monitoring succeeds.
- Consequences:
  - A concurrent storage failure can no longer hide evidence that native
    cleanup was incomplete.
  - Operator-facing reason-code cardinality does not change; the composite
    failure remains a storage-category failure while retaining inspection
    detail for diagnosis.
  - This repair does not broaden the process-group envelope or change the
    explicitly accepted residual limits in ADR 501.
- Follow-up:
  - Keep pairwise error-precedence tests whenever another concurrent security
    collaborator is added to media execution.
  - Verify the complete supervisor diff again after this remediation before
    integrating it into the rebuilt stack.

## Task Record

- Motivation:
  - Close the only reportable finding from the independent supervisor diff scan
    before the supervisor implementation reaches the integration branch.
- Design notes:
  - The narrow enforcement boundary is the function that owns both completed
    results. It does not change monitor polling, cancellation signaling, native
    process behavior, database behavior, or public APIs.
  - A composite variant is used only when both collaborators fail. This avoids
    string parsing and preserves the storage error as the Rust error source.
  - The inspection half is boxed only on that rare dual-failure path so the
    runtime error enum remains within the strict Clippy size bound without
    discarding typed evidence.
- Test coverage summary:
  - Focused unit coverage proves a job-control `DataError` plus an inspection
    failure retains both the probe failure and secondary cleanup evidence.
  - Positive coverage proves a job-control failure after successful inspection
    retains the existing storage-error classification.
  - Existing cancellation and non-cancellation inspection precedence tests
    remain the controls for monitoring-success behavior.
  - Owning-package, policy, documentation, full CI, UI E2E, and final security
    diff verification are required before integration.
- Observability updates:
  - The existing stable storage reason code and category remain unchanged.
  - Simultaneous inspection failure detail is now preserved in the bounded
    error text instead of being silently discarded.
- Status-doc validation:
  - Product capability status does not change. ADR 501 remains the accepted
    native-process envelope and ADR 549 remains a pending architecture proposal.
  - The ADR index, mdBook summary, and generated documentation catalog record
    this nonarchitectural remediation.
- Risk & rollback plan:
  - The main risk is changing failure precedence when both collaborators fail.
    Focused tests lock the intended composite and single-failure behavior.
  - Rollback is the single remediation commit, but it would restore validated
    cleanup-evidence loss and must not be used without an equivalent fix.
- Dependency rationale:
  - No dependency is added. The repair uses existing `Result`, `DataError`,
    `InspectError`, and `thiserror` facilities.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, accepted
    ADR 501, recorded ADR 538, proposed ADR 549, and the security scan evidence.
  - No policy drift or contradiction was found. No required check, Sonar
    criterion, error bound, native-process limit, or architectural boundary is
    relaxed.
