# Runtime shutdown event classification

- Status: Accepted
- Date: 2026-09-10
- Operator approval: 2026-09-10: "Approve **D1, D2, conditional D3, S1, and narrowly scoped F1**. Keep **S2 held** pending a defensible shutdown bound."
- Context:
  - ADR 576's CI run emitted warnings for bootstrap-requested task aborts.
    PR 194 contains related changes absent from local integration.
  - ADR 559's G1 boundary explicitly reserves observable behavior and failure
    classification for decision-specific approval. Historical ADR 454's claim
    that approval was unnecessary is not authorization.
- Decision: S1 accepted within its exact predicate, task scope and message
  contract. S2 remains held pending a defensible shutdown bound.
- Consequences:
  - S1 would change event severity for requested cancellation. S2 would observe
    a task termination that currently is not awaited after an abort request.
  - Neither change is evidence of bounded native containment or complete
    recovery, and neither may be inferred from a desire for clean CI output.
- Follow-up: Implement only S1
  and retain independent event, cancellation, panic, timeout and cleanup proof.

## Approval Resolution

The operator accepted only S1 from the proposal at commit `1d62d087`. S2 is
explicitly held: do not start awaiting an unfinished configuration watcher
after abort, add a deadline, or select a shutdown fallback under this approval.
The reviewed S1 recommendation and historical correction below remain the
scope record; approval now permits a fresh S1 implementation, not adoption of
the earlier combined S1/S2 prototype or a claim that validation passed. No
native-process, broker, recovery, fixture or Sonar criterion is changed by S1.

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

## S2 Bound Investigation: Hold Retained

Read-only source review of `2842a5ee` does not establish a defensible termination
bound. This is not measured shutdown evidence and selects no deadline or
fallback. The two-second apply SLA in `bootstrap.rs` measures a completed
configuration update; it does not time out or bound that work.

The watcher reaches database snapshot loading, policy updates, blocklist
fetch/parsing, persistence, secret reads and engine command enqueueing.
`orchestrator.rs:573` reads the entire HTTP body and then synchronously parses
it at line 659. The unique-rule limit does not bound response bytes, comments,
invalid lines or duplicate lines, so it does not bound non-yielding work.
`revaer-telemetry/src/log_stream.rs:135` also writes directly to stdout, whose
consumer may block. No existing cancellation test supplies those missing bounds.

Joining the watcher would not by itself establish downstream quiescence:
`revaer-torrent-libt/src/adapter.rs:53` waits for queue admission, while
`worker.rs:31` starts a separate worker without returning a join handle. SQLx
0.9.0 listener destruction also schedules asynchronous cleanup. Pending async
I/O alone is not evidence that Tokio abort hangs; the outstanding questions are
non-yielding work, blocking output, scheduling/destruction and the intended
downstream completion contract. S2 remains held. No arbitrary timeout, detached
fallback or process-exit policy is approved or implemented by this investigation.

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

## Restored-Tree Checkpoint

Commit `2ec8c8a18740b5d192ae1bea1ceb2f9f6cc0d9fc` restores every executable,
recipe and instruction path changed by the prototype to `783e4f54` exactly.
Subsequent changes are proposal/ledger documentation only. No prototype behavior
is active, tested as approved, pushed, or part of a release claim.

- `just ci` exited zero, including all 18 package coverage gates, full workspace
  and minimal-feature tests, Clippy, policy, audits, script coverage and release
  build. Existing requested-shutdown WARN lines remain; this is not warning-free
  output or acceptance of S1/S2.
- `just ui-e2e` exited one: 46 passed, one failed, 61 did not run. Media profile
  creation still returned 400 instead of 201 at API spec line 130. Teardown
  reported unexecuted job phases and profile readiness GET routes. No assertion
  or route-coverage requirement was changed.
- Logs are `ci-root-contract-2ec8c8a18740b5d192ae1bea1ceb2f9f6cc0d9fc.log`
  and `ui-e2e-root-contract-2ec8c8a18740b5d192ae1bea1ceb2f9f6cc0d9fc.log`
  under `/Users/vanna/Source/revaer-reviews/2026-09-10/`.
- `just docs-index instruction-drift docs-link-check` passed with 520 entries,
  1,110 checked links and zero link errors; `git diff --check` passed.
- `just sonar-compile-db js-release-coverage js-coverage-merge
  sonar-verify-inputs` passed after UI execution. Rust LCOV contains 237 sources,
  101,312 lines and 94,254 covered lines. JavaScript LCOV has 60 sources,
  5,276 lines and 3,916 covered lines, from executed API/release paths, not the
  skipped UI cases. Reports, script coverage and native inputs are retained in
  the external `runtime-fixture-proposals-coverage/` directory.
- `just sonar-scan` failed before analysis because local `SONAR_TOKEN` is
  unavailable. No new server analysis, published coverage or quality gate is
  claimed. Full-file `just --command sonar analyze secrets` on SUMMARY,
  ADR index, ADRs 564/577/578 and both generated catalogs completed without
  findings. This is secrets-only analysis; no Sonar criterion changed.
- Both stopped/completed worker worktrees were clean and removed. The owned
  PostgreSQL container was removed; TCP 5441, 7070 and 18080 were closed.
  `just clean-test-fixtures` passed; all four ignored fixture directories were
  absent and `.server_root/library` was empty. Primary user changes and older
  prunable worktree recovery metadata remain preserved.
- `gh pr edit 194 --repo VannaDii/revaer --add-reviewer '@copilot'` returned
  success, but follow-up REST/GraphQL reads still showed no request or review.
  PR 194 remained open, nondraft and at `cfed91f8`. Review submission remains
  unconfirmed; no new PR, push, merge or browser-based monitoring occurred.

## Task Record

- Motivation: Present the demonstrated shutdown warning and unobserved-join
  questions without treating a green-log goal as architectural consent.
- Design notes: S1 is accepted and locally implemented as recorded below;
  S2's lifecycle change remains held. The historical restored-tree checkpoint
  must not be confused with the later S1-only implementation.
- Test coverage summary: Fresh S1 tests and focused gates are recorded below.
  Full integrated gates and warning-free shutdown are not established by them.
- Observability updates: S1's exact event delta is locally implemented. S2's
  unfinished-watcher warning and abort/drop lifecycle remain unchanged.
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

## S1 Implementation Evidence (2026-09-10)

This fresh implementation starts from approval-record commit `826a8f8e` on
`work/media3-approved-shutdown-events-20260910`. It does not cherry-pick the
earlier combined prototype. The historical checkpoints above remain historical;
the following evidence applies only to the approved S1 change.

- Motivation and design: the immediate stop helper records whether it requested
  abort; the graceful helper marks only its existing post-timeout abort path as
  locally requested. The shared join-error logger requires both that request
  and a real cancelled `JoinError` before emitting INFO. Panics and cancellations
  without the local request retain WARN. Grace expiry retains its separate WARN.
- Observable contract: join errors retain the complete displayed `error` and
  structured `task` fields. Requested cancellation uses
  `runtime task cancelled after shutdown abort request`; other join failures use
  `runtime task join failed`. Filesystem joins use the existing immediate stop
  helper with task `fsops`; already-finished configuration joins use task
  `config_watcher` with the request flag false.
- S2 remains held. The unfinished configuration watcher still executes its
  original `abort()` and WARN, then drops the handle without awaiting it. No
  deadline, retry, native-process, broker, recovery, fixture, database, Sonar or
  GitHub criterion changes. A source-contract regression supplements the real
  join tests by asserting this exact held path remains unchanged.
- Test coverage: `bootstrap/shutdown_tests.rs` and its scoped support module
  exercise actual Tokio tasks, real cancellation and panic `JoinError`s, and
  drop witnesses. Eleven asynchronous cases cover requested abort, completed
  success, completed external cancellation, completed panic, graceful success,
  external cancellation and panic during grace, grace expiry and observed abort
  completion, panic during locally requested abort, panic after grace expiry,
  and completed configuration joins. Assertions compare exact event levels,
  messages, task/error fields, ordering, counts and cleanup completion. The
  additional source-contract case protects the held S2 path.
- Event isolation: a future-scoped event subscriber rechecks callsite interest
  per subscriber. It neither installs a global subscriber nor changes production
  filters or the panic hook. `--show-output` retains captured event records and
  deliberate panic-hook output as failure-path evidence.
- `just test-runtime-shutdown lint-runtime-shutdown` passed: 12 tests in each
  app feature configuration, zero failures or ignored tests, plus all-target and
  production panic-free Clippy passes in both configurations. Warnings are
  denied. The final log is
  `/private/tmp/revaer-s1-shutdown-final-gates-20260910.log`.
- `just policy` passed with a disposable `GNUPGHOME`, removed on exit. The first
  attempt was blocked when GPG tried to open the sandbox-protected personal
  trust database; isolation preserved the same committed-key fingerprint check
  without modifying the personal keyring or verification criteria. Logs are
  `/private/tmp/revaer-s1-shutdown-guardrails-20260910.log` and
  `/private/tmp/revaer-s1-shutdown-guardrails-isolated-20260910.log`.
- The first focused Clippy run rejected an explicit conditional panic in the
  test drop witness. Replacing it with the equivalent assertion resolved the
  finding without suppression; its diagnostic remains in
  `/private/tmp/revaer-s1-shutdown-lint-20260910.log`.
- Dependency rationale: no new dependency, manifest or lockfile change. Tests
  use existing Tokio, tracing, anyhow and standard-library facilities.
- Risk and rollback: consumers matching the old filesystem/configuration
  join-failure strings must account for the approved common message and task
  field. A local abort request still does not prove exclusive cancellation
  causality. Reverting this S1-only patch restores prior classification without
  changing shutdown lifecycle. No bounded-shutdown claim follows.
- Stale-policy check: reviewed root, Rust, devops and Sonar scoped instructions
  and the full ADR. The new focused Just recipes and corresponding devops rule
  explicitly preserve S2's hold and the integrated gates. No policy relaxation,
  unrelated historical-record rewrite, catalog or index change is included.
- Integration limits: full `just ci`, `just ui-e2e`, workspace feature-unification
  validation, strict Sonar and applicable GitHub checks remain parent-owned and
  are not claimed by these focused results. The unfinished-watcher WARN is
  intentionally still present; S1 is not a claim of warning-free full CI.
- Final local hygiene: `just fmt instruction-drift clean-test-fixtures` and
  `git diff --check` passed. No media, database, server or container was needed
  for the Tokio tests. The worktree-owned 5.1 GiB build directory was removed,
  all ignored fixture directories are absent, and raw logs remain outside the
  worktree. No Sonar source upload, push or PR operation was performed.
