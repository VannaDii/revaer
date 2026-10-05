# Self-contained transcoding and bounded recovery

- Status: Accepted
- Date: 2026-10-02
- Operator approval: 2026-10-02, "Yes, approved, and now restore us to the safe
  rollback point." Approval applies to the revised proposal presented immediately
  before that reply. Distributed workers remain excluded.
- Supersedes: ADR 512 and ADR 513; ADR 588 DISC-1's durable
  directory/name frontier, DISC-3's pre-release upgrade machinery, DISC-5's
  distributed quota/debit accounting, and DISC-6's lease/quiescence coupling.
  Also replaces retention of disposable media after its last consumer finishes,
  including ADR 513's full-workspace age when it would retain such media.
  Replaces corresponding requirements in their supporting appendices and earlier
  ADRs only within this scope. ADR 593 remains authoritative for recovery.
- Implementation status: Approved. Isolated supervisor detour removed;
  cancellation and checkpoint recovery progress is recorded in
  [ADR 596](596-cancellation-checkpoint-recovery.md). Native automatic discovery
  is integrated. Step-5 acceptance is recorded there; remaining strict Sonar
  and package qualification are carried into goal 7, with no release-ready claim.

## Requested Decision

The operator approved this self-contained service model and its four rules.
Approval replaces the conflicts named above, not the feature scope or release
gates. The following decision retains the reviewed proposal's substance.

Revaer performs discovery, scheduling, inspection, planning, execution,
verification, final replacement and recovery itself. It needs no remote workers
or distributed coordinator. Configured fast local storage may hold processing
scratch while original media stays on its configured storage.

1. **Discovery: rescan rather than reconstruct an interrupted scan.** Keep the
   configured schedule, coalesce watcher/schedule triggers, and scan with bounded
   memory. Save discovered candidates and their job identities as they are
   accepted. After interruption or watcher uncertainty, scan again; existing
   identity keys prevent duplicate jobs. No durable per-directory epochs or
   per-filename traversal queue.
2. **Jobs: restart the unfinished step.** Persist completed step checkpoints and
   job outcomes. On service restart, reuse completed work, discard or overwrite
   the unfinished step's Revaer-owned temporary outputs, and rerun that step.
   No separate attempt/aggregate lease clocks, claim-generation takeover protocol
   or per-root recovery leader. Preserve explicit cancellation as cancellation;
   shutdown interruption is resumable and does not consume a failure retry.
3. **Cleanup: delete eligible temporary files and retry failures individually.**
   Protect active jobs, retained checkpoints and unfinished replacement/rollback
   artifacts. Remove an eligible workspace, then acknowledge cleanup through its
   stored procedure. Already absent means success. A failed deletion or database
   acknowledgement is retried on the next sweep without blocking other entries.
   No independently leased cleanup claims or takeover protocol.
4. **Scratch: budget peak space and promptly remove obsolete media.** Before
   starting work, account for the files that must coexist, including staged input,
   current outputs and concurrent jobs. Defer work that will not fit. Reuse paths
   for unfinished outputs and remove intermediates after their last consumer.
   Do not keep a new copy for every transformation or retain completed temporary
   media for diagnostics. Release reserved capacity only when files are gone.

## Implementation Boundary

- One Revaer service process owns each managed root, enforced by the existing
  Revaer root lock; a second process fails admission rather than coordinating
  distributed work. This does not assert exclusivity against unrelated software.
  Preserve configured in-process job concurrency and one active replacement per
  source. Stop old work before replay; do not recreate a custom supervisor.
- Keep normalized stored-procedure persistence for schedules, candidates, job
  identities, checkpoints and outcomes. Keep existing job/attempt workspace keys;
  do not create a compatibility conversion layer. A checkpoint reference does
  not require keeping obsolete media: rebuild from an earlier usable checkpoint
  or the unchanged original when a required intermediate has been removed.
- Discovery retains source/aggregate identity, sidecar selection, immutable
  configuration binding and duplicate-enqueue prevention. Incomplete scans cannot
  establish absence; keep two clean observations before diagnostic tombstones.
  Manual-only associations remain manual. Configuration activation still triggers
  re-evaluation without silently changing an existing job's snapshot.
- Retain file/aggregate size, traversal, bounded queue, memory, process deadline
  and configured concurrency limits. Remove principal/deployment rolling byte
  debit ledgers, distributed occupancy reservations, lease-derived timing
  inequalities and the slow-storage-derived 21/84-day scan envelope. Slow scans
  need ordinary progress/cancellation, not a promise based on assumed throughput.
  Do not replace removed limits with a new quota subsystem.
- Retain existing source checks under ADR 592. Changed or missing sources stop
  that attempt and receive a recorded outcome, not replacement or silent rebind.
  Missing/corrupt completed intermediates are rebuilt from the preceding usable
  checkpoint; otherwise normal checkpoint reuse applies.
- Originals remain untouched until verified final replacement. Reconcile an
  interrupted replacement using its existing backup/manifest before processing
  or deleting its artifacts. Reusable intermediates need no ownership receipts.
- Clean disposable media on completion, cancellation and failure, preserving only
  files needed for resumable work or unfinished replacement/rollback. Reclaim
  abandoned job scratch at startup after recovery classifies it. Failed cleanup
  stays charged against scratch capacity and is retried; never admit more work
  on the assumption that deletion succeeded. Preserve existing bounded sweep
  cadence and batch sizes for retries, and diagnostic-record retention, not
  age-based retention of unnecessary media. Remove only identified Revaer-owned
  paths; unknown files are not cleanup candidates. No cleanup leadership protocol.
- Replace distributed takeover/lease/protocol acceptance tests with tests of
  this actual single-owner workflow. This is an explicit scope change, not a
  waiver of CI, UI, security, coverage or real recovery verification.

## Future Workers Are Not Implementation Scope

A future separate worker image could register with a primary and process on
local fast disks. Root ownership and final replacement could remain with the
primary. That is a future design requiring its own approval, not a current
dependency or a permanent prohibition. Do not implement worker registration,
remote work transfer, autoscaling, new broker/control protocols, or speculative
distributed scaffolding under this ADR.

## Pros And Cons

**Pros:** fewer tables and coordination protocols; straightforward restart and
cleanup; implementation effort directly serves the operator workflow.

**Cons:** interrupted discovery repeats scanning, and interrupted steps repeat
some processing. Multiple Revaer replicas cannot share a root. There is no
per-principal bandwidth fairness guarantee. These are acceptable first-release
tradeoffs; distributed scheduling is not part of this proposal. Prompt cleanup
can mean recomputing a removed intermediate after interruption. A full scratch
disk defers work instead of allowing storage growth without a bound.

## Validation And Delivery

Demonstrate real service restart during discovery without duplicate jobs; restart
during processing with temporary output rebuilt and originals unchanged; explicit
cancellation; source change/loss; interrupted final replacement recovery; and
cleanup failure followed by successful retry without deleting protected files.
Prove second-process root rejection and configured in-process concurrency.
Demonstrate scratch admission on insufficient space, bounded peak usage across
concurrent jobs, prompt intermediate deletion, startup reclamation and capacity
remaining charged after a deletion failure. Exercise real local execution
without any registered worker or remote processing service.

Implement outside-in, single-agent, preserving useful existing workflow work.
Change the single init only when required by this workflow; no migrations,
backward-compatibility work or independent database investigation. On approval,
update predecessor status and scoped instructions before removing obsolete code.
All existing release gates remain: `just ci`, `just ui-e2e`, strict Sonar with
positive coverage, applicable GitHub checks, resolved feedback, validated Linux
amd64/arm64 packages and the linear PR stack with at most 10,000 changed lines.

## Task Record

- Motivation: deliver the full workflow locally with simple restart and bounded
  scratch usage instead of distributed coordination and accumulating media copies.
- Design notes: repeat interrupted work, preserve completed checkpoints, and
  reserve reconciliation for final replacement. Approval scope is explicit above.
- Test coverage summary: rollback formatting, instruction drift and whitespace
  checks passed, as did four restored entrypoint tests and a fresh Linux arm64
  app test-binary build. Full CI/UI were run and failed as recorded below;
  no complete runtime or release qualification claimed.
- Observability updates: retain scan progress, job/step outcomes and cleanup
  failures with bounded reasons; no new telemetry framework.
- Status-doc validation: ADR navigation updated; predecessors remain accepted
  now superseded within the stated scope. No product readiness claim changed.
- Risk and rollback plan: partial adoption could retain contradictory protocols.
  Apply approved instruction/code changes together, preserving user edits. Before
  release, revert only that scoped implementation if required; never delete
  original media or recovery backups to roll back code.
- Dependency rationale: no new dependency proposed.
- Stale-policy check: reviewed global simplicity rule, supplied root instructions,
  scoped Rust instructions, ADRs 512/513/592/593 and ADR 588's discovery approval
  delta. Operator clarified local primary-worker capability and bounded scratch;
  distributed workers are future work only. Operator approval replaces only
  the named contracts; quality criteria remain unchanged. ADR 593's retained
  discovery approval and predecessor/support notices are aligned below.

## Safe Rollback Checkpoint, 2026-10-02

- Removed the isolated fingerprint helper/reader, supervisor, registration codec,
  owner registry, identity adapter, control pipes and their dedicated tests.
- Restored the ordinary async application entrypoint. Removed only the detour's
  direct `nix` dependency and `rustix` pipe/time features, preserving other lockfile
  and dependency changes.
- Preserved configuration/API/UI, single-init schema, discovery source-loss checks,
  job execution/replacement and the CI fixture correction. No branch reset,
  blanket checkout, or deletion of unrelated work was performed.
- This rollback removes a detour; it does not claim the retained workflow or the
  simplified recovery design is fully qualified. Current validation results and
  the next action are recorded after verification.
- Preservation check: SHA-256 before/after matched for selected configuration,
  job runtime, discovery/fingerprint/watcher, init, API handler, UI view and
  attribute-race fixture files. Application entrypoint and workspace dependency
  definitions match the pre-detour HEAD while useful bootstrap changes remain.
- Passed: `just fmt`, `just instruction-drift`, `git diff --check`, four locked
  offline minimal-feature app entrypoint tests, and fresh Linux arm64 library
  test-binary compilation. These are rollback checks, not full service proof.
- Current CI failure: `just ci` stops in policy at
  `K1 frozen source changed: scripts/database_rebaseline/final_sql.rb`.
  Evidence: `target/rollback-594-ci.log`. The rollback did not change this file.
- Current UI failure: fresh-source `just ui-e2e` passes 81 tests but fails native
  watcher and schedule activation with 409 `media_configuration_pending_contract`;
  110 tests did not run and global UI coverage remains incomplete.
  Evidence: `target/rollback-594-ui-e2e.log`. This is not frontend-only evidence
  or a passing full operator workflow. No fresh Sonar/package/review pass claimed.
- Cleanup: disposable CI/UI database containers removed by harness cleanup;
  owned UI temporary root removed. No test media remained in checked worktree
  target/test-result/fixture directories. Active worktree retained for ongoing work.
- Next action: finish native watcher/schedule admission through the real operator
  workflow under this accepted design; fix only demonstrated dependencies. Treat
  the frozen-source CI failure as a bounded gate repair, not a new DB investigation.
