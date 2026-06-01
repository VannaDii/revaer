# Media jobs empty-list refresh slice 11

- Status: Accepted
- Date: 2026-05-31
- Context:
  - The media UI refresh flow requests `/v1/media/jobs` before an operator has selected or created a media profile.
  - The API handler required `media_profile_public_id`, causing the whole refresh bundle to fail and preventing unrelated media page data, including compliance metadata, from rendering.
- Decision:
  - Make the media jobs profile filter optional at the HTTP query boundary.
  - Preserve status validation when provided.
  - Return an empty job list when no profile filter is supplied.
- Consequences:
  - Positive outcomes:
    - The media page can render profile counts, readiness, latest capability, compliance metadata, and zero jobs in an empty first-run state.
    - Profile-scoped job listing behavior remains unchanged when a profile id is provided.
  - Risks or trade-offs:
    - This is not a global jobs listing; broader jobs-tab behavior still needs a dedicated stored procedure and API contract.
- Follow-up:
  - Add a real recent-jobs query once the jobs tab expands beyond profile-scoped listing.

## Task Record

- Motivation:
  - Keep the slice-11 UI surface usable in an empty first-run state and avoid blocking the compliance panel on an unrelated missing profile filter.
- Design notes:
  - Chose an empty response over implicit global listing because persistence currently exposes profile-scoped job listing only.
  - Kept invalid status filters as validation errors even when no profile id is provided.
- Test coverage summary:
  - Added a failing handler test for no profile filter returning an empty job list.
  - Ran the focused media jobs handler tests after implementation.
- Observability updates:
  - None. This is request-shape compatibility for an existing read endpoint.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: clients may mistake the empty unfiltered response for global job support. Roll back by reverting the optional query change, test, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
