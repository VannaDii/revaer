# Media inspect probe timeout boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Media job planning runs ffprobe during source inspection before execution and destructive replacement.
  - Capability probing and verifier execution already use bounded process boundaries, but inspection still used an unbounded `Command::output()` call.
  - A malformed, hostile, or stalled media input could therefore hold the worker in inspection and accumulate unbounded probe output before any cancellation checkpoint.
- Decision:
  - Run system inspection probes through an explicit process boundary with null stdin, piped stdout and stderr, a 60 second timeout, and child termination on timeout or wait failure.
  - Bound retained stdout to 4 MiB and fail malformed when exceeded, because ffprobe JSON larger than that is not acceptable inspection input for a single source probe.
  - Bound retained stderr to a 64 KiB tail so failure diagnostics remain actionable without unbounded memory growth.
- Consequences:
  - Stalled ffprobe inspection fails closed instead of occupying a media worker indefinitely.
  - Oversized inspection stdout is rejected before JSON parsing.
  - Very large but valid ffprobe reports above the cap now require an intentional runtime policy change instead of being accepted implicitly.
- Follow-up:
  - Evaluate whether the main ffmpeg execution runner should gain a separate wall-clock policy in addition to cooperative cancellation.

## Task Record

- Motivation:
  - Close a current runtime/input safety gap found while reviewing the media transcoding stack head outside-in for inspection, execution, replacement, and capability probing boundaries.
- Design notes:
  - The implementation follows the existing capability and verifier process patterns: spawn, poll, kill, reap, join bounded reader threads, and return normalized adapter errors.
  - The public inspection adapter trait did not change.
  - The timeout is local to the system ffprobe executor; tests and injected probe adapters remain deterministic.
- Test coverage summary:
  - Added unit coverage for timed-out inspection probes and oversized stdout rejection.
  - Reran focused `revaer-media-runtime` inspect tests and formatting.
- Observability updates:
  - Existing inspection failure surfaces now include timeout and bounded stderr details in `ProbeFailed`.
- Status-doc validation:
  - Updated ADR index and mdBook summary only; app/API docs were not touched because no public app or API contract changed.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: unusually large ffprobe JSON or sources requiring more than 60 seconds for the first-frame inspection now fail planning.
  - Rollback: revert this ADR and the inspection executor change, then rerun the focused inspect tests to restore the prior unbounded behavior.
- Dependency rationale:
  - No dependencies were added; the process boundary uses only `std`.
