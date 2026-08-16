# Media operator surface boundary

- Status: Accepted
- Date: 2026-08-15
- Operator approval: Recommendation approved wholesale by the operator on
  2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The PR 87 operator review requires complete create, preview, run, retry, and
    retention workflows, including per-path discovery associations, desired
    target stream graphs, and workspace, backup, output, and verification controls.
  - The integrated backend already exposes richer target, retention, watcher, job
    action, and diagnostic routes than the UI client consumes. The UI currently
    has a 195-line API file and a 2,069-line view that mixes fetching, mutation,
    parsing, state coordination, and rendering.
  - Missing controls must be backed by normalized stored-procedure data and typed
    APIs; raw YAML is an import/export path, not the primary operator surface.
- Decision:
  - Recommended option: define the operator surface outside-in as seven bounded
    workflows: readiness and collections; profile and discovery association;
    desired target graph authoring and pinning; policy and verification;
    preview plus dry-run or replace; job control and diagnostics; and retention
    plus workspace, backup, and output policy.
  - Consume existing backend collection and action routes first. Add typed,
    normalized child resources only where the current API cannot represent a
    required per-path association or operational policy; do not overload profile
    booleans or introduce JSONB state.
  - Split the UI API by resource (`profiles`, `discovery`, `targets`, `policies`,
    `jobs`, and `retention`). Keep request construction and response decoding
    there, pure validation and display transforms in `logic`, workflow state and
    stale-request fencing in `state` or controllers, and focused rendering in
    components. `view.rs` becomes a route-level shell using tabs for workflows.
  - Preserve bounded pagination and lazy job diagnostics. Mutations return updated
    typed resources and refresh only their owned collection.
  - Alternative considered: split the existing view but expose only current
    controls. Rejected because it does not satisfy the operator workflows.
  - Alternative considered: use YAML as the complete advanced editor. Rejected
    because it bypasses ergonomic validation and does not provide inspectable
    resource workflows.
  - Alternative considered: place all controls on one page. Rejected because it
    preserves the current mixed ownership and scales poorly.
- Consequences:
  - Operators can configure and validate the complete service without falling
    back to undocumented API calls.
  - New normalized API contracts may be required for per-path discovery and
    operational path policies; those contracts increase implementation scope.
  - Resource ownership and stale-request fencing reduce accidental cross-panel
    state replacement.
- Follow-up:
  - Inventory each required workflow against existing OpenAPI routes and write a
    gap table before adding endpoints.
  - Implement backend and typed-model gaps before their UI controls, then add
    Playwright flows for create, preview, dry-run, replace confirmation, cancel,
    retry, diagnostics, and retention.
  - Verify responsive desktop/mobile layouts, keyboard and screen-reader access,
    loading/empty/error states, and no overlapping text or controls.

## Task Record

- Motivation:
  - Resolve the remaining PR 87 operator-surface review with an explicit product
    and component boundary before changing APIs or persistence.
- Design notes:
  - The recommendation reuses existing routes where possible and keeps new state
    normalized and stored-procedure-backed.
- Test coverage summary:
  - Proposal only. Implementation requires model/handler contract tests, OpenAPI
    export validation, UI unit tests, and complete Playwright workflows.
- Observability updates:
  - Preserve current API error mapping and job audit events. Add no client-only
    success claim until the server returns the updated resource.
- Status-doc validation:
  - Updated the ADR index and documentation summary. Current UI capability claims
    remain unchanged pending implementation.
- Risk & rollback plan:
  - Ship one workflow per stack PR behind existing authenticated routes. Revert a
    workflow vertically if it regresses; do not leave an exposed control without
    its backend contract or verification path.
- Dependency rationale:
  - No dependency is proposed; use the existing Yew and typed API stack.
- Stale-policy check:
  - Reviewed `AGENTS.md`, UI/data instructions, ADRs 318, 430, 431, and 439, the
    current OpenAPI routes, and the PR 87 review summary.
  - Drift found: current backend capability exceeds the UI client, while several
    requested operational controls still lack an explicit normalized UI contract.
