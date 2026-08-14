# Clean runtime shutdown observability

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Not required; this is a corrective observability change with no architecture choice.
- Context:
  - Bootstrap deliberately aborts indexer, import, filesystem, and configuration tasks during orderly shutdown when they have not already completed.
  - Tokio reports those requested aborts as cancelled join errors.
  - The current bootstrap logs every cancelled join as a warning, so successful coverage and E2E runs emit repeated false operational alarms.
  - Panics, unexpected cancellation, and graceful-shutdown timeouts must remain visible as warnings.
  - The coverage job reruns the media UI flow after Rust coverage. Its retry data used timestamp-only identifiers and asserted a transient loading state, making the gate vulnerable to retained retry data and fast refresh completion.
- Decision:
  - Record a join cancellation at info level only when bootstrap itself requested that cancellation.
  - Continue warning for every other join error.
  - Await filesystem and configuration task handles after requesting cancellation so shutdown observes their terminal state.
  - Keep the existing graceful-shutdown timeout warning and classify the cancellation caused by the subsequent abort as expected.
  - Give every media UI retry a collision-resistant identifier and assert only stable post-save state.
- Consequences:
  - Normal shutdown no longer emits misleading warnings.
  - Unexpected runtime exits and task panics retain warning-level visibility.
  - Runtime lifecycle and cancellation timing remain unchanged.
- Follow-up:
  - Run focused bootstrap tests, `just ci`, and `just ui-e2e`.
  - Confirm the remote coverage log no longer contains the corrected shutdown warnings.

## Task Record

- Motivation:
  - Strict CI and operational review require warning output to represent actionable failures rather than expected control flow.
- Design notes:
  - Classification depends on both an explicit local cancellation request and Tokio's cancelled join-error classification.
  - No error is silently discarded; expected cancellation is logged once at info level.
- Test coverage summary:
  - Adds a regression test proving an explicitly aborted Tokio task reports cancellation.
  - Makes the media UI flow retry-isolated and removes timing-dependent assertions.
  - Full local and remote verification is required before handoff.
- Observability updates:
  - Requested shutdown cancellation changes from warning to info.
  - Unexpected join failures and shutdown timeouts remain warnings.
- Status-doc validation:
  - The ADR index and documentation summary are updated. Product status documentation is unaffected because runtime behavior does not change.
- Risk & rollback plan:
  - Risk is misclassifying an unexpected cancellation after a requested abort. The explicit request flag bounds the lower severity to bootstrap-owned cancellation only.
  - Revert this commit to restore prior logging if cancellation classification regresses.
- Dependency rationale:
  - No dependency is added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and the bootstrap/runtime requirements in `MEDIA_TRANSCODING.md`.
  - No policy drift was found and no criterion is relaxed.
