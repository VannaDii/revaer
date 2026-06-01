# Media profile not-found detail slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Media profile lookup paths returned a generic list failure detail when a requested profile was absent.
  - Review feedback called out that genuine 404 responses should communicate not-found instead of list failure.
- Decision:
  - Add a dedicated `media profile not found` detail for profile GET and profile PATCH lookup paths.
  - Keep storage/list failures mapped through the existing list-failure detail.
- Consequences:
  - Positive outcomes:
    - API clients receive an accurate problem detail for profile 404s.
    - List operation failures remain distinguishable from missing resources.
  - Risks or trade-offs:
    - Clients matching the previous generic text for profile 404s must update to the clearer detail.
- Follow-up:
  - Keep route-specific not-found details for future media resources as endpoints are added.

## Task Record

- Motivation:
  - Resolve the lingering media-profile not-found detail mismatch and keep PR feedback aligned with the branch contents.
- Design notes:
  - Changed only the post-list lookup miss paths; underlying list facade errors still use the list failure detail.
- Test coverage summary:
  - Added a failing assertion for the profile GET problem detail before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api get_media_profile_returns_not_found_with_default_facade -- --test-threads=1`.
- Observability updates:
  - None. This changes API problem-detail text only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: API clients relying on the old generic detail text must adapt. Roll back by restoring the prior detail constant in the profile lookup misses and removing this ADR/test assertion.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
