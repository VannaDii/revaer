# Attempt-scoped workspace retention transaction

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- The replayed workspace janitor protects and deletes directories by media-job
  UUID. Approved ADR 500 later requires managed workspaces, manifests, and
  terminal reconciliation to use `(job UUID, attempt number, claim generation)`.
  Job-only retention can therefore alias retries or delete another generation's
  checkpoint source.
- Retention starts concurrently with media-job startup recovery. Tokio's first
  interval tick is immediate, so an old but recoverable workspace can be removed
  before replacement, outbox, lease, or checkpoint ownership is classified.
- The current janitor removes filesystem content before database retention, but
  any one per-entry filesystem failure defers all database pruning. This preserves
  evidence but lets one damaged path block unrelated successful cleanup.
- Resumable attempts introduce paused, recovering, and prior-generation
  checkpoint workspaces that must remain protected even when they are not the
  current running claim.
- Cleanup must remain bounded, deterministic, stored-procedure-backed, injected,
  preservation-biased under uncertainty, and compatible with per-root recovery
  leadership from proposed ADR 512.

## Options

1. **Retain job-only protection and global deferral.** Preserve current behavior
   and increase retention ages to reduce collisions. This cannot isolate retries
   or guarantee recovery safety.
2. **Use attempt-generation keys and per-attempt cleanup transactions.** Protect
   every recoverable workspace tuple, order startup behind root recovery, and
   acknowledge filesystem and database cleanup independently per tuple.
3. **Prune database evidence before filesystem removal.** Delete diagnostic rows
   first and clean files afterward. This simplifies database retention but can
   destroy the only durable evidence needed to classify or retry a failed
   filesystem cleanup.

## Recommendation

- Adopt option 2.
- Every managed workspace and cleanup candidate is identified by `(job UUID,
  attempt number, claim generation)`, matching approved ADR 500. A retry or
  resumed claim cannot alias another attempt or generation's workspace.
- The coherent retention snapshot protects every workspace needed by an attempt
  in `running`, `pausing`, `paused`, `recovering`, `verifying`, or `finalizing`
  state, every validated checkpoint referenced by a nonterminal attempt, and
  every terminal workspace whose replacement, finalization acknowledgement, or
  outbox reconciliation remains incomplete.
- A queued job with no workspace requires no filesystem key. If admission or a
  prior implementation created a queued workspace, it remains protected until
  recovery classifies it explicitly.
- Startup retention waits for the proposed ADR 512 recovery barrier for each
  managed root. The recovery leader first classifies attempts, leases,
  checkpoints, replacement manifests, and the complete terminal outbox; only
  then may retention inspect that root.
- Each candidate uses one per-attempt cleanup transaction:
  1. A stored procedure takes a coherent eligibility snapshot and records or
     renews a bounded cleanup claim without pruning evidence.
  2. The injected filesystem cleaner removes only the exact attempt-generation
     path and records a bounded outcome. An already absent path is idempotent
     success.
  3. After filesystem success, a stored procedure acknowledges removal and
     prunes only the database diagnostics whose policy has expired.
- A filesystem or acknowledgement failure remains attached to that exact
  attempt-generation candidate and is retried later. Successful unrelated
  candidates continue through acknowledgement and database pruning in the same
  sweep.
- A process crash after filesystem removal but before database acknowledgement
  is safe: the retained cleanup claim and missing-path success make the next run
  complete the database transition without reconstructing or guessing state.
- Filesystem directories with no database row are treated as orphans only when
  the directory name parses as the complete attempt-generation key, the
  configured age has expired, root recovery is complete, and no replacement
  manifest, checkpoint reference, lease, or outbox evidence claims ownership.
  Ambiguous or malformed entries are preserved and reported.
- Retention continues to distinguish complete workspaces from diagnostics-only
  failed or cancelled workspaces. Terminal cleanup may compact diagnostic state,
  but deletion must preserve every artifact still referenced by audit,
  checkpoint, quarantine, replacement, or rollback evidence.

### Supported Operational Values

- Run one immediate sweep after the relevant root recovery barrier opens, then
  preserve the replay's one-hour scheduled cadence.
- Preserve the persisted default full-workspace age of 24 hours.
- Preserve the persisted default diagnostics-only age of 720 hours.
- Preserve the persisted default bound of 128 examined candidates per sweep.
- Preserve existing schema validation bounds of 1 through 87,600 hours for both
  workspace ages and 1 through 4,096 candidates per sweep. These are accepted
  ingress bounds, not a reason to change the recommended defaults.

### Explicitly Undecided Timings

- No separate retry delay is selected for a failed cleanup candidate; absent a
  later approved policy, it becomes eligible at the next scheduled sweep.
- Cleanup-claim expiry, abandoned-cleaner takeover, and fairness cursor lifetime
  remain undecided until the worker lease and recovery timings in proposed ADR
  512 are selected and fault-tested.
- This proposal does not introduce an additional startup delay after the recovery
  barrier or shorten either persisted retention age without operational evidence.

## Consequences

- Retry and resume workspaces cannot collide or be deleted through a shared job
  UUID.
- Recovery, retention, and terminal reconciliation share one ownership key and
  one per-root startup order.
- Filesystem deletion still precedes destructive database evidence pruning, but
  one failed path no longer blocks unrelated successful database retention.
- Cleanup requires durable per-candidate state or equivalent generation-fenced
  acknowledgement, increasing schema and stored-procedure surface.
- Paused and recovering attempts can retain workspace capacity longer than
  terminal attempts. Capacity and retention observability must make that use
  explicit without deleting active recovery evidence.
- Orphan handling becomes deliberately conservative; malformed or uncertain
  directories require operator remediation rather than heuristic deletion.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only the attempt-generation retention key, protected
  state set, per-root recovery barrier, per-attempt cleanup transaction,
  filesystem-before-evidence-pruning order, scoped partial-failure retry, orphan
  eligibility rules, and supported default values described above.
- Approved ADR 500's attempt key and proposed ADR 512's worker ownership remain
  authoritative inputs. This proposal does not redefine claim, checkpoint,
  aggregate-lease, replacement, or outbox semantics.
- Before v1, accepted persistence changes must update
  `crates/revaer-data/init.sql`. Historical migration 0188 is provenance only and
  must not be restored.
- Acceptance would not authorize database-first diagnostic deletion, job-only
  workspace aliases, unbounded directory scans, heuristic deletion of malformed
  entries, deletion before root recovery, a background legacy-layout migration,
  or shorter retention defaults.
- This proposal does not change completed-job or failed-job database-history
  policy beyond coordinating eligible diagnostic pruning after filesystem
  success. A different history-retention contract requires separate approval.
- Runtime collaborators remain injected and all runtime persistence remains
  stored-procedure-backed.
- No schema, runtime, filesystem, API, deployment, or documentation contract may
  change until this ADR receives explicit decision-specific operator approval.

## Validation

- Proposal validation is documentation-only; this record changes no retention,
  filesystem, schema, recovery, or database behavior.

| Scenario | Required result after approval |
| --- | --- |
| Service was stopped beyond a retention window | Paused, recovering, checkpoint-referenced, and unreconciled workspaces survive the startup sweep. |
| One job has multiple attempts and claim generations | Only the exact eligible tuple is removed; every protected tuple remains unchanged. |
| One filesystem removal fails while others succeed | Successful candidates acknowledge and prune; the failed candidate retains evidence and retries independently. |
| Process crashes after removal but before acknowledgement | The next run treats the absent path as success and completes only that candidate's database transition. |
| Database fails before or after filesystem removal | No unclaimed path is removed; after removal, retained cleanup state permits safe acknowledgement retry. |
| Root recovery is incomplete or leadership changes | Retention performs no mutation until the current recovery leader opens the barrier. |
| Malformed, ambiguous, or still-owned orphan directory | The entry is preserved, reported with a stable reason, and excluded from automatic deletion. |
| More candidates exist than the configured bound | Each run remains bounded and the persisted cursor or ordering eventually reaches every eligible candidate. |

- After approval, add stored-procedure and filesystem integration tests for every
  protected state, generation collision, policy edge, partial failure, absent
  path, acknowledgement crash, root outage, leadership takeover, malformed
  orphan, and bounded-fairness case.
- Add startup integration coverage proving replacement and outbox recovery finish
  before the first retention mutation.
- An accepted implementation is not complete until focused tests, `just ci`, and
  `just ui-e2e` pass.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- The principal replay commit is PR 170 `2ed2be27`; PR 177 `c0492c93` supplies
  the later integrated media runtime behavior. Commit `8572b1fe` retired the
  final v0 migration history into `crates/revaer-data/init.sql`.
- Historical ADR 435 and historical migration 0188 describe job-only protection,
  an immediate first tick, hourly cadence, filesystem-first cleanup, global
  database deferral, and the current default values. Their implementation and
  `Accepted` label are not decision-specific operator approval.
- Explicitly approved ADR 500 supersedes ADR 435's job-only workspace identity
  with the attempt-and-generation tuple. Approved ADRs 448 and 483 constrain
  checkpoint ownership and implementation boundaries.
- Proposed ADR 512 supplies the root recovery barrier and resumable attempt states
  consumed by this recommendation. It remains pending and authorizes no
  implementation unless separately approved.

## Follow-up

- Obtain explicit decision-specific operator approval before implementation or
  before changing this ADR to `Accepted`.
- Decide this proposal together with proposed ADR 512 because retention safety
  depends on its attempt states, root recovery barrier, and ownership evidence.
- If approved, implement the normalized eligibility and cleanup-acknowledgement
  contract before changing filesystem traversal or startup ordering.
- Reconcile `MEDIA_TRANSCODING.md`, retention policy documentation, operator
  health surfaces, and capacity guidance in the accepted implementation change.

## Task Record

- Motivation:
  - Replace the replay's job-only janitor contract with an operator-reviewed
    retention transaction that cannot collide with approved attempt-scoped
    workspace and recovery ownership.
- Design notes:
  - The database proves eligibility and records cleanup ownership, the filesystem
    is removed next, and database evidence is pruned only after removal succeeds.
  - Failures are isolated to one attempt-generation candidate while uncertainty
    always fails toward preservation.
- Test coverage summary:
  - This proposal adds no retention, schema, filesystem, recovery, or integration
    tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this proposal-only change.
  - The validation matrix and full repository gates remain mandatory after any
    approval and implementation.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation must expose bounded cleanup eligibility, protected,
    removed, partial-failure, acknowledgement-retry, orphan, and recovery-barrier
    outcomes without using paths, jobs, attempts, or generations as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, ADR 435, approved ADRs 448, 483, and 500,
    proposed ADR 512, PR 170 and PR 177 behavior, and current retention schema.
    This proposal does not claim its recommendation is approved or implemented.
  - `README.md`, roadmap/status documents, operator guides, and runtime behavior
    are unchanged; only the ADR index and documentation summary expose this
    pending proposal.
- Risk & rollback plan:
  - This proposal changes no production behavior and can be rolled back by
    removing this ADR and its two catalogue entries.
  - After implementation, rollback must preserve every workspace or diagnostic
    record whose exact ownership cannot be represented by the older runtime;
    uncertain candidates must be quarantined or retained rather than rekeyed.
- Dependency rationale:
  - No dependency is proposed. Existing stored procedures, injected clocks,
    bounded filesystem primitives, and current workspace layout utilities are
    sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md` as prospective implementation
    constraints.
  - No policy drift was found. Historical migration 0188 remains
    provenance-only under the current pre-v1 `init.sql` rule.
