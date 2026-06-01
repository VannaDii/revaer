# Media job cancel and retry API slice 10/14

- Status: Accepted
- Date: 2026-05-31
- Context:
  - `MEDIA_TRANSCODING.md` calls for API coverage for job cancellation and retry, and execution slice 14 requires cancellation and failure recovery paths.
  - The data layer and runtime media store already exposed stored-procedure backed cancel/retry operations, but the API facade and HTTP router did not make them reachable.
- Decision:
  - Add `media_job_cancel` and `media_job_retry` to the media application facade.
  - Wire production `MediaService` to `MediaStore::cancel_job` and `MediaStore::retry_job`.
  - Add authenticated `POST /v1/media/jobs/{media_job_public_id}/cancel` and `POST /v1/media/jobs/{media_job_public_id}/retry` routes that return `204 No Content` on success.
  - Keep not-found, invalid-status, and storage mapping centralized through the existing media error mapper.
- Consequences:
  - Positive outcomes:
    - Operators and future workers can cancel queued/running/verifying jobs through the API boundary.
    - Failed or cancelled media jobs can be requeued through the same API boundary.
    - Existing stored-procedure status guards remain the source of truth for legal transitions.
  - Risks or trade-offs:
    - The current UI still lists jobs compactly; a richer job action surface remains a follow-up.
    - Cancel and retry are state transitions only; worker interruption and cleanup behavior still needs broader execution-runtime coverage.
- Follow-up:
  - Add UI controls for job cancellation and retry once the jobs table is expanded beyond the compact recent-jobs list.
  - Extend worker orchestration tests so cancellation also interrupts active media execution and triggers workspace cleanup.

## Task Record

- Motivation:
  - Continue closing API and execution-control gaps from `MEDIA_TRANSCODING.md` with the smallest useful status-transition surface.
- Design notes:
  - Reused the existing stored procedures and `MediaStore` methods rather than adding duplicate status logic in the API layer.
  - Used action subresources under the existing job resource to avoid adding request bodies for transitions that need only a job id.
  - Kept route protection aligned with the rest of `/v1/media`.
- Test coverage summary:
  - Added handler tests for cancel and retry error mapping against the default unavailable media facade; both failed before the handlers existed and pass after wiring.
  - Extended the app media service round-trip test to cancel a queued job and retry the cancelled job back to queued.
- Observability updates:
  - No new telemetry was added. Job status changes remain observable through existing job list/get responses.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: clients may call cancel/retry while worker cleanup is still incomplete. Roll back by reverting the facade methods, service delegation, handlers, routes, tests, and this ADR.
- Dependency rationale:
  - No dependencies were added. This slice uses existing Axum, application facade, and runtime store paths.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
