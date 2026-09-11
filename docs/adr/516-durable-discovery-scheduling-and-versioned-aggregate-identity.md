# Durable discovery scheduling and versioned aggregate identity

> Current approval: The operator accepted ADR 588's exact applicable choices
> on 2026-09-11 at reviewed commit `9575c077`. The [resolution](588-first-release-decision-package.md#approval-resolution)
> releases only those named design holds. Earlier pending/candidate wording
> below retains its historical context; implementation and qualification are
> not certified, and all other conditions remain binding.

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- `MEDIA_TRANSCODING.md` requires manual discovery by default, disabled-by-default
  watchers and schedules per path-to-profile association, operator-selected
  minute or hour intervals, immediate enqueueing, missed-event recovery,
  immutable job snapshots, and fingerprints for change detection, caching,
  resumability, and reconversion prevention.
- The replayed scheduler keeps due times, watcher debounce state, overflow state,
  and scan cursors in one process. Restart resets schedule history and traversal,
  and large scans continue only when another schedule or watcher-overflow trigger
  occurs.
- Fixed polling, debounce, event-capacity, traversal, directory, sidecar, byte,
  and elapsed limits are embedded in runtime code. Tests prove those bounds are
  enforced, but no representative library, filesystem, restart, or overload
  evidence establishes that the values are correct production defaults.
- Replayed aggregate identity hashes media and fixed-heuristic sidecars twice
  before enqueue, then hashes again during execution without an aggregate hashing
  deadline. Sidecar membership bypasses approved ADR 450 snapshot rules, and
  ambiguous ownership can omit a sidecar instead of failing the aggregate.
- Database deduplication keys observed state by profile and source path. An
  unchanged source does not create new work merely because target or policy
  versions changed, even though the baseline planner cache key includes source,
  target, policy, and capability identity.
- Historical ADR 430 describes the replayed scheduling and aggregate choices but
  has no decision-specific operator approval. Its `Accepted` label and existing
  tests are provenance, not authorization under ADR 445.
- Scheduler ownership, cursor durability, aggregate membership and encoding,
  deduplication, overload behavior, and production automation cross database,
  filesystem, process, policy, API, UI, and deployment boundaries and require an
  explicit architectural decision.

## Baseline And Held Existing Choices

- The baseline requires automatic discovery to exist, but explicitly keeps it
  disabled by default and does not select the replayed scheduler topology or
  constants.
- Automatic watchers and scheduled scans must remain disabled until exact
  scheduler values and budgets receive separate approval and the durable
  implementation passes its validation gates. Existing profile flags, normalized
  rows, or passing watcher tests do not authorize activation.
- The current aggregate algorithm, fixed sidecar ownership rules, resource
  limits, and path-plus-observation deduplication are held from destructive use.
  They must not authorize source replacement, backup, quarantine, source-adjacent
  publication, or another mutation.
- Manual preview and dry-run planning may remain available only when they create
  no source-adjacent files and do not treat the current aggregate as sufficient
  destructive evidence. Any later destructive execution must use the approved
  aggregate and source-lease contract.
- Reconstruction must fail closed on ambiguous ownership, unsupported aggregate
  versions, incomplete member evidence, or exhausted mandatory identity budgets.
  It must not silently omit members, restart with permissive defaults, or degrade
  to path and timestamp identity.
- This ADR changes no runtime behavior by itself. Its acceptance authorizes
  fail-closed enforcement and the durable architecture, while automatic and
  destructive behavior remains held pending separate exact-value approval.

## Options

1. **Ratify the process-local scheduler and current aggregate.** Preserve the
   embedded polling, debounce, capacities, in-memory cursors, fixed sidecar
   heuristics, repeated full hashing, and profile/path deduplication. This is the
   smallest change but is not restart-deterministic, policy-complete, or supported
   by production-scale evidence.
2. **Use a durable scheduler and hash every aggregate on every observation.**
   Persist due work and scan cursors in PostgreSQL and compute a complete
   cryptographic aggregate for every candidate on every scan or event. This is
   straightforward and exact but can repeatedly read very large libraries and
   media without using stable observations as a safe cache hint.
3. **Use a durable fenced scheduler with two-tier versioned identity.** Persist
   due runs and traversal progress, use bounded descriptor-derived member
   observations to decide when a cryptographic aggregate must be recomputed, and
   persist a versioned exact member manifest and digest for job safety. Revalidate
   that exact aggregate at claim and immediately before mutation.
4. **Retain manual-only discovery.** Do not activate watchers or schedules until a
   later release. This is a safe interim posture but does not deliver the complete
   first-release automation required by the baseline.

## Recommendation

- Adopt option 3 as the accepted architecture. Production activation follows only
  after the operator selects or revises the unresolved choices.
- Make PostgreSQL the durable source of truth for discovery schedule state,
  rescan requests, run generations, ownership, and bounded traversal progress.
  All runtime database access remains through stored procedures.
- Claim at most one active discovery run for a managed root binding under a
  generation fence. Multiple service processes must not scan or advance the same
  run concurrently, and stale owners must not publish candidates or cursors.
- Anchor schedule progression to durable due time rather than process startup.
  Coalesce missed intervals into bounded recovery work rather than producing an
  unbounded catch-up storm.
- Continue an incomplete scan through fair bounded batches without waiting for a
  new configured cadence. Preserve progress in normalized frontier rows or
  another explicitly approved relational representation; do not use JSONB.
- Treat native watcher events as advisory acceleration. Coalesce events by root
  and aggregate owner, and convert overflow or watcher uncertainty into one
  durable rescan request. Scheduled scans remain the recovery path for missed
  events, restarts, and offline storage.
- Traverse from the approved ADR 447 managed-root identity using descriptor-
  relative, no-follow operations and deterministic ordering. A cursor is valid
  only for its root identity and discovery-run generation.
- Compile sidecar membership from approved ADR 450's versioned, ordered,
  snapshotted token grammar through the ADR 451 effective policy. Fixed extension
  or prefix fallback is not permitted when required rules are absent or invalid.
- Persist a versioned aggregate contract containing an exact normalized member
  manifest and cryptographic digest. Member kind, canonical root-relative name,
  stable observations, content evidence, sidecar rule provenance, aggregate
  contract version, and digest algorithm must have unambiguous canonical
  semantics.
- Treat size, modification time, change time, device, and inode as bounded change
  observations and race defenses, not as cryptographic identity. Recompute the
  exact digest whenever approved observations or membership change, and do not
  perform a redundant second full hash merely to enqueue stale work safely.
- Revalidate the complete approved aggregate on first successful claim and
  immediately before the first mutation. Approved ADR 500's cooperative source
  lease remains required through destructive finalization or rollback.
- Deduplicate discovery intent by managed-root binding, canonical root-relative
  source key, aggregate contract version and digest, target version, and policy
  version. ADR 449 continues to bind capability on first successful claim, so
  capability identity joins later planning and audit evidence rather than being
  silently selected by the discovery scheduler.

## Unresolved Operator Choices

- The discovery-run claim cadence, ownership refresh, expiry, contention, and
  retry timings.
- The missed-interval policy beyond bounded coalescing, including whether one or
  more historical intervals may be represented in audit evidence.
- Global, per-root, and per-policy scan concurrency and fairness.
- Default and maximum traversal depth, entries, files, selected bytes, elapsed
  time, directory entries, event capacity, debounce, aggregate members, sidecar
  bytes, hashing bytes, and hashing duration.
- Whether the exact traversal frontier is stored as directory and name rows,
  ordinal work items, or another normalized relational contract.
- The initial aggregate contract version, canonical encoding, digest algorithm,
  and algorithm-upgrade procedure.
- Exact member scope beyond the primary media and sidecars selected by approved
  rules, including companion files and any future external metadata.
- Handling for an aggregate or directory that exceeds an approved hard limit:
  terminal diagnostic, operator-remediable pause, or another fail-closed state.
- Deletion and rename semantics for stale discovery observations and whether they
  create explicit jobs, tombstones, or only durable discovery diagnostics.
- Whether target or policy changes automatically create rescan requests for all
  affected unchanged sources or are activated through an explicit operator
  re-evaluation command.
- No replayed capacity, timing, hash encoding, or ownership heuristic is adopted
  by this ADR merely because it already exists or has unit tests.

## Consequences

- Schedules, overflow recovery, and traversal resume deterministically across
  restart and multiple service processes.
- Watchers remain a latency optimization rather than the sole source of truth for
  library state.
- Exact aggregate identity becomes a versioned auditable contract aligned with
  immutable policy and target intent instead of a hidden path heuristic.
- Relational run, frontier, rescan, member, and identity state adds schema and
  stored-procedure complexity and requires retention and reconciliation rules.
- Two-tier identity avoids repeated full reads when stable evidence proves no
  observation changed, while mandatory full revalidation preserves destructive
  safety.
- Existing discovered jobs or fingerprints cannot be silently reinterpreted as a
  new aggregate version. They must be re-observed, explicitly migrated by an
  approved decision, or held from destructive execution.
- Automation remains unavailable until the durable model, operator controls,
  observability, and fault validation are complete.

## Implementation Boundary

- This accepted ADR authorizes only durable fenced discovery-run ownership,
  relational scan progress and rescan requests, watcher coalescing and overflow
  recovery, approved-root traversal, versioned exact aggregate identity,
  and configuration-aware deduplication. Limits and failure states remain outside
  this authorization until explicitly approved by the operator.
- This acceptance does not authorize production activation without selected
  scheduler timings, budgets, aggregate encoding, and limit behavior. Those
  values must be approved in this ADR or an explicitly linked follow-up decision
  before automatic discovery is enabled.
- Any accepted persistence work belongs in the v0 `init.sql`; historical
  migrations 0152, 0182, and 0187 remain provenance and must not be replayed as
  the implementation mechanism.
- The implementation surface is limited to normalized discovery persistence and
  stored procedures, injected app runtime services, managed-root traversal,
  watcher adapters, aggregate construction and revalidation, API/UI operator
  surfaces, bounded telemetry, and their tests.
- Accepted ADR 512 separately owns the worker state machine; its exact lease
  timings remain unresolved. Discovery-run ownership must compose with, but remain
  distinct from, worker attempt ownership and ADR 500's destructive source lease.
- This ADR does not authorize automatic enablement, JSONB application state,
  path or timestamp identity, fixed unsnapshotted sidecar fallback, an external
  scheduler, weaker root checks, criteria suppression, or unrelated worker,
  replacement, target, or policy behavior.
- Approved ADRs 445, 446, 447, 449, 450, 451, 484, and 500 remain binding.
- Structural schema, runtime, API, UI, workflow, generated-contract, deployment,
  and specification changes may implement only the accepted disabled and fail-
  closed architecture. Automatic or destructive behavior may not become active
  until separate decision-specific approval records the unresolved values.

## Validation

- Decision validation to date is documentation-only. This ADR adds no
  implementation or behavioral test.

| Scenario | Required result after exact-value approval and implementation |
| --- | --- |
| Restart before, during, and after each scan batch | Durable due state and the fenced frontier resume without duplicate ownership, lost progress, or permissive restart. |
| Two or more service processes claim one root | Exactly one current run generation advances or publishes; every stale write is rejected. |
| Multiple missed intervals or offline storage | Recovery is bounded, observable, and storm-free; no interval silently enables destructive work. |
| Watcher burst, overflow, loss, rename, create, modify, and delete | Events coalesce deterministically and uncertainty creates one durable rescan request; the scheduled recovery path converges. |
| Huge, deep, changing, non-Unicode, unreadable, or symlink-heavy trees | Approved limits and root rules fail closed with resumable or terminal evidence selected by policy; no root escape occurs. |
| Sidecar rule precedence, ambiguity, and VobSub companions | Membership follows the snapshotted ADR 450 grammar exactly; ambiguity and incomplete required companions never disappear silently. |
| Aggregate member changes during hashing, enqueue, claim, or replacement | Unstable evidence is rejected, stale jobs cannot mutate sources, and pre-mutation revalidation detects the complete keyset change. |
| Same source under a new target or policy version | Configuration-aware discovery creates explicit new intent or an operator-visible re-evaluation state according to the approved choice. |
| Aggregate algorithm or encoding upgrade | Old and new versions remain distinguishable; no digest is reinterpreted under different canonical semantics. |
| Budget exhaustion and cancellation | Work stops within approved bounds, persists valid progress only under the current generation, and reports bounded reasons without losing the recovery path. |

- Before production activation, add stored-procedure concurrency and fencing
  tests, restart and failover tests, traversal property and race tests, large-
  directory and large-media benchmarks, watcher fault injection, sidecar
  ownership matrices, aggregate known-answer vectors, deduplication tests across
  target and policy versions, and destructive pre-mutation rejection tests.
- An accepted implementation is not complete until strict Sonar coverage and
  result guardrails, the complete media conversion and filesystem fault matrix,
  `just ci`, and `just ui-e2e` pass.

## Provenance

- Evidence was inspected read-only in
  `/private/tmp/revaer-approved-decisions-stack` on branch
  `work/media3-approved-decisions-stack`.
- The baseline requirements are in `MEDIA_TRANSCODING.md`, including discovery
  defaults and associations, immediate enqueueing, missed-event recovery,
  immutable configuration snapshots, fingerprint types, and the planner cache
  key.
- Replayed scheduling, watcher, traversal, aggregate, and enqueue behavior is in
  `crates/revaer-app/src/media_discovery_runtime.rs`,
  `media_discovery_watcher.rs`, `media_discovery_scan.rs`,
  `media_discovery_fingerprint.rs`, and the discovery job procedures called by
  `crates/revaer-data/src/media/jobs.rs`.
- Relevant replay commits include `133ffb43` and `75eb2b9f`, with earlier review
  hardening and test commits in their ancestry. Existing implementation,
  authorship, and passing tests are not decision-specific approval.
- Historical ADR 430 and migrations 0152, 0182, and 0187 describe current or
  attempted behavior. They are provenance only and do not authorize the
  scheduler, limits, aggregate encoding, or deduplication contract.
- Approved ADRs 445, 446, 447, 449, 450, 451, 484, and 500 establish governance,
  immutable snapshot transport, managed-root ownership, capability binding,
  sidecar grammar, effective policy compilation, operator workflows, and
  destructive source leasing preserved by this ADR.
- Accepted ADR 512 keeps discovery-run ownership distinct from worker-attempt
  ownership. Its undecided timings are not imported or selected here.

## Follow-up

- Obtain separate explicit operator selection or revision of the unresolved
  choices before production activation.
- Measure representative library sizes, directory fanout, watcher pressure,
  storage throughput, database contention, restart behavior, and aggregate hash
  cost before proposing concrete production limits or timings.
- Define the normalized run, frontier, rescan, member, and aggregate contracts and
  stored procedures in disabled, fail-closed form first. After exact values are
  approved, implement and activate injected scheduling, traversal, watchers,
  identity, operator surfaces, and observability against those contracts.
- Keep automatic watchers and schedules disabled throughout implementation until
  the complete approved validation matrix passes.
- Reconcile `MEDIA_TRANSCODING.md`, generated API contracts, deployment guidance,
  operator documentation, and status claims only in the implementation change
  that activates approved exact values.

## Task Record

- Motivation:
  - Convert the discovery scheduler and aggregate-identity choices exposed by the
    architecture audit into an explicit operator decision instead of silently
    inheriting process-local state and unexplained constants.
- Design notes:
  - The recommendation separates durable discovery-run ownership, advisory
    watcher acceleration, bounded traversal, change observations, and exact
    cryptographic aggregate identity.
  - All exact capacities, timings, encodings, and limit outcomes remain pending;
    no current value becomes a default by implication.
- Test coverage summary:
  - No runtime, schema, filesystem, watcher, API, UI, or media-conversion test was
    added or run by the ADR-only change.
  - ADR checks are `git diff --check`, `just instruction-drift`, and
    `just docs-link-check`.
  - The validation matrix and full repository gates remain mandatory after any
    exact-value approval and implementation.
- Observability updates:
  - No telemetry changes are made by this ADR.
  - A future implementation must expose bounded run transition, trigger,
    overflow, limit, identity, deduplication, and recovery outcomes. Root paths,
    source paths, member names, digests, and run ids must not be metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, historical ADR 430, approved ADRs 445-451,
    484, and 500, and accepted ADR 512 for composition boundaries.
  - `README.md`, runtime status, API contracts, operator guides, workflows,
    deployment files, and the specification remain unchanged. The ADR index and
    documentation summary expose this accepted architecture and unresolved exact-
    value hold.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must retain readable run, cursor, member,
    aggregate-version, and deduplication evidence and must hold any state an older
    runtime cannot interpret exactly.
- Dependency rationale:
  - No new dependency is required. PostgreSQL, existing filesystem descriptor
    APIs, current digest support, injected runtime boundaries, and normalized
    policy data are sufficient for the recommended design.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/revaer-ui.instructions.md`, and
    `.github/instructions/devops.instructions.md` as prospective implementation
    constraints.
  - No policy drift was found. Automation and destructive aggregate use remain
    held, no criteria are relaxed, and production activation remains blocked
    pending separate decision-specific approval and measured limit selection.
