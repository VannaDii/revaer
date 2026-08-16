# Media execution and worker runtime boundaries

- Status: Accepted
- Date: 2026-08-15
- Operator approval: Recommendation approved wholesale by the operator on
  2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Operator reviews on PRs 78, 81, and 138 require command compilation, process
    supervision, preflight, verification, replacement, persistence, audit,
    retention, workspace accounting, and sidecar publication to have cohesive,
    injected boundaries.
  - The integrated leaf currently has 7,266 lines in
    `revaer-media-runtime/src/execute/mod.rs`, 4,810 lines in
    `revaer-media-runtime/src/jobs/mod.rs`, and 14,150 lines in
    `revaer-app/src/media_job_runtime.rs`. Tests account for substantial portions,
    but production responsibilities also remain mixed.
  - Public media behavior, stored-procedure contracts, claim fencing, attempt
    isolation, and crash consistency must remain unchanged while files move.
- Decision:
  - Recommended option: decompose within the existing two crates. Do not create
    new crates or change public API ownership for this refactor.
  - In `revaer-media-runtime`, make `execute/mod.rs` a small facade over model and
    error types, execution control, command runner, FFmpeg argument compilation,
    container metadata and chapter compilation, stream constraints, desired-graph
    materialization, filesystem steps, and sequence execution. Make `jobs/mod.rs`
    a facade over job models, managed paths, planning, preflight policy and
    reporting, workspace capacity, execution-step assembly, and compact audit.
  - In `revaer-app`, make `media_job_runtime/mod.rs` the injected coordinator over
    claim/fencing, preflight, execution supervision, verification families,
    replacement transaction, persistence/audit, and terminal cleanup. Move audio
    measurement, attachment digesting, and capacity probing into explicit adapter
    modules owned by the runtime component bundle.
  - Introduce a trait only where the coordinator must substitute a side effect in
    tests or bootstrap already injects a collaborator. Pure transforms remain
    functions; module count is not a reason to add indirection.
  - Keep tests beside the contract they exercise in test-only modules or files,
    including crash-point, cancellation, attempt-isolation, path-identity,
    verification-family, and replacement-transaction tests.
  - Alternative considered: move tests only. Rejected because production
    responsibilities and dependency direction would remain mixed.
  - Alternative considered: create one crate per execution concern. Rejected as
    premature dependency and public-surface expansion.
- Consequences:
  - The worker coordinator becomes reviewable as a state transition flow while
    adapters and pure verification logic can be tested independently.
  - Moves across three large files have high merge-conflict risk and therefore
    must be staged before feature changes that touch the same code.
  - Narrow facades preserve current callers and permit incremental stack PRs.
- Follow-up:
  - First lock characterization tests and public exports; then move pure models
    and functions; then extract injected side-effect adapters; finally reduce the
    coordinator to state transitions.
  - Keep each production file near the repository's 300-400 non-test LOC target;
    document any cohesive exception rather than suppressing lint or Sonar.
  - Run affected-crate tests after every move and full CI, UI E2E, media fixtures,
    API export, images, and Sonar on the integrated boundary.

## Task Record

- Motivation:
  - Resolve three overlapping review requirements through one explicit boundary
    decision instead of independent, conflicting file splits.
- Design notes:
  - This proposal preserves existing crate, database, and API ownership. It
    separates orchestration from pure policy and side effects without changing
    media semantics.
- Test coverage summary:
  - Proposal only. The implementation gate is the existing characterization and
    failure-point suite plus all required integrated checks.
- Observability updates:
  - Preserve phase, audit, verification, and terminal event names. Module moves
    must not add duplicate logging as errors propagate.
- Status-doc validation:
  - Updated the ADR index and documentation summary. Capability status is unchanged.
- Risk & rollback plan:
  - Stage moves as behavior-preserving PRs and revert the current move if a facade
    contract changes unexpectedly. Do not combine rollback with lost fencing,
    verification, or audit behavior.
- Dependency rationale:
  - No dependency or new crate is proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust/FFI instructions, ADRs 318, 427, 429, 439, and the
    current PR 78, 81, and 138 review summaries.
  - Drift found: closeout records describe repaired behavior but do not resolve
    the still-active production-module boundary requirement.
