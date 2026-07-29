# Media video constraint execution

- Status: Accepted
- Date: 2026-07-22
- Context:
  - ADR 323 persisted video profile, level, bitrate, HDR, and color constraints in desired targets and job snapshots, but the execution builder still only used codec and generic policy-level HDR/color settings.
  - Losing those fields between target compilation and FFmpeg argv generation meant a job could claim immutable video constraints without attempting to encode them.
- Decision:
  - Add per-stream `VideoStreamConstraints` to `VideoTranscodePolicy`, keyed by compiled desired stream id.
  - Populate those constraints in the app worker from the immutable desired-target job snapshot after compiling the target graph.
  - Emit per-output-stream FFmpeg arguments for profile, level, bitrate, color primaries, transfer, and colorspace when a desired video stream is actually transcoded.
  - Keep the existing coarse HDR/color policy for profile-level transform choices; target constraints add exact stream-shape signaling without replacing transform policy.
- Consequences:
  - Desired-target video constraints now affect execution instead of only persistence and job snapshot evidence.
  - ADR 325 now adds candidate and final inspection checks for profile, level metadata, bitrate tolerance, and HDR10 color signaling against the immutable snapshot.
- Follow-up:
  - Add richer HDR side-data verification and codec-specific level normalization once those contracts are represented explicitly.

## Task Record

- Motivation:
  - Make the persisted video target constraints operational in FFmpeg command construction.
- Design notes:
  - The desired graph continues to use the stable core `MediaStream` shape; execution-only stream constraints live in `VideoTranscodePolicy` to avoid reclassifying every desired graph initializer.
  - The worker maps target stream constraints to compiled desired video streams by target order, unclaimed desired stream id, and target codec.
- Test coverage summary:
  - Added an execution-builder regression proving profile, level, bitrate, and HDR/color args appear for a constrained transcoded desired stream.
  - Extended the worker non-dry-run test so the recorded FFmpeg command includes video constraints sourced from the snapshotted desired target.
  - Focused verification run in this turn: `cargo test -p revaer-media-runtime desired_graph_video_constraints_apply_to_transcoded_output_stream -- --nocapture` and `cargo test -p revaer-app media_job_runtime_executes_non_dry_run_with_injected_runner -- --nocapture` passed.
  - Full verification run in this turn: `just ci` passed, including `cargo llvm-cov --workspace --all-features --no-report`, per-package 90% line coverage checks, `coverage/lcov.info`, and release build generation.
  - UI verification run in this turn: `just ui-e2e` passed with 104 Playwright tests.
- Observability updates:
  - No new metrics or log labels; this changes deterministic command construction for existing job phase and command audit surfaces.
- Status-doc validation:
  - Rechecked ADR 317 and ADR 323. ADR 317 is updated to keep the remaining verification gap explicit.
- Risk & rollback plan:
  - Risk is codec-specific FFmpeg option compatibility. Roll back by removing the per-stream constraint args and keeping persisted constraints as non-executed snapshot data.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, ADR 317, and ADR 323.
  - No policy drift or criteria relaxation was introduced.
