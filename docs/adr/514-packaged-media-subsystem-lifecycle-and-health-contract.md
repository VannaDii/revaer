# Packaged media subsystem lifecycle and health contract

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

- The packaged runtime defaults the media workspace to
  `/data/media-workspaces`, but the image creates only `/data` and a mounted data
  volume hides image-layer directories. The application requires an absolute
  root without one explicit package-owned provisioning and durability contract.
- Helm defaults `/data` to `emptyDir` while documentation says destructive work
  requires persistence. A writable path alone cannot prove that attempt
  workspaces, checkpoints, and replacement evidence survive process or pod
  replacement.
- Media capability discovery can fail while health and remediation APIs must
  remain available. Global application readiness, media-subsystem readiness,
  profile advisory readiness, and source-bound execution authority are distinct
  states that the current health contract does not model coherently.
- Discovery, job, and retention tasks are spawned independently and observed only
  during shutdown. An unexpected task exit can leave a partially functioning
  process that still reports database readiness.
- Signal handling currently lets API graceful shutdown complete before media
  shutdown starts, then grants each media task a separate 30-second wait. This
  can exceed the packaged 45-second pod grace and does not establish one bounded
  drain contract.
- The package must preserve approved dry-run isolation, root ownership,
  checkpoint recovery, process supervision, and degraded-capability behavior
  without making a profile-only readiness assessment an execution guarantee.

## Options

1. **Retain independent tasks and dependency-only probes.** Keep current startup,
   shutdown, and task ownership. This is small but can advertise readiness after
   worker loss and can exceed termination grace.
2. **Restart each media task independently in-process.** Add per-task restart and
   readiness state. This improves availability but can restart claims or
   retention without rerunning the common root recovery barrier.
3. **Use one supervised media subsystem and whole-process restart for invariant
   failures.** Model lifecycle and degraded states centrally, recover roots before
   work, drain API and media concurrently, and let the deployment supervisor
   restart the complete process after an unexpected child failure.

## Recommendation

- Adopt option 3.
- The package or bootstrap boundary performs one idempotent provisioning and
  validation step for every configured allowlisted workspace root before media
  workers start. Job, dry-run, and retention paths do not create or repair the
  root implicitly.
- Root validation proves absolute managed placement, ownership and mode,
  filesystem identity, overlap rules, capacity access, and the durability and
  sole-writer attestations required by approved ADRs 447 and 512.
- Destructive readiness requires an explicitly configured durable workspace
  volume. Helm's default `emptyDir` and any unattested writable root remain
  dry-run or disposable-evaluation only. The runtime must not infer durability
  from successful directory creation or filesystem writes.
- One media-subsystem supervisor owns lifecycle states `starting`, `recovering`,
  `ready`, `degraded`, `draining`, `failed`, and `stopped`, plus the discovery,
  job, and retention task handles and the accepted ADR 512 root-recovery barrier.
- Startup makes the API and non-media runtimes available after control-plane and
  database bootstrap. The media subsystem remains `recovering` until root
  provisioning, capability state, worker startup, and required reconciliation
  are classified. Claims and retention remain disabled until their applicable
  barriers open.
- A capability refresh failure is a handled media degradation. It blocks media
  planning or execution that requires a valid capability snapshot but does not
  terminate the process or block health and remediation APIs. A later successful
  refresh can move the media subsystem toward readiness without restarting the
  process.
- `/health/live` remains dependency-independent process liveness. It returns
  success while the process can coordinate an orderly exit, including while
  draining.
- `/health/ready` requires database-backed control-plane readiness, completed API
  bootstrap, and a lifecycle state other than draining or failed. It does not
  require media capabilities, root recovery, or destructive eligibility, so
  remediation remains reachable during media degradation.
- `/health/full` reports the bounded media-subsystem lifecycle state, worker-task
  state, recovery state, workspace durability and eligibility, and capability
  degradation. It must not claim that media `ready` proves one profile and source
  can execute; that decision remains source-bound under approved ADR 442 and the
  separate accepted ADR 505 contract.
- Recoverable database, capability, or root-recovery errors keep the supervisor
  alive in `recovering` or `degraded` and retry through one injected bounded
  backoff policy. An unexpected child return, task panic, or invariant failure
  transitions to `failed`, initiates bounded whole-process shutdown, and exits
  nonzero. Individual media tasks are not restarted independently.
- SIGTERM and SIGINT atomically transition the process to `draining`, make global
  readiness fail, stop new claims, and start API drain plus media pause
  concurrently. Discovery and retention stop at safe boundaries; active jobs use
  accepted ADR 512's resumable pause rather than terminal cancellation.
- One aggregate application deadline covers API drain and every runtime task.
  Replacement finalization may complete within that deadline; unfinished work
  must leave durable recovery evidence. At the deadline, remaining tasks are
  aborted and the process exits with an unclean-shutdown outcome rather than
  reporting successful cleanup.

### Supported Operational Values

- Preserve `/data/media-workspaces` as the packaged default workspace path below
  the `/data` volume.
- Preserve `dataPersistence.enabled: false` as a safe dry-run and disposable
  evaluation default; it does not authorize destructive readiness.
- Preserve one 30-second aggregate application shutdown budget instead of three
  sequential per-task budgets.
- Preserve the Helm default pod termination grace of 45 seconds and the schema
  requirement that it remain strictly greater than the application budget.
- Preserve SIGTERM and SIGINT as Unix shutdown inputs and the existing
  dependency-independent liveness and database-backed global readiness routes.
- ADR 501's approved five-second native-process escalation remains inside, not in
  addition to, the 30-second aggregate application budget.

### Explicitly Undecided Timings

- The recoverable startup and runtime retry sequence may use the replay's
  one-second delay as its initial delay, but exponential steps, jitter, maximum
  delay, reset window, and retry budget remain undecided pending failure and
  database-load evidence.
- No in-process task-restart delay is selected because unexpected task loss is a
  whole-process failure. Deployment-supervisor restart and backoff timing remain
  outside this application contract.
- Existing startup, readiness, and liveness probe periods and thresholds are not
  changed or newly authorized by this accepted architecture decision. Any tuning
  requires deployment evidence and must preserve the 30-second/45-second
  shutdown relationship.

## Consequences

- Operators retain API and remediation access when media tools, roots, or
  recovery are degraded without receiving a false execution-readiness claim.
- Destructive work cannot silently depend on an ephemeral workspace that loses
  checkpoints and replacement evidence on pod replacement.
- Unexpected worker loss cannot leave a partially active process advertising
  healthy media operation; the complete process restarts through one recovery
  path.
- API and worker drains share one deadline and fit within the packaged pod grace,
  but long-lived requests may be closed when the deadline expires.
- Health state requires one shared bootstrap-owned registry available before the
  startup capability refresh and worker tasks begin.
- Whole-process restart sacrifices isolated child availability in exchange for
  deterministic root recovery and a smaller failure model.

## Implementation Boundary

- This accepted ADR authorizes only package-owned root provisioning and durability
  classification, the media-subsystem lifecycle states, centralized child-task
  supervision, global versus media health semantics, handled capability
  degradation, whole-process failure behavior, concurrent signal drain, and
  supported shutdown values described above.
- Approved ADRs 444, 447, 448, 452, 483, 500, and 501 remain binding. Accepted
  ADRs 512 and 513 own worker recovery and retention ordering; this ADR consumes
  their accepted contracts without redefining their unresolved timings.
- Accepted ADR 505 owns profile advisory readiness, universal impossibility, queue
  admission, and source-bound preflight. This ADR does not supersede or redefine
  ADR 505 and does not make subsystem readiness an execution grant.
- Accepted ADRs 504 and 506 remain outside this lifecycle decision. This ADR does
  not authorize attempt-read API changes or manual-execution command
  semantics.
- This ADR does not authorize independent child-task restart, a new process
  manager dependency, media-capability fallback, fabricated snapshots, media
  gating of remediation APIs, destructive work on ephemeral or unattested roots,
  a longer native-process grace, or a weaker shutdown deadline.
- Environment reads and concrete adapters remain in bootstrap or package wiring;
  runtime logic receives the lifecycle, health, clock, backoff, root, and process
  collaborators from callers.
- Runtime, API, Docker, Helm, schema, generated-contract, and operator-guide
  changes must remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only; this record changes no
  bootstrap, health, task, process, Docker, Helm, or runtime behavior.

| Scenario | Required implementation result |
| --- | --- |
| Media tools are missing or malformed at startup | Process is live and globally ready after control-plane bootstrap; media is degraded; execution fails closed; refresh can recover. |
| Workspace is missing, misowned, overlapping, ephemeral, or durably attested | Provisioning or eligibility fails closed, disposable storage remains dry-run-only, and only valid durable roots become destructive-ready. |
| Root replacement or outbox recovery is incomplete | API remediation remains available while claims and retention for that root remain blocked. |
| Discovery, job, or retention task returns or panics unexpectedly | Supervisor records failure, readiness fails, the complete process exits nonzero, and the deployment restart reruns recovery. |
| Recoverable database or capability failure | Supervisor remains alive in recovering or degraded state and retries through the injected bounded policy without a busy loop. |
| SIGTERM or SIGINT while idle or during execution, verification, or replacement | Readiness fails immediately, no new claim starts, API and media drain concurrently, and active work pauses or leaves recoverable evidence. |
| Long API connection and active native process exceed the application budget | Native escalation stays within 30 seconds, remaining tasks abort at the aggregate deadline, and the 45-second pod grace remains available. |
| Media subsystem is ready but one profile/source is impossible | Global and subsystem health do not authorize execution; source-bound preflight remains authoritative. |

- During implementation, add packaged-image and Helm integration tests for root
  provisioning, empty and persistent volumes, ownership failures, capability
  repair, root-recovery blocking, task failure, nonzero restart, signal handling,
  active native processes, API drains, and aggregate deadline enforcement.
- Add health contract tests for every lifecycle transition and for the separation
  of liveness, global readiness, subsystem status, profile advisory readiness,
  and source-bound execution authority.
- An accepted implementation is not complete until focused lifecycle tests,
  `just ci`, and `just ui-e2e` pass.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- Relevant replay commits are PR 190 `1ff80944`, PR 191 `48dfa65d`, and PR 194
  `4ebd3eca`, with cooperative worker shutdown behavior from PR 108 `b44b5ceb`
  and PR 109 `d6a8d0ea`.
- Historical ADRs 440 and 441 describe the packaged workspace, probe,
  capability-degradation, signal, and grace behavior. Their `Accepted` labels and
  implementations are not decision-specific operator approval.
- The replay currently uses `/data/media-workspaces`, `dataPersistence.enabled:
  false`, a 30-second per-task application wait, and a 45-second pod grace. It
  starts media tasks independently and checks their join results only during
  shutdown.
- Explicitly approved ADRs 444, 447, 448, 452, 483, 500, and 501 constrain dry-run
  mutation, root ownership, resumability, dependency injection, crate ownership,
  workspace recovery, and native-process supervision.
- Accepted ADR 505 was inspected as a separate profile and admission decision. Its
  scope remains separate and does not make health or lifecycle state an execution
  grant.

## Follow-up

- Implement this ADR with accepted ADRs 512 and 513 so package startup,
  recovery, retention, and shutdown share one state model and barrier ordering.
- Resolve retry-backoff timing from measured database and deployment failure
  evidence through a separately approved follow-up before implementation requires
  concrete defaults.
- Implement the lifecycle registry and supervisor before changing
  health routes, root provisioning, Helm eligibility, or shutdown orchestration.
- Reconcile `MEDIA_TRANSCODING.md`, Docker and Helm documentation, health and
  operator APIs, generated contracts, and deployment runbooks in the accepted
  implementation change.

## Task Record

- Motivation:
  - Convert the PR 190-194 packaged lifecycle behavior and its remaining
    supervision, persistence, readiness, and shutdown gaps into one explicit
    operator decision.
- Design notes:
  - Global control-plane availability is intentionally separate from media
    subsystem health and from source-bound execution authority.
  - The complete process, rather than one child task, is the restart unit because
    startup recovery and retention barriers are subsystem-wide invariants.
  - The 30-second application deadline is aggregate and leaves 15 seconds inside
    the current packaged pod grace for process and platform termination.
- Test coverage summary:
  - The ADR-only change added no bootstrap, health, runtime, Docker, Helm,
    process, or integration tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this ADR-only change.
  - The validation matrix and full repository gates remain mandatory after any
    implementation.
- Observability updates:
  - No telemetry changes are made by this ADR.
  - A future implementation must publish bounded lifecycle, worker-exit,
    recovery, capability, root-eligibility, drain, deadline, and forced-abort
    outcomes without source, path, job, attempt, profile, or pod identifiers as
    metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, historical ADRs 440 and 441, PR 190-194
    runtime and package behavior, approved ADRs 444, 447, 448, 452, 483, 500, and
    501, and accepted ADR 505. This ADR is accepted but does not claim the contract
    is implemented.
  - `README.md`, roadmap/status documents, runtime behavior, API contracts,
    Docker and Helm behavior, and operator guides are unchanged; the ADR index and
    documentation summary expose this accepted but unimplemented decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must retain a package configuration that
    fails destructive work closed when durability or root ownership cannot be
    proven and must preserve recovery evidence across process replacement.
- Dependency rationale:
  - No new dependency is required. Existing Tokio task supervision, watch
    channels, health state, package primitives, deployment supervision, and
    injected process boundaries are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md` as prospective implementation
    constraints.
  - No policy drift was found. This ADR does not weaken readiness, resource,
    release, persistence, or quality-gate requirements.
