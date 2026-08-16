# Fenced resumable worker ownership and recovery

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- The replayed media worker claims one job on a fixed poll, executes the complete
  attempt in one process-local sequence, and has no durable operation lifecycle
  or reusable checkpoint identity.
- Cooperative service shutdown currently converts a newly claimed or active job
  into durable terminal cancellation. That conflates an operator request to
  cancel work with a process lifecycle event that should remain resumable.
- Stale-worker recovery terminalizes running or verifying attempts after a fixed
  heartbeat age. It cannot distinguish an interrupted deterministic operation
  from unrecoverable work and has no cross-process recovery leader.
- Approved ADR 448 requires durable per-step checkpoints inside one attempt,
  globally and policy-bounded concurrency, explicit resumable transitions, and
  one active replacement transaction per source aggregate.
- Approved ADR 500 requires attempt-and-generation-scoped workspaces and
  reconciliation, complete terminal-outbox paging, and a cooperative aggregate
  lease held through replacement finalization or rollback. It intentionally does
  not define the lease protocol or timing.
- The unapproved ADR 503 proposal describes one possible PostgreSQL lease and
  recovery-leadership protocol, but it does not define the complete checkpoint,
  shutdown, scheduler, and stale-recovery state machine. Its exact lease timing
  has no implementation or fault-test evidence.
- Runtime database access must remain stored-procedure-only, collaborators must
  remain injected, and v1 must not introduce an independently leased operation
  queue or a new coordination dependency.

## Options

1. **Retain singleton whole-attempt replay.** Keep one serial worker, terminalize
   stale or shutdown-interrupted attempts, and restart expensive work in a new
   attempt. This is small but does not satisfy approved resumability or safe
   multi-process source ownership.
2. **Use database-fenced resumable attempts with aggregate leases and root
   recovery leadership.** Persist attempt and operation transitions, fence every
   owner write, coordinate claim capacity and source leases through PostgreSQL,
   and resume only validated checkpoints.
3. **Create an independently leased operation queue or external coordinator.**
   Schedule every DAG node independently. This supports wider distribution but
   exceeds ADR 448's v1 boundary and adds materially more cancellation,
   artifact-ownership, and recovery state.

## Recommendation

- Adopt option 2.
- This accepted ADR supersedes the unapproved ADR 503 proposal. It supersedes only a
  pending proposal, not an accepted decision or implemented contract.
- Persist attempt states sufficient to distinguish `queued`, `running`,
  `pausing`, `paused`, `recovering`, `verifying`, `finalizing`, and terminal
  `completed`, `failed`, and `cancelled` outcomes.
- Persist operation states `pending`, `running`, `completed`, `failed`, and
  `invalidated`, together with generation-fenced input fingerprints, output
  fingerprints, dependency completion, invalidation reason, and bounded managed
  checkpoint references.
- Service shutdown transitions active work through `pausing` to `paused` in the
  same attempt. Only completed and validated operation artifacts are reusable;
  interrupted native-process output is invalidated before the attempt becomes
  resumable. Operator cancellation remains a distinct terminal transition and
  wins if it is pending during recovery.
- A resumed attempt receives a new claim generation without consuming a retry.
  A new attempt number is created only by explicit retry after a terminal failure
  or by the accepted ADR 520 `operator_replan` transition; resuming an attempt
  only advances claim generation. Every write remains fenced by job, attempt, and
  current claim ownership.
- The claim boundary enforces one instance-global capacity limit and the claimed
  job's immutable snapshotted policy limit. Eligible jobs are selected in stable
  oldest-first order while leased or otherwise ineligible aggregates are skipped
  without consuming an attempt or retry.
- Potentially destructive work must atomically acquire the global slot, policy
  slot, attempt ownership, and exclusive aggregate lease before activation. The
  aggregate key derives from the approved managed-root identity and normalized
  root-relative aggregate path. Dry runs do not acquire a destructive lease.
  A verified no-op may skip the lease only if its no-op state is durably proven
  before attempt activation; otherwise it is treated conservatively as
  potentially destructive.
- Attempt ownership and aggregate ownership are separate fenced leases. Both use
  database server time, monotonically increasing generations, and stored
  procedures. Failure to renew before expiry stops new operation starts and
  prevents the stale owner from checkpointing, finalizing, or releasing another
  owner's lease.
- A per-root recovery leader blocks new destructive claims and workspace
  retention for that root. It takes over only expired ownership, loads every
  attempt and replacement transaction for the root, pages the complete terminal
  outbox by stable keyset, validates checkpoints and manifests, and releases the
  root only after reconciliation is durably acknowledged.
- Stale recovery does not fail work merely because its former heartbeat expired.
  It moves the attempt through `recovering`, validates completed checkpoints,
  invalidates interrupted operations and their dependants, and returns the same
  attempt to `paused` or claimable work. It marks the attempt failed only when a
  persisted invariant, source identity, capability binding, or checkpoint cannot
  be recovered safely.
- The aggregate lease remains held after terminal database commit until the
  replacement filesystem transaction has finalized or rolled back, finalization
  is durably acknowledged, and the corresponding terminal event is reconciled.

### Supported Operational Values

- Preserve the replay's one-second claim scan as the initial scheduler cadence.
- Preserve an initial instance-global concurrency limit of one. Existing policy
  snapshots also seed `max_concurrency` to one; the schema's larger validation
  ceiling is not evidence that higher production concurrency is safe.
- Preserve the 250-millisecond control poll only as active cancellation and
  shutdown response latency. It is not the aggregate-lease refresh interval.
- Page at most 1,024 terminal-outbox rows per query, but continue by stable
  keyset until the complete unpublished set has been classified.
- Preserve ADR 500's approved 1 GiB source-filesystem staging reserve and ADR
  501's approved native-process deadline, output, process-group, and five-second
  escalation values.

### Explicitly Undecided Timings

- Aggregate-lease refresh and expiry timings remain undecided. In particular,
  this ADR does not adopt ADR 503's proposed 10-second refresh or 60-second
  expiry values.
- Attempt-heartbeat persistence cadence and lease-based stale-recovery threshold
  remain undecided. The replay's one-hour terminal stale threshold is evidence of
  current behavior, not a recommended resumable-recovery value.
- No additional lease-contention delay, scheduler fairness window, retry jitter,
  or recovery-leader backoff is selected without measured database, failover,
  filesystem, and shutdown evidence.

## Consequences

- Service replacement and planned shutdown no longer fabricate operator
  cancellation or force a whole-attempt retry.
- Completed deterministic work can be reused only when all approved source,
  graph, capability, command, and output identities still match.
- Concurrent service processes cannot mutate the same source aggregate or let a
  stale finalizer complete after ownership changes.
- Recovery must complete per root before destructive claims or retention can
  start, increasing startup coordination while making filesystem decisions
  deterministic.
- The operation, attempt, claim, lease, replacement, outbox, and checkpoint
  schemas and their stored procedures must change as one contract.
- Metrics and logs require bounded transition and reason dimensions; job, path,
  aggregate, attempt, claim, and lease identifiers must not become metric labels.
- Deployments with writers that do not honor the cooperative root contract remain
  ineligible for destructive processing.

## Implementation Boundary

- This accepted ADR authorizes only the attempt and operation state machines,
  claim and capacity ordering, PostgreSQL-backed aggregate ownership, per-root
  recovery leadership, resumable shutdown distinction, checkpoint validation,
  stale recovery, and complete replacement/outbox reconciliation described here.
- This ADR supersedes the pending ADR 503 protocol proposal. ADR 503 must remain
  superseded and must not be implemented independently.
- This ADR does not authorize any exact lease refresh, lease expiry,
  heartbeat, stale-recovery, contention, or recovery-backoff timing listed as
  undecided above.
- Approved ADRs 442, 444, 446, 447, 448, 449, 483, 500, and 501 remain binding.
  This ADR fills their worker-ownership and recovery protocol gap without
  reopening their planning, dry-run, snapshot, root, process, or crate decisions.
- Implementation must remain within ADR 483's existing crate boundaries, use
  injected collaborators, and route every runtime database operation through a
  stored procedure.
- Before v1, accepted persistence changes must update
  `crates/revaer-data/init.sql`. Historical migrations 0166, 0167, and 0186 are
  provenance only and must not be restored.
- This ADR does not authorize a distributed operation queue, an external
  coordinator, filesystem-specific mandatory locks or snapshots, compatibility
  with noncooperating writers, background legacy-transaction migration, higher
  concurrency, or weaker fencing and verification.
- Schema, runtime, API, health, deployment, and generated-contract changes must
  remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only; this record changes no worker,
  database, filesystem, process, API, or deployment behavior.

| Scenario | Required implementation result |
| --- | --- |
| Two processes select the same aggregate | Exactly one aggregate lease and active attempt; contention consumes no attempt or retry and performs no filesystem mutation. |
| Crash at every operation transition | Stale writes are fenced; completed valid checkpoints resume; interrupted output and dependant checkpoints are invalidated. |
| Crash at every replacement, acknowledgement, and outbox boundary | The recovery leader deterministically finalizes or rolls back while holding ownership, then publishes each terminal event exactly once. |
| Shutdown before claim, after claim, during native execution, verification, or finalization | No new claim starts; the same attempt pauses or leaves recoverable durable evidence; shutdown never records terminal cancellation. |
| Operator cancellation at the same boundaries | Cancellation remains durable and terminal and wins during subsequent recovery. |
| More than 1,024 unpublished terminal rows | Stable keyset paging classifies and reconciles the complete set before filesystem mutation. |
| Multiple policy limits and available global slots | Active work never exceeds either the instance-global bound or each job's immutable snapshotted policy bound. |
| Database loss, lease expiry, or stale-owner finalization | New mutations fail closed and every stale owner write is rejected by current generations. |

- During implementation, run state-transition model tests, stored-procedure concurrency
  tests with multiple database sessions, checkpoint corruption and dependency
  invalidation tests, real process interruption tests, every replacement crash
  point, and multi-process recovery-leadership tests.
- An accepted implementation is not complete until focused fault tests,
  `just ci`, and `just ui-e2e` pass.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- Relevant replay commits are PR 108 `b44b5ceb`, PR 109 `d6a8d0ea`, and PR 177
  `c0492c93`, plus replacement-manifest validation `6c495064`, stale-worker
  recovery `f21c8231`, and finalized-replacement completion `885e50c8`.
- Historical ADRs 345, 347, 349, 351, 352, and 437 and historical migrations
  0166, 0167, and 0186 describe replay behavior. Their labels and implementations
  are not treated as decision-specific operator approval.
- Explicitly approved ADRs 448, 483, 500, and 501 establish the resumability,
  crate-boundary, attempt-key, reconciliation, lease-requirement, reserve, and
  native-process constraints preserved by this ADR.
- Approved ADRs 442, 444, 446, 447, and 449 separately govern source-bound
  capability planning, dry-run isolation, immutable worker policy, root
  ownership, and capability binding.
- The untracked proposal at
  `/private/tmp/revaer-media3-restack/docs/adr/503-source-aggregate-lease-and-recovery-leadership.md`
  was inspected as pending evidence. Its `Proposed` status and
  `Operator approval: Pending` field are not approval.

## Follow-up

- Resolve lease refresh, expiry, heartbeat, stale-recovery, and retry-backoff
  timings through a separately approved follow-up using measured fault and
  deployment evidence before implementation requires concrete defaults.
- Define the normalized schema and stored-procedure state machine
  first, then implement scheduler, checkpoint, recovery, replacement, and
  shutdown behavior against that contract.
- Reconcile `MEDIA_TRANSCODING.md`, API and operator status surfaces, deployment
  guidance, and generated contracts in the accepted implementation change.

## Task Record

- Motivation:
  - Convert the shutdown, claim, checkpoint, stale-recovery, replacement, and
    lease gaps exposed by the PR 108-194 replay into one operator decision that
    composes the already approved execution ADRs.
- Design notes:
  - Attempt identity, claim ownership, aggregate ownership, and operation
    checkpoints are separate durable concepts with independent generations.
  - Process shutdown is resumable lifecycle control; operator cancellation is a
    terminal business decision.
  - The recommendation uses the existing PostgreSQL failure domain and does not
    introduce an operation queue or external coordinator.
- Test coverage summary:
  - The ADR-only change added no runtime, schema, filesystem, process, API, or deployment
    tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this ADR-only change.
  - The validation matrix and full repository gates remain mandatory after any
    implementation.
- Observability updates:
  - No telemetry changes are made by this ADR.
  - A future implementation must report bounded attempt and operation
    transitions, claim capacity, lease contention and takeover, checkpoint reuse
    and invalidation, recovery results, and replacement reconciliation reasons
    without high-cardinality identifiers or duplicate error logging.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, replayed shutdown and recovery ADRs, the
    pending ADR 503 proposal, and approved ADRs 442, 444, 446-449, 483, 500, and
    501. This ADR is accepted but does not claim the contract is implemented.
  - `README.md`, roadmap/status documents, operator guides, API contracts, and
    runtime documentation are unchanged; the ADR index and documentation summary
    expose this accepted but unimplemented decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must preserve readable attempt, lease,
    checkpoint, replacement, and outbox evidence and must invalidate rather than
    reinterpret any state an older runtime cannot prove safe.
- Dependency rationale:
  - No new dependency is required. PostgreSQL, existing filesystem transaction
    manifests, checked fingerprints, injected clocks, and current runtime
    primitives cover the recommended contract.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md` as prospective implementation
    constraints.
  - No policy drift was found. Historical migrations remain provenance-only, and
    implementation requiring unresolved timing defaults remains blocked pending a
    separate decision-specific approval.
