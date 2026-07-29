# Media video constraint verification

- Status: Accepted
- Date: 2026-07-22
- Context:
  - ADR 324 made immutable desired-target video constraints affect FFmpeg command construction, but candidate and final verification still accepted any output whose projected stream graph matched.
  - That left a false-success path where a candidate could have the expected container, stream order, and codec while missing the requested video profile, level, bitrate, or HDR/color signaling.
- Decision:
  - Carry the immutable desired-target snapshot through the worker execution path after preflight compilation.
  - Add a separate video-constraint verification check after every successful graph comparison for source no-op, candidate, and final replacement verification.
  - Match constrained target rows to desired video stream ids with the same target-order and codec mapping used by execution policy construction.
  - Fail closed when an expected constrained stream or field is absent. Normalize profile strings compactly, compare level through normalized stream metadata, allow a bounded 5% bitrate tolerance, and treat `hdr10` as requiring BT.2020 primaries, SMPTE ST 2084 transfer, and BT.2020 non-constant luminance colorspace.
- Consequences:
  - A graph-compatible candidate can no longer replace the source if the inspected technical video constraints are wrong or missing.
  - Verification history now records `source_video_constraints`, `candidate_video_constraints`, or `final_video_constraints` beside the existing graph checks, with the first mismatched stream field in the stored details.
  - HDR verification remains limited to HDR10 color signaling until inspection retains and enforces richer mastering-display, content-light, Dolby Vision, and HLG semantics.
- Follow-up:
  - Extend immutable target policy to audio loudness and dynamic range.
  - Add richer HDR side-data verification and codec-specific level normalization once those contracts are represented explicitly.

## Task Record

- Motivation:
  - Close the false-success gap between executing requested video constraints and proving that the accepted candidate/final output actually carries them.
- Design notes:
  - The pure desired graph remains unchanged; technical constraint verification is inspection-level because fields such as profile, bitrate, color, and side data are not part of `MediaGraph`.
  - Verification uses the job snapshot, not current profile configuration, so later profile edits cannot alter an in-flight job's acceptance criteria.
  - The new checks use non-overlapping verification indexes to avoid colliding with candidate-safety and final-sidecar checks.
- Test coverage summary:
  - Added a worker regression where the candidate graph matches but color transfer differs from the immutable HDR10 target. The job fails before replacement, records a failed `candidate_video_constraints` row, and preserves source bytes.
  - Reran the media-job runtime test filter: `cargo test -p revaer-app media_job_runtime_ -- --nocapture`.
- Observability updates:
  - Added persisted verification check kinds for source, candidate, and final video constraints. No new metric labels were added.
- Status-doc validation:
  - Rechecked ADR 317 and ADR 324. ADR 317 now describes the narrower remaining media-transcoding gaps after this verification slice.
- Risk & rollback plan:
  - Risk is over-strict verification on real encoders that omit level metadata or report bitrate differently. Roll back by removing the video-constraint check while keeping command construction intact; do not relax silently.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No policy drift or quality-gate relaxation was introduced.
