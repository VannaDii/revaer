# Media UI schedule interval bounds validation slice 11/13

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media profile UI allowed schedule intervals to be parsed as integers without enforcing the backend's 1 through 525600 minute bounds.
  - Discovery schedules are disabled by default, but enabled schedules still need bounded client-side validation.
- Decision:
  - Add a pure UI helper that parses schedule intervals and rejects non-numeric or out-of-bounds values.
  - Route optional schedule interval submission through that helper before creating a media profile.
- Consequences:
  - Positive outcomes:
    - Operators receive immediate validation feedback for invalid schedule intervals.
    - UI validation now mirrors the API/database schedule interval bounds.
  - Risks or trade-offs:
    - Future schedule bound changes must update both backend validation and the UI helper.
- Follow-up:
  - Add browser E2E coverage for profile creation validation once media fixtures are stable.
  - Apply the same helper to profile patch controls if schedule editing is split into a dedicated form.

## Task Record

- Motivation:
  - Keep media discovery schedule controls bounded and aligned with backend policy before requests are submitted.
- Design notes:
  - Kept validation in `features/media/logic.rs` alongside retention parsing for focused unit tests.
  - Preserved optional schedule interval behavior: empty input still maps to no interval.
- Test coverage summary:
  - Added a failing UI logic test for non-numeric, zero, and above-maximum schedule interval values before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui parse_schedule_interval_input_rejects_non_numeric_and_unbounded_values -- --test-threads=1`.
- Observability updates:
  - None. This is client-side validation before API calls.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: users entering previously accepted invalid intervals now see a client-side error. Roll back by removing the helper, form wiring, test, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
