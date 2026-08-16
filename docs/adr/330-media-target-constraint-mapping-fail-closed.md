# Media Target Constraint Mapping Fail-Closed

- Status: Accepted
- Date: 2026-07-22
- Context:
  - Desired target rows can carry video and audio technical constraints that must be applied to the compiled desired graph before preflight and verified after execution.
  - A constrained target row that cannot be mapped to a compiled desired stream must not be treated as a no-op, because that can omit encoder arguments or skip verification for an authored policy.
- Decision:
  - Constrained video and audio target rows now fail closed when their stream key, kind, and codec cannot be mapped to a compiled desired stream.
  - Preflight policy construction returns `media_job_target_constraint_stream_unmatched` before planning any execution steps.
  - Runtime verification records an explicit unmatched target-stream constraint failure instead of reporting an empty constraint set.
  - The alternative of retaining silent skips was rejected because it can make a malformed target appear satisfied.
- Consequences:
  - Positive: Authored target constraints can no longer disappear between target compilation, FFmpeg argv construction, and verification.
  - Risk: Existing malformed desired-target rows that previously passed as unconstrained jobs will now fail planning or verification until the target stream codec/kind selectors are corrected.
- Follow-up:
  - Continue closing ADR 317's remaining codec-level and HDR side-data semantic gaps with similarly fail-closed checks.

## Task Record

- Motivation:
  - Move the media transcoding runtime closer to production correctness by eliminating a fail-open constraint-mapping path.
- Design notes:
  - Reused the existing desired-target stream mapper for both video and audio constraints.
  - Kept the runtime error as a stable machine code and placed stream-key details in verification check fields.
  - No source-level lint suppressions or Sonar relaxations were added.
- Test coverage summary:
  - Added focused tests proving unmatched constrained video target rows fail preflight policy construction and unmatched constrained audio target rows produce explicit verification mismatch data.
- Observability updates:
  - Verification mismatch records now include `target_stream:<stream_key>:audio_constraint=mapped` or `target_stream:<stream_key>:video_constraint=mapped` for unmapped constrained rows.
- Status-doc validation:
  - Rechecked `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, ADR 317, and ADR 329.
  - Updated ADR indexes to include this task record.
- Risk & rollback plan:
  - Roll back this commit if legitimate target rows fail due to an unexpected compiler mapping bug, then add a regression fixture for that stream shape before reinstating fail-closed behavior.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Reviewed root and scoped instruction files for Rust, DevOps, and Sonar rules touched by this change.
  - No stale or contradictory policy references were changed.
