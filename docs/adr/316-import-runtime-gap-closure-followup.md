# Import runtime gap-closure follow-up

- Status: Accepted
- Date: 2026-05-23
- Context:
  - The import job runtime worker landed, but two gaps remained:
    - `prowlarr_api` runtime handling used a single placeholder identifier and error reason without validating the persisted run configuration snapshot.
    - `ERD_INDEXERS_CHECKLIST.md` still marked the in-process scheduler item unchecked even though scheduler/runtime wiring had already shipped.
- Decision:
  - Tighten `ImportJobRuntime` `prowlarr_api` processing by validating required config snapshot keys (`prowlarr_url`, `secret_public_id`) before result recording.
  - Derive deterministic `prowlarr_identifier` from configured Prowlarr host/path prefix instead of a fixed placeholder string.
  - Keep live external API execution explicitly out of scope for this pass and continue failing closed with stable runtime detail codes.
  - Reconcile checklist truth by marking the in-process scheduler/worker item complete and annotating remaining runtime-executor sub-gaps explicitly.
- Consequences:
  - Positive outcomes:
    - Runtime output is more actionable and deterministic for operators reviewing import results.
    - Checklist status now matches shipped scheduler behavior while preserving open runtime executor work as unchecked.
  - Risks or trade-offs:
    - `prowlarr_api` still does not execute live remote imports; this change improves correctness/traceability but does not complete end-to-end payload ingestion.
- Follow-up:
  - Add live outbound import execution for `prowlarr_api`, including secret decryption/runtime adapter wiring and result materialization.
  - Add acceptance tests that validate runtime result identifiers and error details from real run configurations.

## Task record

- Motivation:
  - Close immediate runtime and status-tracking gaps without overstating completion of the broader live import executor scope.
- Design notes:
  - Configuration parsing stays deterministic and local to runtime worker input; no new dependencies added.
  - Identifier derivation uses bounded truncation to preserve column limits and panic-free behavior.
- Test coverage summary:
  - Added pure unit coverage for URL-to-identifier derivation helper behavior.
  - Full validation remains `just ci` and `just ui-e2e` per repo policy.
- Observability updates:
  - Runtime failure details now distinguish missing runtime config from not-yet-implemented live executor behavior.
- Risk & rollback plan:
  - Rollback by reverting `import_job_runtime.rs` helper/branch logic and checklist/ADR documentation updates.
- Dependency rationale:
  - No new crates introduced; reused existing runtime and helper patterns.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - No instruction contradictions found; removed checklist drift by aligning shipped scheduler status with actual implementation.
