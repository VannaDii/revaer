# Media UI foundation slice 11

- Status: Accepted
- Date: 2026-05-30
- Context:
  - Media transcoding backend slices are present, but the UI had no first-party route for profile/job/capability visibility or YAML import/export workflows.
  - `MEDIA_TRANSCODING.md` slice 11 requires a media feature surface in `revaer-ui`.
- Decision:
  - Added a new `Media` UI route and sidebar entry.
  - Implemented a `features/media` slice with typed API helpers and a page that exposes profile/job inventory, capability readiness/refresh, and YAML validate/apply/export interactions.
  - Added a UI E2E spec that verifies the route renders the expected controls.
  - Alternatives considered:
    - Fold media controls into the indexers page (rejected: mixes unrelated operator domains and weakens feature-slice boundaries).
    - Delay UI until all runtime slices are complete (rejected: blocks operator visibility and import/export verification paths).
- Consequences:
  - Positive outcomes:
    - Operators can access media transcoding controls from a dedicated route.
    - UI now exercises key media API surfaces during E2E.
  - Risks or trade-offs:
    - Initial media page is intentionally minimal and will require incremental UX hardening as additional media behaviors land.
- Follow-up:
  - Expand media tabs/forms to cover full profile CRUD and watcher/schedule workflows from slice 11.
  - Add deeper UI validation/error state coverage for import/apply outcomes.

## Task Record

- Motivation:
  - Continue `MEDIA_TRANSCODING.md` with a concrete slice-11 implementation rather than keeping media functionality API-only.
- Design notes:
  - Kept transport calls in `features/media/api.rs` and route rendering in `features/media/view.rs`.
  - Wired route + shell nav with a dedicated media icon to preserve existing app layout patterns.
  - Used existing API models and shared authenticated client; no new dependencies introduced.
- Test coverage summary:
  - Added `tests/specs/ui/media.spec.ts` to validate media route rendering and core controls.
  - Re-ran full required gates (`just ci`, `just ui-e2e`).
- Observability updates:
  - No new telemetry/events added in this slice.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md` for policy alignment.
  - No stale references or contradictions discovered in those instruction files for this slice.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: no.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk: route wiring or async media calls could regress sidebar navigation or render behavior.
  - Rollback: revert this ADR's linked commit to remove media route/slice and restore prior navigation.
- Dependency rationale:
  - No new dependencies were added; existing Yew and API client stack is sufficient.
