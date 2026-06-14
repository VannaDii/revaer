# Media HDR Format Fail-Closed Validation

- Status: Accepted
- Date: 2026-07-22

## Context

- Desired target video rows accepted any non-empty `hdr_format` string.
- Runtime verification only has an implemented contract for `hdr10`: BT.2020 primaries, SMPTE ST 2084 transfer, BT.2020 non-constant luminance colorspace, and required mastering-display/content-light side-data descriptors.
- Accepting other HDR labels into immutable target catalogs created a false contract. A profile could persist an unsupported desired state and fail later at verification instead of being rejected before enqueue and compilation.

## Decision

- Treat `hdr10` as the only supported `hdr_format` value until the desired-target schema, FFmpeg argument construction, complete inspection payload, and candidate/final verification model exact semantics for another HDR format.
- Reject unknown HDR format values in the core desired-target compiler and in the stored-procedure-backed desired-target stream append path.
- Add database check constraints to keep both catalog rows and immutable job snapshots from storing unsupported HDR labels.

## Consequences

- Unsupported HDR formats now fail closed at configuration time with the existing `media_desired_target_video_shape_invalid` database detail.
- The runtime no longer needs to discover an unsupported HDR target after a job has already been snapshotted.
- Adding Dolby Vision, HDR10+, HLG, or exact mastering-display/content-light value semantics requires an explicit schema and verification expansion instead of a loose string.

## Task Record

- Motivation:
  - Move the service closer to production correctness by removing an unsupported desired-state input that could be mistaken for a verified media contract.
- Design notes:
  - Keep the accepted HDR value set intentionally small and versioned in both core validation and database constraints.
  - Preserve the existing data-layer function signature so application code and SQLx query shape stay stable.
  - Reuse the existing video-shape database error detail for unsupported HDR labels because the invalid field is part of the video target shape.
- Test coverage summary:
  - Added core desired-target compiler coverage proving a video stream with `hdr_format = dolby_vision` returns `UnsupportedHdrFormat`.
  - Added stored-procedure integration coverage proving the same unsupported HDR label is rejected with `media_desired_target_video_shape_invalid`.
- Observability updates:
  - No new metrics or log labels were added.
  - Rejections surface through existing configuration-validation errors before job creation.
- Risk and rollback plan:
  - Risk: any existing local draft or catalog row using an unsupported HDR label will now be rejected by migration or future writes.
  - Roll back by dropping the HDR-format check constraints and removing the stored-procedure/core validation, while keeping the implemented HDR10 verifier intact.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, ADR 317, and ADR 331.
  - No policy drift or criteria relaxation was introduced.
