# Media UI retention bounds validation slice 11/12

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires UI configuration surfaces to avoid unbounded diagnostic retention.
  - The profile form parsed retention days as an integer but did not enforce the documented 1 through 3650 day bounds before submission.
- Decision:
  - Add a pure UI helper that parses retention days and rejects non-numeric or out-of-bounds values.
  - Route profile creation through that helper so invalid retention values are blocked before API submission.
- Consequences:
  - Positive outcomes:
    - Operators receive immediate feedback for invalid retention settings.
    - The UI now mirrors the database/profile validation bounds for retention days.
  - Risks or trade-offs:
    - Future retention policy bounds must update both the backend validation and this UI helper.
- Follow-up:
  - Add equivalent browser E2E coverage when the media profile flow has stable fixtures.
  - Extend UI validation to schedule interval bounds and any diagnostic artifact retention-specific controls as they land.

## Task Record

- Motivation:
  - Prevent the media UI from submitting retention settings outside the bounded policy range required by the plan.
- Design notes:
  - Kept validation in `features/media/logic.rs` so it is testable without rendering the page.
  - Reused the helper in the profile form instead of adding another inline parse branch.
- Test coverage summary:
  - Added a failing UI logic test for non-numeric, zero, and above-maximum retention values before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui parse_retention_days_input_rejects_non_numeric_and_unbounded_values -- --test-threads=1`.
- Observability updates:
  - None. This is client-side validation before API calls.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: users entering previously accepted invalid retention values now see a client-side error. Roll back by removing the helper, form wiring, test, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
