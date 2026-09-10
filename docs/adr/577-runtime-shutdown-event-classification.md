# Runtime shutdown event classification

- Status: Proposed
- Date: 2026-09-10
- Operator approval: Pending. ADR 557-559 approval does not approve this delta.
- Context:
  - ADR 576's CI run emitted warnings for bootstrap-requested task aborts.
    PR 194 contains related changes absent from local integration.
  - ADR 559's G1 boundary explicitly reserves observable behavior and failure
    classification for decision-specific approval. Historical ADR 454's claim
    that approval was unnecessary is not authorization.
- Decision: None made. Present S1 and S2 below; preserve current runtime
  behavior and warnings until the operator approves a specific delta.
- Consequences:
  - S1 would change event severity for requested cancellation. S2 would observe
    a task termination that currently is not awaited after an abort request.
  - Neither change is evidence of bounded native containment or complete
    recovery, and neither may be inferred from a desire for clean CI output.
- Follow-up: Obtain operator decisions, then implement only approved changes
  and retain independent event, cancellation, panic, timeout and cleanup proof.

## S1: Requested Cancellation Classification

Recommendation, not authorization: emit an info event for a Tokio cancelled
join only if the current stop operation requested the abort. Keep warnings for
panics, already-finished external cancellation, cancellation observed during
the graceful wait without a local abort request, and graceful deadline expiry.
Expected cancellation remains observable rather than silently discarded.

The exact scope is bootstrap's task-join handling for indexer, import, filesystem,
configuration and media runtime tasks. It does not change native command error
classification, process-broker availability or fatality, job outcomes, readiness,
deadlines, retries, database state, Sonar, or fixture diagnostics. It does not
supersede the warning on graceful deadline expiry with a lower-severity event.
Sharing the helper would also replace the filesystem/configuration-specific
join-failure message with `runtime task join failed` and a structured `task`
field. That observable message-shape change is part of S1's requested approval,
not an implicit cleanup. No external log-consumer compatibility is proven.

An explicit request flag proves this stop operation requested an abort; it does
not establish exclusive causality if another caller concurrently aborts the
same task. If the operator requires exclusive-cause attribution, reject this
proposal and design the cancellation-ownership protocol separately. No such
protocol is included or implicitly approved here.

Alternatives are retaining all existing warnings, or suppressing cancellation
events altogether. Retaining warnings preserves current observability; silent
suppression loses evidence and is not recommended. Approval of S1 applies only
to the stated cancelled-join predicate and scope. Any expansion to other error
classes, deadline events, native failures or changed ownership assumptions
requires renewed review; it is not a generic severity-reduction permission.

## S2: Await Configuration Watcher Termination

Recommendation, not authorization: after requesting configuration-watcher
cancellation, await and classify its join result through the same stop helper
already used for other bootstrap tasks. The current unfinished-watcher path
requests abort and drops the handle without observing termination.

This changes shutdown's completion semantics. Tokio abort is cooperative: if
the watcher is inside non-yielding or blocking work, awaiting it can extend
shutdown indefinitely. Existing use of the helper for other tasks is not a
bounded-termination proof for the watcher. This proposal adds no deadline and
does not approve a force-exit, detached task, or process-containment fallback.
Before acceptance, review the watcher and its callees for this availability
risk. S1 may be decided independently of S2; neither supplies missing broker
or recovery timing approval.

## Required Validation After Approval

- Capture real events from actual stop helpers, including requested cancellation,
  already-finished success, external cancellation, panics, graceful completion,
  external cancellation during grace, and grace expiry followed by observed
  abort completion. Assert retained levels, fields, counts and cleanup evidence.
- Test real Tokio JoinErrors, including a panic even when the request flag is
  true. A boolean-only unit test is insufficient.
- Isolate event capture from the workspace's global tracing initialization
  without altering production filters, installing a permissive global subscriber,
  suppressing the panic hook, or weakening the full-suite assertions.
- Exercise all-feature and minimal-feature configurations through Just recipes,
  then full `just ci`, `just ui-e2e`, strict Sonar and applicable GitHub checks.
  Validate no normal-shutdown warning disappears by also hiding a real failure.

## Approval Boundary Correction

Local commit `27770554` prematurely classified this as a nonarchitectural task
and changed runtime code and a focused recipe before the G1 restriction was
verified. That classification was incorrect. No operator consent is inferred
from the local commit, PR 194, historical ADR 454, or task continuation.

On discovering the explicit boundary, the parent stopped implementation and
the test worker, restored its own runtime/recipe/instruction edits to the prior
executable state, and converted this record to Proposed. The prototype remains
only in local history for audit; it must not be published, activated, or extended
without approval. This correction is not a claim that the earlier action was
authorized. No UI assertion, migration, Sonar rule or remote setting was changed.
The test worker had begun a focused compilation before interruption; it reported
no completed test result. Its new test files, module declaration, local build
output and worktree were removed. No implementation or behavioral pass follows
from that stopped attempt.

## Task Record

- Motivation: Present the demonstrated shutdown warning and unobserved-join
  questions without treating a green-log goal as architectural consent.
- Design notes: S1 and S2 distinguish event classification from lifecycle
  completion. No architecture is selected; the prior runtime behavior remains.
- Test coverage summary: Proposal only; no new implementation test result or
  warning-free runtime result is claimed. Documentation and restored-tree gate
  evidence must be attached separately.
- Observability updates: None activated. S1's exact event delta is proposed;
  current warning behavior is retained pending approval.
- Status-doc validation: Update ADR status, indexes and generated catalog to
  reflect the hold. Do not copy ADR 454's incorrect approval inference.
- Risk & rollback plan: Severity changes can conceal failure; joining a
  cooperative task can delay shutdown. Keep these risks explicit and preserve
  original behavior until approved. The prematurely implemented local delta
  was removed; the primary user's changes remain untouched.
- Dependency rationale: No dependency added. Proposed tests would use existing
  Tokio, tracing and test support; no new dependency is authorized here.
- Stale-policy check: Reviewed root, Rust/devops instructions, ADR 559's G1
  boundary, current bootstrap, PR 194's diff and ADR 576 evidence. Corrected this
  task's Recorded/nonarchitectural assertion to Proposed/Pending, and removed
  active instructions/recipe entries for the unapproved runtime change.
