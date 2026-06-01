# Media destructive dry-run override confirmation slice 10/11

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires destructive manual overrides of dry-run profiles to include the exact typed `replace` confirmation phrase.
  - The media job-create request exposed `dry_run=false` without carrying a confirmation phrase through the API and app service.
- Decision:
  - Add optional `replace_confirmation` to media job-create API payloads and facade parameters.
  - When a manual job requests `dry_run=false`, fetch the owning profile and reject dry-run-profile overrides unless `replace_confirmation` is exactly `replace`.
  - Perform the confirmation check before execution-capability validation so clients receive the destructive-override remediation first.
- Consequences:
  - Positive outcomes:
    - Saved dry-run profiles cannot be manually run destructively without the explicit confirmation required by the plan.
    - Non-dry-run saved profiles keep the existing execution-capability gate and do not require an extra phrase.
  - Risks or trade-offs:
    - Existing clients that send `dry_run=false` for dry-run profiles must add the confirmation field before the request can succeed.
- Follow-up:
  - Add UI job-execution controls that require the same exact phrase before submitting destructive overrides.
  - Add E2E coverage when manual job execution UX lands.

## Task Record

- Motivation:
  - Close the destructive override safety gap in the current media job-create flow.
- Design notes:
  - Kept the confirmation phrase exact and case-sensitive.
  - Reused existing profile lookup and service error mapping rather than duplicating profile state in the HTTP handler.
- Test coverage summary:
  - Added a failing app-service test for dry-run-profile destructive override without confirmation before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-app media_job_create_requires_replace_confirmation_for_dry_run_profile_override -- --test-threads=1`.
  - Regenerated OpenAPI with `just api-export`.
- Observability updates:
  - None. Existing problem-detail mapping surfaces the stable `media_job_replace_confirmation_required` code.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: callers relying on destructive dry-run overrides without confirmation receive 400 responses. Roll back by removing the request field, service validation, OpenAPI update, test, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
