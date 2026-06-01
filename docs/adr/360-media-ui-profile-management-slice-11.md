# Media UI profile management slice 11

- Status: Accepted
- Date: 2026-05-30
- Context:
  - The media page existed as a read-only foundation and did not let operators manage profile lifecycle.
  - `MEDIA_TRANSCODING.md` slice 11 requires profile and policy controls in the UI surface.
- Decision:
  - Added typed UI API calls for creating media profiles and patching existing profiles.
  - Added an in-page profile creation form for key, roots, retention, and dry-run mode.
  - Added per-profile dry-run toggle actions to switch between dry-run and replace-enabled states.
- Consequences:
  - Positive outcomes:
    - Operators can now perform core profile management from the UI without leaving the media view.
    - The media route now exercises write-path API integration in addition to read snapshots.
  - Risks or trade-offs:
    - Input validation remains lightweight client-side and relies on API error responses for full constraints.
- Follow-up:
  - Add target/policy compatibility editors and discovery watcher/schedule controls.

## Task Record

- Motivation:
  - Continue the media transcoding implementation plan by moving slice 11 from route scaffolding into actionable configuration workflows.
- Design notes:
  - Kept transport calls in `features/media/api.rs` and confined form state/actions to `features/media/view.rs`.
  - Reused existing `/v1/media/profiles` create and patch endpoints to avoid duplicate API surfaces.
- Test coverage summary:
  - Updated `tests/specs/ui/media.spec.ts` assertions for the new profile management surface.
  - `just ci` currently fails at the existing per-crate coverage gate baseline unrelated to this UI change.
  - `just ui-e2e` rerun is required after final branch stabilization.
- Observability updates:
  - No telemetry schema changes; toast messages added for profile create/patch outcomes.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: yes.
  - Contradictions/stale references removed: ADR catalog/sidebar references were behind active media ADR files and were updated.
- Risk & rollback plan:
  - Roll back by reverting this ADR-linked commit to restore read-only media UI behavior.
- Dependency rationale:
  - No new dependencies added.
