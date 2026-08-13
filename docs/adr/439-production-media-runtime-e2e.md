# Production media runtime end-to-end verification

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Database-backed runtime tests injected fake inspectors, command runners, and verification executors.
  - Real-media fixture tests exercised FFmpeg and ffprobe without crossing the production job-runtime, stored-procedure, replacement, and terminal-persistence boundary.
- Decision:
  - Add an ignored integration test that constructs `MediaJobRuntime::new`, provisions disposable PostgreSQL state through production stored procedures, generates a temporary H.264 Matroska source, and executes a metadata rewrite through the real FFmpeg and ffprobe adapters.
  - Require the test to prove candidate and final graph verification, atomic source replacement, completed execution phases, compact audit persistence, and workspace cleanup.
  - Run the test from the canonical `just test-media-conversion` recipe after fixture-catalog verification.
- Consequences:
  - The media conversion gate now fails when production adapters do not compose correctly, even if their injected unit tests remain green.
  - The gate requires PostgreSQL, FFmpeg with `libx264`, and ffprobe.
- Follow-up:
  - Extend the same production boundary with additional representative profiles when they expose adapter interactions not covered by focused tests.

## Task Record

- Motivation:
  - Validate that a claimed media job can traverse the actual production runtime into a verified, persisted, and cleaned-up terminal replacement.
- Design notes:
  - The generated source is small and deterministic in structure, while assertions target observable contracts instead of encoder-specific bytes.
  - Playback probing is disabled for this headless test profile; mux validation, full-stream decode, keyframe seek, graph verification, and final reinspection remain enabled.
- Test coverage summary:
  - Run the focused production runtime test, `just test-media-conversion`, `just ci`, and `just ui-e2e`.
  - `just cov` accumulates the ignored external-tool test into the workspace profile before enforcing package thresholds and producing Sonar LCOV.
- Observability updates:
  - The CI conversion report includes the production-runtime boundary outcome and fails its overall summary when that boundary fails.
  - API-only E2E coverage now skips the unrelated UI startup path when UI coverage is explicitly disabled, preventing a cold UI build from withholding API coverage evidence and the downstream Sonar scan.
- Status-doc validation:
  - Re-checked `README.md`, `MEDIA_TRANSCODING.md`, and operator-facing workflow documentation. No capability-status wording required a change; the ADR indexes and scoped agent instructions were updated.
- Risk and rollback plan:
  - The additional gate increases CI runtime and may reveal missing native-tool capabilities. Restore the required tools or fix the production path; rollback consists of reverting this test and recipe change together only if the operator explicitly removes the boundary requirement.
- Dependency rationale:
  - No dependencies added. The test uses the repository's existing PostgreSQL harness and required FFmpeg toolchain.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, the Justfile, and the PR workflow. Updated the Rust, UI, and DevOps instructions and added structural guardrail regression coverage to keep the canonical media recipe and workflow requirements aligned.
