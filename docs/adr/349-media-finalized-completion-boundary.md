# Media finalized completion boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Cooperative cancellation correctly fences normal completion before the worker crosses the destructive replacement boundary.
  - A replacement that has already been finalized on the source filesystem cannot be represented safely as cancelled afterward.
- Decision:
  - Add a finalized-replacement completion stored procedure that always records `completed` for worker-owned finalized replacements.
  - Acknowledge any cancellation that arrived after finalization and report that fact back to the runtime.
  - Use that same finalized boundary from live execution and startup recovery, recording late post-finalization cancellation as verification evidence while still publishing the completed lifecycle event.
- Consequences:
  - Job state remains consistent with irreversible filesystem state after finalized replacements.
  - Operators can still see that a cancellation arrived too late to stop the already-finalized mutation.

## Task Record

- Motivation:
  - Close the race where a cancellation requested after replacement finalization could make durable job state say `cancelled` even though the source file had been replaced and rollback evidence removed.
- Design notes:
  - Normal dry-run and no-op completion keep the cancellation-aware completion path.
  - Destructive replacement jobs switch to a dedicated finalized-completion procedure only after final replacement verification and backend finalization succeed.
  - The procedure acknowledges the current cancel generation so stale-worker recovery does not reinterpret the late request, rejects merely running jobs, and remains idempotent for already-completed recovered jobs.
- Test coverage summary:
  - Added data-layer stored-procedure tests for finalized completion with a pending cancellation, running-job rejection, and repeated completion idempotency.
  - Added an app runtime regression test that pauses after filesystem finalization, requests cancellation, then verifies completed job state, source replacement, verification evidence, and completion event publication.
  - Extended startup replacement recovery coverage to verify recovered finalized replacements acknowledge late cancellation and remain completed.
- Observability updates:
  - Added a verification check row named `late_cancel_after_finalized_replace` when cancellation arrives after finalized replacement.
- Status-doc validation:
  - Reviewed the media ADR index and mdBook summary for the new task record entry.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: callers could misuse the finalized path before the filesystem mutation is irreversible.
  - Rollback: revert this procedure and runtime call-site change; rerun media data and app runtime tests.
- Dependency rationale:
  - No dependencies were added.
