# Media stale worker recovery

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Media jobs are durably claimed by moving queued rows to running or verifying and refreshing `heartbeat_at` while work proceeds.
  - A process restart after claim could leave a media job in progress forever because the worker only claimed queued jobs.
- Decision:
  - Add a stored-procedure recovery boundary for stale running and verifying media jobs.
  - Treat stale jobs with an unacknowledged cancellation as cancelled, and stale jobs without a pending cancellation as failed with `media_job_worker_heartbeat_stale`.
  - Run stale recovery before media job claiming so restarted workers clear abandoned in-flight rows before accepting new work.
- Consequences:
  - Operators see terminal job state after a worker restart instead of indefinitely running jobs.
  - Long-running active work relies on the existing heartbeat path and the one-hour recovery threshold.
- Follow-up:
  - Consider exposing the recovery threshold as operator configuration if production workloads show a wider runtime envelope.

## Task Record

- Motivation:
  - Close the user-visible reliability gap where media jobs could remain running or verifying after a worker process died.
- Design notes:
  - Runtime database mutation remains behind a stored procedure.
  - Pending cancellation wins stale recovery so an operator-requested cancellation is not reported as a worker failure.
  - The recovery procedure returns affected job ids and terminal status so the runtime can emit the existing failure event for failed recoveries.
  - Startup replacement recovery persists completed job state for already-finalized filesystem transactions before stale-worker recovery runs.
- Test coverage summary:
  - Added a stored-procedure regression covering stale failed recovery, stale cancellation recovery, and retry after stale failure.
  - Added runtime store coverage for database-error surfacing through the new method.
  - Kept the new stored procedure in the schema inventory and tightened visibility for strict all-target lint coverage.
  - Added a runtime regression proving finalized replacement recovery is not later failed by stale-worker recovery.
  - Added runtime coverage for stale-worker failed and cancelled recovery event handling.
- Observability updates:
  - Failed stale recoveries publish `media_job_failed` with `media_job_worker_heartbeat_stale` and log the recovered job id.
- Status-doc validation:
  - Reviewed the media ADR index and mdBook summary for the new task record entry.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: an active job whose heartbeat is blocked for more than one hour can be marked terminal by another worker instance.
  - Rollback: revert the migration and Rust caller/wrapper changes, then rerun media data and runtime tests.
- Dependency rationale:
  - No dependencies were added.
