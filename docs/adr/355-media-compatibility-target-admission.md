# Media compatibility target admission

- Status: Accepted
- Date: 2026-07-30
- Context:
  - Non-dry-run media discovery and direct job creation checked only for a globally valid capability snapshot before queueing work.
  - Profiles can also pin a compatibility target that names required output video and audio codecs.
  - A globally valid snapshot is insufficient when the pinned compatibility target requires an encoder that the current runtime cannot provide.
- Decision:
  - Add a profile execution readiness gate in the app service before non-dry-run discovery or direct job admission.
  - Keep the existing global snapshot validity checks, then verify that the latest persisted compatibility-target version has encode support for both configured codecs.
  - Treat `copy` as always satisfiable by the compatibility-target gate because source-dependent stream-copy viability remains a worker planning concern.
  - Return explicit invalid-request codes for missing compatibility target rows and unsupported target codecs.
- Consequences:
  - Non-dry-run jobs fail before queueing when the profile asks for a compatibility target that the current capability snapshot cannot encode.
  - Dry-run-only profiles keep their existing admission behavior.
  - Desired-target stream, container, subtitle, HDR, color, profile, and level readiness remain enforced by worker planning until the next profile-readiness API slice moves those checks earlier.
- Follow-up:
  - Add a profile-scoped readiness API that reports compatibility target and desired target readiness before operators queue work.
  - Share the richer readiness evaluator with direct job creation, discovery enqueue, and worker preflight.
  - Decide whether stored procedures should also reject readiness failures atomically or whether app admission plus worker preflight is the accepted concurrency boundary.

## Task Record

- Motivation:
  - Close the profile compatibility gap found while splitting PR 31 media work into reviewable stacked slices.
- Design notes:
  - `MediaService::ensure_profile_ready_for_execution` reads the latest capability snapshot once, preserves fail-closed global snapshot validation, and only lists compatibility targets when the profile references one.
  - `ensure_profile_compatibility_target_readiness` is a pure helper so codec support behavior has fast unit coverage outside the database harness.
  - The service selects the latest version for a matching compatibility target key, mirroring existing catalogue list semantics.
- Test coverage summary:
  - Added unit coverage for supported compatibility targets, unsupported codec encode support, and missing target rows.
  - Added app-service coverage proving direct non-dry-run job creation rejects an unsupported compatibility target before queueing.
  - Kept the API media watcher coverage deterministic when the background watcher wins the source-fingerprint race before the explicit watcher endpoint runs.
  - Full local `just ci` and `just ui-e2e` remain dependent on the local Docker/Postgres availability required by the repository harness.
- Observability updates:
  - No new metrics were added. The existing app error code surface now identifies compatibility readiness admission failures.
- Status-doc validation:
  - ADR index and book summary were updated for this task record.
- Risk & rollback plan:
  - Risk: operators with previously accepted non-dry-run profiles can now receive validation errors until their capability snapshot or compatibility target catalog is corrected.
  - Rollback: revert this ADR's service gate and tests, restoring global snapshot-only admission. That rollback should require explicit operator consent because it allows known-unsupported target profiles to queue work.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No policy drift or contradictions were found while adding this slice.
