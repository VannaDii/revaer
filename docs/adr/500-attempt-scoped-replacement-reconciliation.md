# Attempt-scoped replacement reconciliation

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Worker-owned database writes and replacement transaction names now carry the
    active claim generation, and terminal outbox rows retain the exact attempt.
    Managed workspaces are still keyed only by job UUID, however, and startup
    reads at most 1,024 unpublished terminal rows before classifying every
    replacement transaction.
  - The worker revalidates the descriptor-bound media-and-sidecar fingerprint
    after replacement preparation, but that check and the first rename are not
    one atomic filesystem operation. A noncooperating writer can still modify a
    destination in that interval.
  - Preparing durable candidate and recovery copies consumes source-filesystem
    capacity and cannot currently observe cancellation while a large copy is in
    progress.
- Options:
  - Recommended: use `(job UUID, attempt number, claim generation)` for managed
    workspaces, replacement manifests, and terminal reconciliation; page the
    outbox by stable keyset until exhausted; and require an exclusive cooperative
    lease for every enrolled source aggregate from claim validation through
    finalize or rollback. Recheck pinned destination identities and the aggregate
    fingerprint immediately before the first mutation. Make preparation
    cancellation-aware and require its checked byte estimate plus the existing
    1 GiB source-filesystem reserve through an injected capacity probe.
  - Keep job-only workspaces, preserve the fixed outbox page, and require manual
    cleanup before retry. This avoids a contract change but leaves recovery
    incomplete and can block unrelated committed work.
  - Use platform-specific mandatory locking or filesystem snapshots. This can
    narrow the race on selected systems but is not portable across the supported
    deployment targets and would make those facilities production prerequisites.
- Recommendation:
  - Adopt the first option. Treat roots without a sole-writer or cooperative-lease
    guarantee as ineligible for destructive processing. Descriptor identity
    checks remain defense in depth; they cannot provide an absolute guarantee
    against writers that ignore the lease.
- Consequences:
  - Retry work cannot alias a prior attempt's workspace or transaction, and
    startup classifies the complete durable terminal set before mutating files.
  - Source edits fail closed at the mutation boundary, while the documented
    ownership contract closes the otherwise unavoidable check-to-rename race.
  - Deployments that cannot coordinate every writer remain dry-run-only.
  - Workspace layout, stored procedures, manifests, cleanup, readiness, and
    recovery tests must change together.
- Follow-up:
  - After approval, add retry-collision, outbox-page overflow, concurrent-writer,
    reserve-loss, cancellation-during-copy, and every-crash-boundary tests.

## Implementation Boundary

- Approval authorizes only the attempt key, complete keyset reconciliation,
  cooperative lease, immediate identity recheck, and 1 GiB staging reserve
  described above.
- Approval does not authorize filesystem-specific snapshots, mandatory locks,
  background migration of legacy transactions, or a different capacity default.

## Task Record

- Motivation:
  - Close the remaining recovery and source-ownership ambiguity found by the
    independent PR 81 execution review.
- Design notes:
  - The terminal database commit still precedes replacement finalization.
    Uncertain work still rolls back, and durable completed work still finalizes.
- Test coverage summary:
  - Existing coverage proves claim-fenced terminal writes, exact replacement
    transaction keys, aggregate revalidation, rollback, and cancellation. The
    follow-up matrix above is required before this proposal is implemented.
- Observability updates:
  - Recovery logs and bounded metrics will include attempt and claim generation,
    never source paths as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 318, 419, 429, and 495. This proposal
    does not change a current capability claim.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. During implementation,
    rollback must preserve or conservatively quarantine every attempt-scoped
    transaction before restoring an older layout.
- Dependency rationale:
  - No dependency is proposed. Existing descriptor, capacity, and database
    primitives cover the recommendation.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`.
  - No policy drift or contradiction was found.
