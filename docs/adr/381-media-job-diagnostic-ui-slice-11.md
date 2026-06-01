# Media job diagnostic UI slice 11

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Media job operations, violations, and plan reasons are now exposed through authenticated API endpoints.
  - The media plan requires operators to inspect job timelines, selected plans, rejected plans, compliance issues, and final disposition from the UI.
- Decision:
  - Add media UI helpers for nested job diagnostic routes.
  - Fetch operation, violation, and plan-reason rows for refreshed jobs.
  - Store diagnostic rows by media job id in the media feature state.
  - Render expandable recent-job diagnostic details with counts, selected reason, operations, violations, and plan reasons.
- Consequences:
  - Positive outcomes:
    - Operators can inspect persisted job diagnostics from the media page without leaving the UI.
    - The UI now consumes the same row-level diagnostic APIs used by automation.
  - Risks or trade-offs:
    - Diagnostics are fetched for the refreshed job set instead of being lazily loaded per expanded row; a larger job list may need pagination or on-demand loading later.
- Follow-up:
  - Add dedicated job detail selection and report grouping once verification checks, artifacts, and compact audits are persisted.
  - Add E2E assertions around populated diagnostic rows when fixture-backed media jobs are available.

## Task Record

- Motivation:
  - Continue slice 11 by surfacing persisted media job diagnostics in the existing media UI.
- Design notes:
  - Kept route construction and diagnostic summaries as pure helpers so native unit tests can cover the UI contract.
  - Kept diagnostic state in the media feature slice and keyed it by `media_job_public_id`.
  - Preserved the existing refresh model while making the diagnostic rows visible in expandable recent-job details.
- Test coverage summary:
  - Added failing native UI helper tests for nested job diagnostic paths and diagnostic summary text, then implemented the helpers and state.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui media_job_diagnostic -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo check -p revaer-ui --target wasm32-unknown-unknown --all-features`.
- Observability updates:
  - None. This is UI read-side exposure for already persisted diagnostic rows.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: fetching diagnostics for many jobs may increase media page refresh latency. Roll back by removing diagnostic fetching/rendering, helper tests, state fields, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
