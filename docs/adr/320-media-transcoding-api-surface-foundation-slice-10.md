# Media transcoding API surface foundation slice 10

- Status: Accepted
- Date: 2026-05-23
- Context:
  - Media persistence and runtime facades are in place, but no HTTP API surface existed for media profiles, jobs, or capability snapshots.
  - Existing API wiring patterns use injected facades (`indexers`) and strict RFC9457 error mapping in handlers.
- Decision:
  - Add a new API-level media facade trait (`app::media`) with typed params/responses and stable error kinds.
  - Add production media facade implementation in `revaer-app` backed by `revaer-runtime::MediaStore`.
  - Add initial v1 routes and handlers for media profile/job/capability operations with API-key protection.
- Consequences:
  - Positive outcomes: media configuration and job orchestration primitives are now reachable via authenticated HTTP endpoints.
  - Risks/trade-offs: this is a foundation API surface; deeper validation, permissions granularity, and OpenAPI enrichment still need follow-up slices.
- Follow-up:
  - Extend endpoint coverage for planning/preview, cancellation/retry, and verification artifacts.
  - Add UI integration and E2E flows for media tabs/workflows.

## Task Record

- Motivation:
  - Expose media transcoding operations through the same API architecture used by other Revaer feature domains.
- Design notes:
  - Added `crates/revaer-api/src/app/media.rs` (trait, params, responses, error type, noop impl).
  - Added `crates/revaer-app/src/media.rs` implementing the API trait via runtime media store.
  - Added media handlers and routes under `/v1/media/*`:
    - `/v1/media/profiles` (`GET`, `POST`)
    - `/v1/media/jobs` (`GET`, `POST`)
    - `/v1/media/jobs/{media_job_public_id}/phases` (`POST`)
    - `/v1/media/capabilities` (`POST`)
  - Added shared DTOs in `revaer-api-models` for media requests/responses.
- Test coverage summary:
  - Added handler tests for media profile list and write-failure mapping in `http/handlers/media.rs`.
  - Ran `cargo test -p revaer-api media:: -- --nocapture` and `just lint`.
- Observability updates:
  - No additional telemetry/events in this slice.
- Status-doc validation:
  - Updated ADR index and docs summary with this slice.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and API handler patterns.
  - No source-level lint suppressions or policy drift introduced.
- Risk & rollback plan:
  - Revert media app/API files and route wiring to roll back surface changes.
- Dependency rationale:
  - No new dependencies introduced.
