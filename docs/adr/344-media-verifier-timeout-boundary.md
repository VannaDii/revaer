# Media verifier timeout boundary

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Candidate verification runs `ffmpeg` and `ffplay` after a candidate already exists but before destructive replacement is prepared.
  - The verifier process boundary closed stdin, discarded stdout, retained only bounded stderr, and supported cooperative cancellation, but an uncancelled verifier process could still wait indefinitely.
- Decision:
  - Add an explicit wall-clock timeout to the system verifier executor.
  - Kill and reap a timed-out verifier child, drain the bounded stderr tail, and report a stable timeout failure detail.
  - Keep the public verifier trait unchanged and use a conservative default timeout for real full-stream verification.
- Consequences:
  - A wedged media tool cannot block candidate safety verification forever.
  - Legitimate very long verification work now has an upper bound. If operators need a different bound later, that should be exposed as a versioned policy field instead of removing the timeout.
- Follow-up:
  - Continue closing the remaining desired-state verification gaps called out in the consolidated media foundation record.

## Task Record

- Motivation:
  - Move the media service closer to production correctness by making every selected candidate verifier command bounded by cancellation, output limits, and wall-clock time.
- Design notes:
  - The timeout lives inside `SystemVerificationExecutor`, preserving the injected `VerificationExecutor` contract used by tests and worker orchestration.
  - Timeout failures reuse the existing failed-verification surface and include the bounded stderr tail when available.
  - No new dependencies were added.
- Test coverage summary:
  - Added a system-process regression proving a long-lived verifier child is killed quickly on timeout.
  - The regression also verifies bounded stderr context is preserved in the timeout detail.
- Observability updates:
  - Existing persisted verification checks and media-job failure metrics continue to receive `verification_executor` failures for process-boundary errors.
  - Timeout details include a stable `timed out after` diagnostic string for operator triage.
- Status-doc validation:
  - Rechecked `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/devops.instructions.md` for this Rust/runtime/doc change.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: an unusually large or slow full-stream verifier may now fail by timeout rather than continuing indefinitely.
  - Rollback: revert this ADR and the verifier timeout change, then re-run the focused verifier tests before restoring the previous behavior.
- Dependency rationale:
  - No dependencies were added.
