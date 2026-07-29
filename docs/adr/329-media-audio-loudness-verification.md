# Media Audio Loudness Verification

- Status: Accepted
- Date: 2026-07-22

## Motivation

Desired audio policy could request dialog normalization and speech dynamic-range shaping, and runtime command construction emitted FFmpeg filters for those policies. Candidate and final verification still accepted graph-compatible output based on inspected codec, bitrate, and sample rate only, so a failed or ineffective filter could pass replacement without measured LUFS/LRA evidence.

## Design Notes

- Add an injected audio-analysis adapter to the media job runtime. Bootstrap wires the production adapter to FFmpeg, while tests inject deterministic measurements.
- Measure policy-constrained audio streams with FFmpeg's `ebur128=peak=true` filter after candidate and final full-inspection graph checks.
- Fail candidate or final verification if dialog-normalized output is outside `-17.0..=-15.0` integrated LUFS or lacks a true peak at or below `-1.0` dBFS.
- Fail speech dynamic-range output if measured loudness range is above `12.0` LU.
- Persist the mismatch through the existing `candidate_audio_constraints` and `final_audio_constraints` verification checks instead of adding an unbounded metric label or side channel.
- Treat analyzer spawn, execution, or parser failures as verification failures. A candidate cannot replace the source when required measurement evidence is absent.
- Keep this slice scoped to the implemented `dialog-normalized` and `speech` policies. It does not claim full codec-level semantics, arbitrary loudness profiles, or all remaining technical media constraints.

## Test Coverage Summary

- `cargo test -p revaer-app media_job_runtime_rejects_candidate_audio_loudness_mismatch -- --nocapture`
- `cargo test -p revaer-app ebur128_summary_parser_reads_loudness_range_and_peak -- --nocapture`
- `cargo test -p revaer-app ebur128_summary_parser_requires_integrated_loudness_and_range -- --nocapture`
- `cargo test -p revaer-app audio_measurement_policy_checks_fail_missing_peak_and_excess_lra -- --nocapture`
- `cargo test -p revaer-app system_audio_analyzer_reports_spawn_failure -- --nocapture`
- `cargo test -p revaer-app media_job_runtime -- --nocapture`
- `cargo check -p revaer-app --all-targets`
- `just ci` rerun reached the strict per-package coverage threshold loop with `revaer-app` passing; the final release build was rechecked with `just build-release` after the CI session ended before returning its final exit payload.
- `cargo llvm-cov report --package revaer-app --summary-only --fail-under-lines 90` reported `90.24%` line coverage.
- `cargo llvm-cov report --package revaer-media-runtime --summary-only --fail-under-lines 90` reported `90.98%` line coverage.
- `just ui-e2e` passed 104 Playwright tests.

## Observability Updates

- No new metric labels were added.
- Required audio measurement failures are recorded as durable job verification checks with stable expected and actual values.
- Analyzer failures surface as `analysis_failed` with the process or parser error in the existing verification-check detail field.

## Status-Doc Validation

- Rechecked `MEDIA_TRANSCODING.md`, ADR 317, and ADR 328 for stale claims about measured loudness verification.
- ADR 317 is updated to narrow the open gap from all measured LUFS/LRA verification to remaining broader codec and technical-property semantics after this implemented policy slice.
- ADR 328 remains the historical command-policy record and now points to this follow-up verification ADR.

## Risk & Rollback Plan

- Risk: FFmpeg `ebur128` output formatting changes could cause parser failures and fail closed for policy-constrained jobs.
- Risk: Existing files with policy rows may fail verification if their real output does not meet the documented loudness contract.
- Roll back by reverting this runtime adapter wiring and ADR update. Jobs with policy fields would return to command-policy-only verification, so rollback should be temporary and operator-visible.

## Dependency Rationale

- No new dependency was added. The implementation uses the existing FFmpeg runtime process boundary and Rust standard-library process execution behind an injected adapter.

## Stale-Policy Check

- Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, `justfile`, `.github/workflows/pr.yml`, `.github/workflows/sonar.yml`, and `sonar-project.properties`.
- No policy relaxation was introduced. No Sonar source, test, issue, coverage, duplication, binary, SCM, analyzer, dependency-analysis, or quality-gate criteria were weakened.
