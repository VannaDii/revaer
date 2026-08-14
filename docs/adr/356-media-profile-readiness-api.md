# Media profile readiness API

- Status: Accepted
- Date: 2026-07-30
- Context:
  - Compatibility-target support is now enforced when callers create non-dry-run media jobs through the app service, but operators still cannot ask whether a specific profile is executable before queueing work.
  - The background discovery runtime queues jobs through `MediaStore`, so scheduled and watcher paths must share the same readiness decision instead of relying on the service-only admission guard.
  - The global capability readiness endpoint only proves that a worker has a valid snapshot. It does not prove that a profile's selected compatibility target can be encoded by that worker.
- Decision:
  - Add `GET /v1/media/profiles/{media_profile_public_id}/readiness` with the evaluated profile, latest capability snapshot, boolean readiness, and a stable not-ready reason code.
  - Reuse the same compatibility-target readiness helper for direct service admission and background scheduled/watcher queueing.
  - Keep desired-target muxer, encoder-selection, subtitle, HDR, color, and source-dependent `copy` viability checks in runtime preflight until those semantics are lifted into a broader profile evaluator.
  - Rejected a new stored procedure for this slice because the decision is derived from already-normalized profile, compatibility-target, and capability read models.
- Consequences:
  - Operators can inspect profile-specific readiness before queueing jobs.
  - Background automation now fails closed when non-dry-run profiles lack a ready capability snapshot or compatible target encoder support.
  - The endpoint is still a profile compatibility readiness surface, not a proof that every desired-target and source-dependent runtime preflight will pass.
- Follow-up:
  - Extend the evaluator with source-inspection-aware preflight when a concrete source path is available.
  - Add CLI surface for profile readiness when the media command group is introduced.
  - Keep scheduler, watcher, direct job, and manual discovery admission wired to the same evaluator as it grows.

## Task Record

- Motivation:
  - Move the media service closer to production readiness by making profile execution readiness observable and by closing the background queueing bypass found during review.
- Design notes:
  - `MediaProfileReadinessResponse` includes `ready`, optional `reason`, the evaluated profile, and the latest capability snapshot.
  - Missing profiles return `404`; missing or invalid capability snapshots return `ready=false` with the existing snapshot reason codes.
  - Unsupported or missing compatibility targets return `ready=false` with the same stable codes used by job admission.
- Test coverage summary:
  - Added app-service tests for missing profile readiness and unsupported compatibility-target readiness.
  - Added a discovery-runtime watcher test proving non-dry-run watcher queueing skips work when capability readiness is missing.
  - Added API handler and E2E coverage for the new profile readiness route.
  - OpenAPI route and schema assertions were updated.
- Observability updates:
  - Background schedule and watcher readiness skips emit the existing media discovery candidate metric with `capability_not_ready`.
  - The runtime warning now includes the stable readiness reason code.
- Status-doc validation:
  - Rechecked `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, `docs/adr/index.md`, and `docs/SUMMARY.md`.
  - No root-policy contradiction was found.
- Risk & rollback plan:
  - Risk: non-dry-run background profiles that previously queued work with incomplete runtime capabilities will now skip until capabilities are refreshed.
  - Rollback by reverting this ADR, the profile readiness endpoint, and the discovery-runtime readiness guard.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No stale instruction references or contradictions were introduced.
