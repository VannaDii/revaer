# Media audio constraint verification

- Status: Accepted
- Date: 2026-07-22
- Context:
  - Desired-target audio rows already carried codec, channel count, and channel layout, but the immutable target graph could not express or prove probeable audio bitrate and sample-rate requirements.
  - FFmpeg command construction could therefore produce graph-compatible audio that satisfied stream shape while missing operator-selected bitrate or sample-rate constraints.
  - ADR 317 keeps loudness and dynamic-range policy open; this decision covers only the probeable bitrate and sample-rate slice.
- Decision:
  - Add immutable desired-target and job-snapshot fields for `audio_bitrate_bps` and `audio_sample_rate_hz` on audio streams only.
  - Preserve audio sample rate in complete FFprobe inspection reports.
  - Apply configured audio bitrate and sample-rate constraints to FFmpeg audio transcodes with per-output-stream arguments.
  - Add source, candidate, and final audio-constraint verification checks after successful graph verification. A graph-compatible output now fails closed when inspected bitrate or sample rate is missing or mismatched.
- Consequences:
  - Targeted audio transcodes can now be configured and accepted only when FFprobe confirms the requested bitrate tolerance and exact sample rate.
  - Verification history records `source_audio_constraints`, `candidate_audio_constraints`, or `final_audio_constraints` next to existing graph and video-constraint checks.
  - Loudness, dynamic range, richer audio layout semantics, and subjective quality policy remain open implementation work.
- Follow-up:
  - Add explicit loudness and dynamic-range target fields, execution policy, and verification once their acceptance criteria are defined.
  - Continue expanding full-inspection verification for chapters, attachments, arbitrary metadata, and richer HDR/color side-data.

## Task Record

- Motivation:
  - Close the false-success path where a candidate can match the desired audio graph while missing configured bitrate or sample-rate properties.
- Design notes:
  - Audio bitrate and sample rate remain outside `MediaGraph`; verification uses the complete inspection report and immutable job snapshot.
  - Stored procedures validate audio-only usage and positive values, and job snapshots copy the target stream constraints at enqueue time.
  - Execution maps target constraints to desired audio stream ids in target order, matching the existing deterministic constraint-mapping pattern.
- Test coverage summary:
  - Added FFprobe adapter coverage proving audio `sample_rate` survives complete inspection normalization.
  - Reran `cargo test -p revaer-media-runtime inspect:: -- --nocapture`.
  - Reran `cargo test -p revaer-media-runtime execute:: -- --nocapture`, including audio shape command construction.
  - Added and ran `cargo test -p revaer-app media_job_runtime_rejects_candidate_audio_constraint_mismatch -- --nocapture`, proving a graph-compatible candidate fails before replacement when sample rate differs and FFmpeg argv includes the requested audio bitrate and sample rate.
  - Ran the full `just ci` gate, including live database migrations through migration 157, workspace tests, per-crate 90% line coverage enforcement, LCOV and HTML coverage artifact generation, and release build.
  - Ran the full `just ui-e2e` gate; all 104 Playwright tests passed.
- Observability updates:
  - Added persisted verification check kinds for source, candidate, and final audio constraints. Existing media-job failure events and metrics are reused.
- Status-doc validation:
  - Updated ADR 317 to record audio bitrate and sample-rate execution/verification as operational while leaving loudness, dynamic range, richer HDR, chapters, attachments, and arbitrary metadata open.
- Risk & rollback plan:
  - Risk is over-strict verification on encoders or containers that report bitrate imprecisely. Verification keeps the existing bounded bitrate tolerance and exact sample-rate comparison. Roll back by reverting the migration, DTO, execution, and verification changes together; do not silently drop checks from a live target.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No Sonar, lint, coverage, workflow, dependency, or source-scope relaxation was introduced.
