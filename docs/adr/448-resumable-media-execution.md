# Resumable media execution

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The v1 contract calls for deterministic DAG execution, retry support,
    resumability, intermediate reuse, failure isolation, and policy-controlled
    maximum parallel jobs.
  - Current operation rows record the selected plan but have no queued, running,
    completed, failed, or invalidated state and no checkpoint artifact identity.
  - The runtime claims one job per tick and executes all pre-replacement steps as
    one in-memory sequence. Retry creates a new whole-job attempt.
  - Replacement preparation, commit, final verification, rollback, and recovery
    already form a stricter transactional boundary than ordinary transform
    steps and must remain serialized per source aggregate.
- Decision:
  - Select Option B, durable per-step checkpoints inside one job attempt, for v1
    as approved by the operator.
  - **Option A: whole-attempt replay.** Keep operations as plan evidence and
    restart every failed attempt from source. Add bounded multi-job scheduling
    only. This is simpler but does not satisfy intermediate reuse or true step
    resumability and can repeat expensive transcodes.
  - **Option B: durable per-step checkpoints inside one job attempt.** Persist
    operation lifecycle, input/output fingerprints, dependency completion,
    invalidation reason, and bounded artifact references. Resume only verified
    deterministic steps; serialize the replacement transaction. This satisfies
    v1 on one node without introducing a distributed queue.
  - **Option C: independently leased operation queue.** Model every DAG node as
    a claimable unit with separate leases and worker scheduling. This maximizes
    parallelism and future distribution but substantially expands coordination,
    cancellation, artifact ownership, and recovery complexity.
  - Keep interfaces compatible with a later Option C transition without
    implementing a distributed operation queue in v1.
- Consequences:
  - Option B requires an immutable plan plus mutable, generation-fenced operation
    execution records and verified checkpoint artifacts.
  - Concurrency must be bounded globally and by the claimed job's snapshotted
    policy, with one active replacement transaction per source aggregate.
  - A checkpoint is reusable only when source identity, desired graph, capability
    snapshot, command contract, and output fingerprint still match.
  - Cancellation, maintenance windows, thermal/power pauses, shutdown, and stale
    leases need explicit resumable transitions rather than process-local state.
- Follow-up:
  - Specify the operation state machine, lease fencing,
    checkpoint validation, scheduler fairness, and replacement serialization
    before implementation.
  - Coordinate with ADR 446 so runtime limits are available from immutable job
    policy snapshots.

## Task Record

- Motivation:
  - Surface the execution-state decision required to close the v1 resumability
    and concurrency gap without weakening replacement safety.
- Design notes:
  - This decision preserves current verification and rollback as mandatory
    boundaries under every option.
  - It does not authorize distributed execution or new dependencies.
- Test coverage summary:
  - Evidence gathered from the worker loop, claim/retry procedures, operation
    schema and readers, execution sequence, replacement recovery, and the v1
    execution contract.
  - No behavior has changed yet.
- Observability updates:
  - A future implementation must report operation transitions, resume/replay
    reasons, checkpoint reuse, lease recovery, queue depth, active concurrency,
    and replacement serialization without duplicating error logs.
- Status-doc validation:
  - Checked the pipeline, runtime-policy, and execution sections of
    `MEDIA_TRANSCODING.md`.
- Risk & rollback plan:
  - No runtime risk before approval.
  - After implementation, rollback must preserve readable attempt evidence and
    invalidate, not silently reuse, checkpoints the older runtime cannot prove.
- Dependency rationale:
  - No dependency proposed. Existing PostgreSQL procedures and Tokio primitives
    are sufficient for the proposed v1 model.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; the proposal keeps
    runtime state stored-procedure-backed and collaborators injected.
