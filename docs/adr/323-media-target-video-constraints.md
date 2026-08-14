# Media target video constraints

- Status: Accepted
- Date: 2026-07-21
- Context:
  - ADR 317 keeps the media transcoding service explicitly incomplete until desired targets cover normalized video profile, level, bitrate, HDR, color, and remaining stream constraints with acceptance-test evidence.
  - The implemented desired-target graph already persists container, stream kind/order, codec, audio shape, subtitle placement, dispositions, and titles, but video-specific technical constraints were not represented in immutable target versions or job snapshots.
  - The repository requires normalized relational persistence, stored-procedure runtime access, no hidden source/test exclusions, and explicit task records for implementation slices.
- Decision:
  - Add nullable video-only constraint fields to `media_desired_target_stream` and `media_job_desired_target_stream`: `video_profile`, `video_level`, `video_bitrate_bps`, `color_primaries`, `color_transfer`, `color_space`, and `hdr_format`.
  - Add v3 append/list stored procedures for desired-target streams and job desired-target stream snapshots while preserving the existing v1/v2 procedure contracts.
  - Reject non-positive video bitrates and reject video-only fields on non-video streams in both the database and core target validation.
  - Carry the fields through API DTOs, handler normalization, service mapping, YAML/catalog round trips, worker snapshot reconstruction, and data integration tests.
  - Alternatives considered:
    - Deferring the fields until execution support exists was rejected because queued jobs must snapshot immutable desired intent before the worker can enforce it.
    - Adding a JSON metadata column was rejected because repo policy forbids conglomerate persistence for application state.
- Consequences:
  - Desired targets can now persist and snapshot the main video constraints called out by ADR 317 without weakening existing audio or subtitle validation.
  - Execution and candidate verification still need to enforce these constraints against FFmpeg args and post-replacement inspection before the broader media service can be called complete.
- Follow-up:
  - Use the stored video constraints to drive FFmpeg encoder/profile/color arguments.
  - Extend candidate and final verification to compare probed video profile, level, bitrate tolerance, HDR, and color fields against the immutable target snapshot.

## Task Record

- Motivation:
  - Make concrete progress against the remaining desired-target graph gap without hiding the service-level incompleteness.
- Design notes:
  - The v3 stored procedures keep runtime access procedure-backed and preserve older procedure compatibility.
  - Video shape validation lives in both SQL and `revaer-media-core` so API, worker, and database boundaries agree.
  - Job snapshots copy the video constraints at enqueue time so later catalog edits cannot alter claimed work.
- Test coverage summary:
  - Added data-layer assertions that video constraints round-trip through desired-target catalog rows and are copied into job snapshots.
  - Added core validation coverage for video-only fields on non-video streams.
  - Refreshed the test Playwright lockfile so transitive `fast-uri` resolves to the non-vulnerable `3.1.4` release and `npm --prefix tests audit --audit-level=low` reports zero vulnerabilities.
  - Verification run in this turn: `just ci` and `just ui-e2e` passed after the final lockfile and mapping changes; `just ci` regenerated `coverage/lcov.info` and passed the per-crate coverage threshold.
  - Local Sonar issue-state query reported no open or confirmed issues for `VannaDii_Revaer`. Direct local file verification upload was blocked by the execution approval policy, so scanner-side PR analysis remains the authoritative Sonar pass.
- Observability updates:
  - No new metrics or events; this slice extends immutable configuration and snapshot evidence only.
- Status-doc validation:
  - Rechecked `MEDIA_TRANSCODING.md` and ADR 317. ADR 317 is updated to state the narrower remaining gap accurately.
- Risk & rollback plan:
  - Roll back by reverting migration 0156 and the corresponding DTO/DAL/core mappings before any deployment applies the migration.
  - Existing v1/v2 stored-procedure callers remain compatible because they pass null video constraints.
- Dependency rationale:
  - No new dependencies.
  - Updated the existing test-only transitive `fast-uri` lockfile entry from `3.1.3` to `3.1.4` to resolve GHSA-v2hh-gcrm-f6hx without adding or suppressing dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No policy drift was introduced or found that required relaxing criteria.
