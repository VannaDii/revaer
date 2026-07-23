# Lazy Media Job Diagnostics UI

- Status: Accepted
- Date: 2026-08-12
- Context:
  - The media page fetched jobs once per profile and then fetched six diagnostic collections for each displayed job during every snapshot refresh.
  - The review requires bounded, constant-request list loading and per-job diagnostic isolation through aggregate API contracts.
- Decision:
  - Fetch the first bounded page from `GET /v1/media/jobs/recent` without profile fanout.
  - Fetch all six diagnostic collections from `GET /v1/media/jobs/{id}/diagnostics` only when the operator opens that job's disclosure.
  - Track loading, loaded, and failed state per job. Associate each in-flight load with a unique request identifier and accept its result only while that request remains active and the disclosure remains open.
  - Cache successful diagnostics while the job remains in the recent page. Retry a failed request when its disclosure is closed and opened again.
  - Alternatives considered: concurrent calls to the six legacy routes still amplify each disclosure into six requests; eager batch loading still spends work on diagnostics the operator did not request.
- Consequences:
  - Initial media-page refresh uses one bounded jobs query regardless of profile count.
  - One job's diagnostic failure is rendered inline and cannot block or replace another job's state.
  - Closing a disclosure invalidates an in-flight response but does not attempt transport-level abort; the completed response is discarded without mutating UI state.
- Follow-up:
  - Keep the UI DTOs aligned with the server implementation of both aggregate routes.
  - Exercise pagination controls separately if the media page expands beyond the bounded recent-job overview.

## Task Record

- Motivation:
  - Resolve the two outstanding PR #87 review threads concerning request fanout and eager diagnostics loading.
- Design notes:
  - The recent-job request uses a fixed limit of ten for the existing overview.
  - Cursor values are percent-encoded and the optional profile filter remains available in the transport helper.
  - Stale-response rejection compares an opaque UUID request identifier and also requires that the corresponding disclosure remain open.
- Test coverage summary:
  - Added unit coverage for recent-job and aggregate-diagnostics paths, including cursor encoding and optional filters.
  - Added unit coverage for active, stale, and canceled diagnostic request identification.
  - Passed 133 `revaer-ui` library tests, its native binary test, strict focused Clippy, full workspace `just lint`, `just fmt`, instruction drift, and `wasm32-unknown-unknown` compilation with all features.
  - Attempted `just ci` and Node 24.14.1 `just ui-e2e`; both stopped at database setup because Docker was unavailable and no PostgreSQL endpoint was listening on `localhost:5432`.
  - The UI dependency install also reported two pre-existing high-severity npm audit findings; this task did not alter the dependency graph.
- Observability updates:
  - Per-job loading and failure states are visible within the opened disclosure through stable test selectors; no new logging, metrics, or tracing was required.
- Status-doc validation:
  - Rechecked the ADR index and mdBook summary and added this decision to both. No README, roadmap, or operator-guide behavior changed.
- Risk & rollback plan:
  - The UI requires the two new server routes to be present in the stacked backend deliverable. Roll back this commit together with those route contracts if integration fails.
- Dependency rationale:
  - No dependencies were added. Existing `uuid` and `urlencoding` support request identity and query encoding.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-ui.instructions.md`.
  - No instruction drift or contradictory stale references were found.
