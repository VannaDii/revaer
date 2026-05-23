# Indexer import job runtime worker

- Status: Accepted
- Date: 2026-05-23
- Context:
  - Import job API/CLI surfaces existed, but running jobs could remain in `running` status with no runtime worker finalizing results.
  - Revaer policy requires runtime data access through stored procedures and deterministic, panic-free background execution.
- Decision:
  - Add worker stored procedures for claim, result recording, and terminal state transitions in migration `0122_import_job_runtime_worker.sql`.
  - Add `ImportJobRuntime` in `revaer-app` and wire it into bootstrap lifecycle start/stop alongside the indexer runtime.
  - Implement deterministic runtime handling for supported import kinds:
    - `prowlarr_backup`: parse backup reference details, record a deterministic result code, and mark completed.
    - `prowlarr_api`: record deterministic not-configured failure and mark failed until live API integration is implemented.
  - Keep all runtime state transitions and writes within `revaer-data` stored-procedure wrappers.
- Consequences:
  - Positive outcomes:
    - Import jobs no longer stall in `running` due to missing worker finalization.
    - Runtime behavior is explicit, testable, and aligned with stored-procedure boundaries.
  - Risks or trade-offs:
    - `prowlarr_api` remains a deliberate placeholder path that fails closed with explicit reason codes until full live importer integration lands.
- Follow-up:
  - Implement live `prowlarr_api` import execution and success-path result materialization.
  - Add end-to-end acceptance coverage that validates runtime transitions from queued to terminal states through API and CLI.

## Task record

- Motivation:
  - Close the remaining import execution gap so created/run import jobs can reach terminal states deterministically.
- Design notes:
  - Runtime worker polling follows existing runtime patterns and remains dependency-injected via app bootstrap wiring.
  - Data-layer wrappers expose typed claim/result/terminal operations and keep SQL confined to migration procedures.
- Test coverage summary:
  - Added data-layer tests for worker claim/result/terminal procedures in `revaer-data`.
  - Re-ran workspace gates via `just ci` and `just ui-e2e` for integrated validation.
- Observability updates:
  - Runtime now emits deterministic result/failure outcomes for import jobs, improving operational traceability of terminal job states.
- Risk & rollback plan:
  - Rollback by reverting migration `0122`, runtime wiring, and wrapper usage; prior behavior reverts to API/CLI-only import job lifecycle without worker finalization.
- Dependency rationale:
  - No new dependencies added; reused existing runtime, data, and telemetry infrastructure.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md` for this scope.
  - No policy contradictions found in this implementation.
  - Removed stale status framing by updating ADR 174 to reflect worker-path availability and explicit remaining `prowlarr_api` limitation.
