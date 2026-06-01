# Media transcoding capability-gated execution steps slice 8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Execution step generation built ffmpeg argv deterministically but did not validate whether required transcode codecs were present in the capability snapshot.
  - This could defer deterministic planning/preflight failures into later runtime execution failures.
- Decision:
  - Add capability-aware execution-step builder APIs that validate required codecs before argv generation:
    - `execute::build_execution_steps_with_capabilities`
    - `jobs::build_job_execution_steps_with_capabilities`
  - Introduce `BuildArgsError::UnsupportedCodec` for explicit unsupported capability failures.
- Consequences:
  - Positive outcomes: unsupported transcode requirements now fail deterministically during preflight step construction.
  - Risks or trade-offs: codec-name matching is currently exact-string based and relies on capability detector codec naming.
- Follow-up:
  - When profile compilation introduces explicit codec targets, extend capability checks to target codec values instead of current fixed transcode defaults.

## Task Record

- Motivation:
  - Close the capability validation gap between snapshot readiness and concrete execution-step generation.
- Design notes:
  - Video transcode now requires `libx265` capability.
  - Audio transcode now requires `aac` capability.
  - Existing non-capability-aware builders remain available for call sites that already validated capabilities elsewhere.
- Test coverage summary:
  - Added execute-level tests for reject/accept codec support in capability-aware step building.
  - Added jobs-level test for unsupported codec rejection.
  - Re-ran `cargo test -p revaer-media-runtime` (25 passed).
- Observability updates:
  - None in this library slice.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional status-doc drift found for this increment.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to media-runtime execution-step construction paths.
  - Rollback is a single commit revert of runtime code and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
