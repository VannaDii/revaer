# CLI tail resume test stabilization slice ci

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The full local CI gate exposed a timing-sensitive CLI tail test failure where the fixed cancellation deadline fired before the first stream event wrote the resume file.
  - Focused runs passed, which confirmed the tail behavior was correct and the test was racing wall-clock scheduling under suite load.
- Decision:
  - Wait for the resume file write as the test's observable success condition before cancelling the tail task.
  - Apply the same deterministic pattern to the setup-tail coverage that used the same fixed cancellation deadline.
- Consequences:
  - Positive outcomes:
    - Tail command-path tests now verify resume persistence without depending on a 200 ms scheduling window.
    - The production tail retry and streaming behavior remains unchanged.
  - Risks or trade-offs:
    - A broken resume-write path now waits up to five seconds before failing, trading a clearer assertion for a bounded delay on real regressions.
- Follow-up:
  - Keep future infinite-stream command tests event-driven rather than cancellation-deadline driven.

## Task Record

- Motivation:
  - Keep the media branch handoff gates reliable while preserving real CLI tail coverage.
- Design notes:
  - Added test-only helpers that poll for the resume file and then abort the spawned tail task.
  - Treated an unexpected tail exit as a test failure because the tail command should continue running after the mocked event stream completes.
- Test coverage summary:
  - Reproduced the full-gate failure in `just ci`.
  - Verified focused coverage with `cargo test -p revaer-cli --lib cli::tests::run_with_cli_executes_select_action_and_tail -- --nocapture --test-threads=1`.
  - Verified the matching setup-tail test with `cargo test -p revaer-cli --lib commands::setup::tests::handle_tail_writes_resume_file -- --nocapture --test-threads=1`.
- Observability updates:
  - None. This is test stabilization only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and the existing ADR/task-record requirements.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: the five-second wait can add delay if resume persistence regresses. Roll back by restoring the prior timeout assertions, though that reintroduces the race.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
