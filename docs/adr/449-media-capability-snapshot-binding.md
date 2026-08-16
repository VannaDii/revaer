# Media capability snapshot binding

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Deterministic planning requires the same input, target, policy, and runtime
    capabilities to produce the same plan.
  - Capability discovery persists immutable completed runs, but jobs and attempts
    do not record which run governs planning or execution.
  - Production currently loads the latest completed capability run when a worker
    begins processing. A retry can therefore use different capability evidence
    without an explicit transition or invalidation record.
  - Binding time determines queue behavior across upgrades, retry determinism,
    and whether stale jobs may execute against changed tools.
- Decision:
  - Select Option B, binding on the first successful claim, as approved by the
    operator.
  - **Option A: bind when enqueued.** Snapshot the latest completed capability-run
    id into the job configuration. This gives queue-time reproducibility, but a
    queued job can become unexecutable after a host upgrade and jobs cannot use
    capabilities discovered between enqueue and first execution.
  - **Option B: bind on first successful claim.** Atomically bind one completed
    capability-run id to the first attempt before planning; every retry and resume
    reuses that binding unless an operator explicitly requests re-planning. This
    reflects execution-time capability evidence while preserving deterministic
    retries.
  - **Option C: bind independently on every attempt.** Every attempt uses the
    latest completed run. This adapts automatically to upgrades but changes plans
    across retries and weakens immutable audit semantics.
  - Require a fail-closed executable
    identity check before reuse and an explicit operator re-plan action when the
    bound tools are no longer available.
- Consequences:
  - Options A or B make capability provenance part of the durable plan and audit
    evidence.
  - Option B requires a generation-fenced stored procedure that binds exactly
    once and returns the complete immutable capability run.
  - Re-planning under different capabilities must create explicit new evidence;
    it may not silently mutate or overwrite the previous selected plan.
- Follow-up:
  - Define binding, executable-identity validation, invalidation,
    and operator re-plan contracts before changing worker behavior.
  - Coordinate checkpoint reuse rules with ADR 448.

## Task Record

- Motivation:
  - Close the unrecorded capability drift boundary before retries and resumable
    execution are made production-reachable.
- Design notes:
  - This ADR chooses when immutable evidence is bound, not how capabilities are
    discovered or which codecs are supported.
  - Runtime database access remains stored-procedure-only under every option.
- Test coverage summary:
  - Evidence gathered from the job snapshot schema, capability-run procedures,
    latest-capability reader, and production worker preflight path.
  - No implementation or behavioral test has been added yet.
- Observability updates:
  - A future implementation must expose the bound capability-run id, tool
    identity mismatch, re-plan request, and checkpoint invalidation reason.
- Status-doc validation:
  - Rechecked the determinism and capability requirements in
    `MEDIA_TRANSCODING.md`; current latest-at-execution behavior is not recorded
    as an intentional contract.
- Risk & rollback plan:
  - No runtime risk before approval.
  - After implementation, rollback must retain capability provenance and must not
    silently return to latest-run selection for existing jobs.
- Dependency rationale:
  - No dependency proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; no scoped rule selects a
    capability binding time.
