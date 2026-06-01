# Media artifact and compact audit UI slice 11

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media diagnostics UI already renders operations, violations, plan reasons, and verification checks for recent jobs.
  - Artifact references and compact audit facts now have API endpoints, but the UI did not fetch or display them.
- Decision:
  - Extend media job diagnostics state with artifact references and compact audit facts.
  - Fetch `/artifacts` and `/compact-audits` for each recent media job alongside the existing diagnostic collections.
  - Update diagnostics summaries and render separate artifact and audit fact sections.
- Consequences:
  - Positive outcomes:
    - Operators can see retained artifact references and compact audit facts in job details.
    - UI pure-helper tests cover the new nested route paths and summary counts.
  - Risks or trade-offs:
    - The recent-jobs diagnostics refresh now performs two more per-job API calls until a future aggregated job-details endpoint exists.
- Follow-up:
  - Consider an aggregated media job diagnostics endpoint if per-job collection fetches become expensive.
  - Add browser-level coverage once the media page has stable fixture-backed job diagnostics.

## Task Record

- Motivation:
  - Complete the API-to-UI surface for artifact references and compact audit facts.
- Design notes:
  - Reused the existing `MediaJobDiagnostics` flow so refresh behavior stays consistent for every diagnostic row family.
  - Kept rendering compact and scannable in the existing recent-jobs details grid.
- Test coverage summary:
  - Added failing UI logic tests for artifact/audit routes and summary counts before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui media_job_diagnostic_paths_use_nested_job_routes -- --test-threads=1`.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui summarize_media_job_diagnostics_counts_rows_and_selected_reasons -- --test-threads=1`.
- Observability updates:
  - None. This displays persisted diagnostic data already exposed by the API.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: additional diagnostics fetches may slow refresh for large recent-job lists. Roll back by removing the new state fields, fetch calls, render sections, tests, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
