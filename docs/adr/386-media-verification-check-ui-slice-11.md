# Media verification check UI slice 11

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Verification checks are now available through the media job API.
  - The media plan requires job details to expose verification results alongside selected plans, rejected plans, operations, and violations.
- Decision:
  - Add the verification-check nested API path helper to the media UI feature.
  - Fetch verification checks as part of the media job diagnostics refresh.
  - Store checks in `MediaJobDiagnostics`, include them in the diagnostic summary, and render them in recent-job expandable details.
- Consequences:
  - Positive outcomes:
    - Operators can inspect persisted verification facts from the media page.
    - The media job diagnostic UI now consumes operations, violations, plan reasons, and verification checks through public endpoints.
  - Risks or trade-offs:
    - Diagnostics are still fetched as part of the page refresh; larger job sets may need on-demand detail loading.
- Follow-up:
  - Add dedicated job selection and grouped verification report views once artifact and compact-audit rows exist.
  - Add E2E assertions around populated verification check rows when media job fixtures are available.

## Task Record

- Motivation:
  - Continue slice 11 by surfacing persisted verification outcomes in the media UI.
- Design notes:
  - Kept route construction and summary text in pure helpers for native tests.
  - Extended existing diagnostic state rather than introducing a separate UI store shape for verification checks.
- Test coverage summary:
  - Added failing native UI helper expectations for verification-check path and summary count, then implemented state, API fetching, and rendering.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui media_job_diagnostic -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo check -p revaer-ui --target wasm32-unknown-unknown --all-features`.
- Observability updates:
  - None. This is UI read-side exposure for already persisted verification facts.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: eager diagnostic fetching may grow refresh latency as job volume grows. Roll back by removing verification-check fetching/rendering, helper expectations, state fields, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
