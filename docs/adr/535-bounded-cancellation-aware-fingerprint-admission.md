# Bounded cancellation-aware fingerprint admission

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Request-triggered source fingerprinting can read every selected media member to
  end of file without a byte or wall-clock budget. Dispatching that work to a
  general blocking executor does not stop a running read when the request is
  cancelled.
- A single request can identify many source paths. Concurrent abandoned requests
  can therefore consume storage bandwidth, blocking workers, descriptors, CPU,
  and queue capacity long after clients stop waiting.
- A process-local semaphore alone does not enforce per-principal fairness across
  service replicas, bound queued work, or reclaim a worker stuck in a blocking
  filesystem call.
- A partial digest is not a source identity. Budget exhaustion must reject or
  defer admission rather than silently hash a prefix, omit aggregate members, or
  degrade to size and timestamp observations.
- ADR 516 requires bounded aggregate hashing but intentionally leaves exact
  production values unresolved. This decision must define fail-closed admission
  mechanics without silently selecting those evidence-dependent values.

## Options

1. **Keep unbounded general blocking tasks.** This is simple but lets cancelled
   requests retain work and permits storage exhaustion to degrade unrelated API
   operations.
2. **Use a process-local semaphore and cooperative thread cancellation.** This
   bounds concurrent tasks and can check cancellation between chunks, but one
   blocked read cannot be terminated safely and replica-wide principal fairness
   remains absent.
3. **Use a dedicated bounded thread pool plus database quotas.** This isolates
   general blocking work and coordinates principals, but timed-out filesystem
   calls can still pin all dedicated workers indefinitely.
4. **Use killable supervised fingerprint workers with durable admission
   reservations.** Reserve worker, queue, byte, time, request, and principal
   budgets before dispatch; pass only retained read-only handles; terminate the
   isolated worker on cancellation or deadline; and accept only complete
   canonical digests.

## Recommendation

- Adopt option 4 for production fingerprint admission. Injected in-process test
  doubles may remain deterministic, but they cannot establish production
  readiness.
- Introduce a `FingerprintAdmissionController` and a typed
  `FingerprintExecutor`. Production bootstrap supplies a bounded, supervised,
  killable worker implementation; request and domain code cannot call a general
  blocking executor directly.
- Every admission policy version must provide finite, positive values for:
  - maximum running fingerprint workers per service instance;
  - maximum queued fingerprint operations per service instance;
  - maximum aggregate members and observed bytes per request;
  - maximum bytes per member and total bytes read per request;
  - maximum queue time, member execution time, and total request wall time;
  - maximum running and queued operations per authenticated principal;
  - maximum admitted fingerprint bytes per principal in a bounded rolling
    window, plus the window duration and retry delay.
- Treat an authenticated service, scheduler, or operator command as an explicit
  principal with its own policy class. Do not collapse all internal work into an
  unlimited principal. Anonymous or unclassified fingerprint admission fails
  closed.
- Acquire one durable principal reservation through generation-fenced stored
  procedures before queueing. Reserve the observed complete aggregate byte count
  and member count atomically; reject arithmetic overflow, unknown sizes,
  duplicate reservations, and requests above any limit before opening a worker.
- Enforce local worker and queue limits in addition to the durable reservation so
  a single replica cannot exhaust descriptors, memory, process slots, or storage
  bandwidth. Queue time counts against the total request deadline.
- Open members under the accepted managed-root and descriptor rules, and pass
  only typed read-only handles plus expected sizes to the supervised worker. The
  worker reads bounded chunks, reports monotonic byte progress, and cannot open
  host paths, network endpoints, or additional files.
- Count actual bytes as they are read. Growth beyond the reserved observation,
  premature EOF, member mutation, extra output, malformed output, or any budget
  overrun terminates the worker and rejects the complete fingerprint.
- Request disconnect, caller cancellation, lease loss, principal revocation,
  deadline expiry, or service shutdown removes queued work immediately. Running
  work receives cooperative cancellation, then forced termination after one
  bounded grace interval. The supervisor closes inherited descriptors, reaps the
  worker, and releases or accounts for the reservation exactly once.
- A timed-out or cancelled worker result is never admitted, even if it arrives
  concurrently with cancellation. The durable completion procedure requires the
  current reservation generation and exact canonical digest evidence.
- Return bounded overload, quota, size, deadline, cancellation, source-change,
  and worker-failure reason codes. Capacity rejection is explicit and retryable
  only when policy supplies a bounded retry delay; it must not fall back to an
  incomplete fingerprint.
- Exact numeric worker counts, byte ceilings, durations, principal windows, and
  grace intervals are not selected here because no representative benchmark or
  workload evidence accompanies this proposal. Production fingerprint admission
  remains unavailable until the operator explicitly approves those values in an
  amendment or linked ADR.

## Consequences

- Abandoned or adversarial requests cannot create unlimited detached hashing
  work, and one principal cannot consume all replica or deployment capacity.
- Process isolation provides a hard termination boundary for filesystem work
  that an in-process blocking thread cannot safely cancel.
- Worker supervision, durable reservations, fairness, and exact accounting add
  process, database, policy, and operational complexity.
- Finite byte and time limits intentionally reject media outside the approved
  service envelope. Operators receive an explicit bounded diagnostic instead of
  a partial or weaker identity.
- Production activation requires benchmark-backed exact values and multi-replica
  overload validation; architectural acceptance alone would not select them.

## Implementation Boundary

- While this ADR remains Proposed, it authorizes no process, schema, runtime,
  API, UI, workflow, deployment, or policy implementation.
- If explicitly accepted, it would authorize only the typed admission controller,
  finite policy fields, durable principal reservations, local worker and queue
  bounds, supervised killable executor, cancellation ordering, exact accounting,
  readiness hold, bounded reasons, and validation described here.
- Acceptance would not authorize any exact numeric production value or activate
  fingerprint admission. Those values require separate decision-specific
  operator approval supported by representative measurements.
- It would not authorize partial hashing, path-based child access, unlimited
  internal principals, detached blocking tasks, best-effort cancellation,
  unbounded queues, quota bypass, weaker source identity, automatic discovery
  activation, or changes to ADR 516's unresolved scheduler values.
- Persistence, if accepted, belongs only in the approved pre-v1 `init.sql`
  transition under ADR 522. The worker protocol must be bounded, canonical, and
  private; it is not a public media API.

## Validation

- Drive concurrent requests at every boundary: one below, exactly at, and one
  above each worker, queue, member, byte, time, request, principal, and rolling-
  window limit. Prove deterministic admission, fairness, accounting, and bounded
  retry behavior across multiple service replicas.
- Cancel requests while queued, before first read, between chunks, during a
  blocked read, at the final byte, during digest publication, and during service
  shutdown. No cancelled result may become a job identity, and all workers,
  descriptors, reservations, and temporary media must be reclaimed.
- Use large sparse files, slow and fault-injected filesystems, growing and
  shrinking files, early EOF, read errors, duplicate members, arithmetic edges,
  process hangs, malformed worker output, crashes, and forced termination.
- Prove requests for one principal cannot starve another, internal principals
  remain bounded, queue deadlines include waiting time, and replica restart does
  not leak or double-release durable reservations.
- Verify complete known-answer digests at the exact byte ceiling. Requests over
  any ceiling must produce no partial digest, no weaker fingerprint, no job, and
  no source-adjacent artifact.
- Benchmark representative media sizes, aggregate member counts, storage classes,
  concurrent principals, and replica counts before proposing exact production
  values.
- After value approval and implementation, run focused admission and supervisor
  fault tests, complete media fixtures, load and cancellation tests, `just ci`,
  `just ui-e2e`, security scanning, release-image verification, and the strict
  Sonar gate.

## Follow-up

- Obtain explicit decision-specific operator approval before implementation.
- Collect representative throughput, latency, queueing, cancellation, sparse-
  file, slow-storage, and multi-principal evidence, then propose exact worker,
  byte, time, request, principal, and grace values for operator approval.
- If the architecture and values are accepted, reconcile implementation with
  ADRs 501, 512, 516, 518, 522, 523, 526, 527, and 533 without enabling automatic
  discovery or destructive execution through this admission change.

## Task Record

- Motivation:
  - Record the unbounded, non-cancellable request hashing risk found by
    independent review before direct fingerprint admission is exposed as a
    production-safe workflow.
- Design notes:
  - The recommendation separates deployment-wide principal accounting from
    per-instance worker isolation and uses a killable process boundary for hard
    cancellation.
  - Full canonical hashing remains mandatory; limits reject work instead of
    reducing identity quality.
- Test coverage summary:
  - This documentation-only proposal adds no runtime, load, cancellation, or
    media test.
  - Proposal validation is limited to generated documentation indexes, policy,
    instruction drift, link checks, and diff hygiene.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation must expose bounded state and reason metrics for
    worker saturation, queue saturation, principal quota, bytes, deadline,
    cancellation, termination, and reservation recovery. Paths, filenames,
    digests, principal identifiers, request identifiers, and job identifiers are
    forbidden as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 501, 512, 516, 518, 522, 523,
    526, 527, and 533. No current capability, automation, or readiness claim is
    changed by this proposal.
- Risk & rollback plan:
  - The proposal changes no production behavior. If a later accepted
    implementation regresses, disable fingerprint admission, terminate and reap
    workers, expire reservations conservatively, and preserve durable diagnostics;
    never restore unbounded hashing.
- Dependency rationale:
  - No dependency is added by this proposal. A future process-supervision or
    platform wrapper must be justified against existing supervisor primitives,
    standard APIs, auditability, portability, and transitive supply-chain cost.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift, contradiction, or criteria relaxation was found.
