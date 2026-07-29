# Media capability probe process boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Capability refresh runs multiple `ffmpeg`, `ffprobe`, and `ffplay` probes before media jobs trust the runtime snapshot.
  - The probe executor used `Command::output()`, which waited without a timeout and retained complete stdout and stderr.
- Decision:
  - Replace `Command::output()` with an explicit process boundary that closes stdin, pipes stdout/stderr, and polls with a per-probe timeout.
  - Bound stdout and stderr collection separately, returning a deterministic malformed-output error if stdout exceeds the accepted probe budget.
  - Retain a bounded stderr tail for nonzero status and timeout diagnostics.
- Consequences:
  - Wedged or noisy media tools can no longer block capability refresh indefinitely or grow memory without bound.
  - Extremely large probe stdout now fails closed instead of being parsed partially.
- Follow-up:
  - Continue reviewing remaining media tool adapters for bounded output, cancellation, and timeout behavior.
  - Continue closing remaining media service gaps through focused stacked PRs.

## Task Record

- Motivation:
  - Move the media service closer to production correctness by sealing the capability-refresh process boundary before jobs rely on capability snapshots.
- Design notes:
  - The public `CapabilityProbeExecutor` trait remains unchanged.
  - The system executor now uses explicit `stdin`, `stdout`, and `stderr` handling, bounded readers, timeout polling, and kill/reap cleanup.
  - No new dependencies were added.
- Test coverage summary:
  - Added unit coverage for bounded stdout retention.
  - Added unit coverage for bounded stderr tail retention.
  - Added system-process coverage for noisy nonzero stderr diagnostics.
  - Added system-process coverage proving active probes time out quickly.
- Observability updates:
  - Existing media capability refresh error mapping is reused.
  - Command failures now include bounded stderr context when available.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`; no instruction drift was introduced.
- Risk & rollback plan:
  - Risk: a valid but unexpectedly huge probe output now fails capability refresh instead of being parsed.
  - Rollback: revert this ADR and the system probe executor changes.
- Dependency rationale:
  - No dependencies were added.
