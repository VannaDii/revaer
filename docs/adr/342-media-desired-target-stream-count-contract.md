# Media desired target stream count contract

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Desired targets are immutable output contracts and must contain at least one stream row.
  - The HTTP handler already rejected `streams: []`, but lower service, storage, runtime snapshot, and OpenAPI surfaces did not all express the same invariant.
- Decision:
  - Reject empty desired-target stream lists in `MediaService::media_desired_target_create` before opening a transaction.
  - Advertise `minItems: 1` on `MediaDesiredTargetCreateRequest.streams` and refresh both OpenAPI artifacts.
  - Harden stored procedures so an empty desired target cannot be pinned to a profile or snapshotted into a job.
  - Defensively reject empty persisted job target snapshots in the media runtime.
- Consequences:
  - Target contracts now fail closed across HTTP, app service, storage, documentation, and runtime reconstruction.
  - Existing manually inserted empty desired-target rows remain inert: they cannot be pinned or used to queue jobs.
- Follow-up:
  - Continue process-boundary review for capability probing.
  - Continue closing remaining media service gaps through focused stacked PRs.

## Task Record

- Motivation:
  - Move the media service closer to production correctness by eliminating an empty desired-target graph path below the HTTP handler.
- Design notes:
  - The service guard avoids creating orphan empty target rows through the normal app facade.
  - The storage migration preserves the existing stored-procedure entry points while adding fail-closed stream-count checks at pin and job snapshot time.
  - The runtime guard treats corrupt or legacy persisted snapshots as invalid desired graphs.
- Test coverage summary:
  - Added OpenAPI unit coverage for `MediaDesiredTargetCreateRequest.streams.minItems == 1`.
  - Added app-service coverage proving empty target creation returns `media_desired_target_streams_required` and leaves no listed target.
  - Added stored-procedure coverage proving an empty desired target cannot be pinned.
  - Added runtime unit coverage proving an empty desired-target job snapshot returns `media_job_desired_target_snapshot_empty`.
- Observability updates:
  - Existing API/service/data error-code surfaces are reused.
  - Runtime invalid-graph errors continue to surface through existing media job failure persistence.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`; no instruction drift was introduced.
- Risk & rollback plan:
  - Risk: manually seeded empty targets become unusable until stream rows are appended.
  - Rollback: revert this ADR, migration, service guard, OpenAPI schema update, and runtime defensive check.
- Dependency rationale:
  - No dependencies were added.
