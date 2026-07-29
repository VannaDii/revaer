# Media execution timeout boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Media execution and audio loudness analysis already run with null stdin, bounded stderr, and cooperative cancellation.
  - A hung child process could still keep a worker in an active command indefinitely when no cancellation is requested.
- Decision:
  - Add wall-clock timeouts to process-backed media command execution and audio loudness analysis.
  - Preserve separate failure identities for operator cancellation, non-zero command exit, and timeout.
  - Kill, reap, and join stderr readers before returning timeout failures.
- Consequences:
  - Runaway media commands fail closed instead of pinning a worker forever.
  - Long legitimate transcodes must finish within the command deadline or be retried after operator review.

## Task Record

- Motivation:
  - Close the remaining command-boundary gap where an active media subprocess could outlive all normal control flow.
- Design notes:
  - Process command execution keeps the existing cancellation path and adds a 12-hour wall-clock deadline for full transcodes.
  - Audio analysis keeps its bounded stderr parser and adds a 30-minute wall-clock deadline.
  - Timeout returns a distinct error rather than masquerading as cancellation.
- Test coverage summary:
  - Added process-runner timeout coverage proving a sleeping child is killed quickly.
  - Added audio-analysis timeout coverage proving the analyzer child is killed quickly.
- Observability updates:
  - Timeout errors retain the command boundary in the propagated failure detail.
- Status-doc validation:
  - Reviewed the media ADR index and mdBook summary for the new task record entry.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: an unusually long but healthy transcode may hit the fixed deadline.
  - Rollback: revert the timeout changes and rerun media runtime/app tests.
- Dependency rationale:
  - No dependencies were added.
