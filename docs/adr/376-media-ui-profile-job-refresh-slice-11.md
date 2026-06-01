# Media UI profile job refresh slice 11

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media jobs API requires a profile filter, but the UI refresh path called the collection route without a profile id.
  - The media plan requires the UI jobs surface to show actual job rows, not a permanently empty list when profiles exist.
- Decision:
  - Add a pure media UI helper for building the profile-filtered jobs path.
  - Update the media API wrapper to fetch jobs for each loaded profile and merge the returned rows.
  - Change the media page refresh flow to load profiles before loading profile-scoped jobs.
- Consequences:
  - Positive outcomes:
    - The media page can display jobs associated with configured profiles.
    - The path-building behavior is covered by native unit tests while the wasm-only view remains checked through a wasm build.
  - Risks or trade-offs:
    - Job requests are currently sequential; this is simpler and deterministic but can be optimized if profiles become numerous.
- Follow-up:
  - Add job-detail panes for persisted operations and violations.
  - Add profile/status filters once the jobs tab grows beyond the first release foundation.

## Task Record

- Motivation:
  - Continue slice 11 by making the existing UI jobs panel query the implemented API shape.
- Design notes:
  - Kept transport DTOs in `models.rs` and placed UI-only path construction in the media feature logic module.
  - Preserved the existing refresh UX and error toast behavior.
- Test coverage summary:
  - Added failing native unit tests for media job path construction, then implemented the helper.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-ui media_jobs_path -- --test-threads=1`.
  - Ran `rustup target add wasm32-unknown-unknown && CARGO_TARGET_DIR=target/media-compliance-red cargo check -p revaer-ui --target wasm32-unknown-unknown --all-features`.
- Observability updates:
  - None. This is UI request wiring only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: a large profile set can make refresh latency linear. Roll back by restoring the single jobs request and removing the helper, tests, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
