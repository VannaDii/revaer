# Media command process boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Media job command execution is the production boundary that invokes transcoder tools.
  - Verification commands already closed stdin and bounded stderr handling, but primary execution commands inherited the parent process streams and only reported exit status.
  - Long-running media tools can emit large diagnostics and must not consume service stdin or write unbounded output through the worker process.
- Decision:
  - Run process-backed media commands with stdin closed, stdout discarded, and stderr captured through a bounded reader.
  - Preserve cooperative cancellation by killing and reaping the active child, then joining the stderr reader before returning the cancellation result.
  - Include bounded stderr in command-failure errors so failed jobs retain actionable diagnostics without unbounded memory growth.
  - Keep the change inside the injected process runner so command construction remains deterministic and test runners can continue to inject their own behavior.
- Consequences:
  - Media workers no longer expose their process streams to transcoder children.
  - Failed command reports have stronger operator evidence while staying memory-bounded.
  - Direct app tests still require a working local native torrent toolchain because the app crate builds that dependency before filtered tests run.
- Follow-up:
  - Continue closing remaining runtime verification gaps through the stacked media PRs.
  - Re-run remote PR checks after pushing the restacked branches.

## Task Record

- Motivation:
  - Move the media service closer to production readiness by sealing the process boundary used for destructive media command execution.
- Design notes:
  - `ProcessCommandRunner` now configures child stdio explicitly instead of inheriting parent handles.
  - Stderr is read on a dedicated thread, retained only up to a fixed diagnostic limit, and drained to avoid blocking noisy child processes.
  - Cancellation still terminates and reaps the child process before reporting cancellation.
- Test coverage summary:
  - Added focused runtime tests for bounded stderr capture, noisy stderr truncation, and existing cancellation behavior.
  - Ran `cargo fmt --all --check`.
  - Ran `cargo --config 'build.rustflags=["-Dwarnings"]' test -p revaer-media-runtime process_command_runner --all-features`.
  - Ran `cargo --config 'build.rustflags=["-Dwarnings"]' test -p revaer-media-runtime execute --all-features`.
  - Ran `cargo --config 'build.rustflags=["-Dwarnings"]' clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions`.
  - Attempted focused `revaer-app` media job runtime tests, but the local native torrent dependency failed during C++ header compilation before app tests could run.
- Observability updates:
  - Command failures now carry bounded stderr detail in the runtime error payload.
- Status-doc validation:
  - Updated `docs/adr/index.md` and `docs/SUMMARY.md` so the task record is discoverable.
- Risk & rollback plan:
  - Risk is limited to process-backed command execution. If command invocation regresses, revert this ADR and the runner/error-shape changes to restore the prior process spawning behavior.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md` for applicable quality, process-boundary, and scanner policy.
  - No stale or contradictory policy was found for this change.
