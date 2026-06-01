# Media UI route foundation slice 11

- Status: Accepted
- Date: 2026-05-30
- Context:
  - Media API/runtime slices exist, but the UI lacked a dedicated route for media profile/job/capability visibility.
  - `MEDIA_TRANSCODING.md` slice 11 requires a first-party media configuration surface.
- Decision:
  - Added a new `/media` route and sidebar entry.
  - Added a new `features/media` UI slice with typed API calls for profiles, jobs, readiness, latest capability, and capability refresh.
  - Added a UI E2E spec for the new media route.
- Consequences:
  - Positive outcomes:
    - Operators can access a dedicated media page in the UI.
    - Media API integration now has UI-level coverage.
  - Risks or trade-offs:
    - This is a minimal foundation page and will need follow-up expansion for full slice-11 UX.
- Follow-up:
  - Expand this route with profile CRUD forms, import/export controls, and discovery/scheduling controls.

## Task Record

- Motivation:
  - Continue implementing the media transcoding plan with visible operator-facing UI.
- Design notes:
  - Kept media transport logic in `features/media/api.rs` and feature-local state in `features/media/state.rs`.
  - Preserved existing shell/sidebar architecture while adding a single route and icon.
- Test coverage summary:
  - Added `tests/specs/ui/media.spec.ts`.
  - Ran `just ui-e2e` successfully.
- Observability updates:
  - No additional telemetry/logging changes in this slice.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: no.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Rollback by reverting this commit to remove the media route and feature module wiring.
- Dependency rationale:
  - No new dependencies added.
