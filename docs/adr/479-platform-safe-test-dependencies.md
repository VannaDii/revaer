# Platform-Safe Test Dependencies

- Status: Recorded
- Date: 2026-08-15
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The first full macOS gate reached `systemstat::System::mount_at` from dashboard tests and aborted on an invalid raw-slice precondition inside the dependency before Rust could unwind.
  - The same workspace run exposed a CLI tail test that cancelled after a fixed delay without first observing the asynchronous resume-file write.
  - Both corrections already existed in later approved stack work but were required at the bottom boundary for independent validation.
- Decision:
  - Route dashboard disk usage through the existing `ApiState` test dependency seam, retaining the production `systemstat` implementation as the default and using a deterministic filesystem-aware provider in dashboard tests.
  - Run the CLI tail operation in a task, wait with a bounded deadline until the resume file is observable, then abort and verify cancellation and file contents.
- Consequences:
  - Dashboard tests no longer execute unsafe platform-specific dependency internals.
  - The tail test validates the event-to-resume-file contract instead of assuming a scheduler-dependent 200 ms completion window.
  - Production dashboard behavior is unchanged.
- Follow-up:
  - Drop duplicate hunks when the descendant API and CLI commits are replayed.
  - Keep platform and asynchronous dependencies injected or observed through deterministic boundaries in future tests.

## Task Record

- Motivation:
  - Make the bottom stack boundary pass the repository-wide test gate deterministically on the operator's macOS environment and Linux CI.
- Design notes:
  - The disk provider is a function pointer with a production default and a test-only replacement method, matching the approved descendant implementation.
  - Resume-file polling accepts only `NotFound` as transient; every other I/O error fails immediately and the total wait is capped at 30 seconds.
- Test coverage summary:
  - Re-ran `just test` with all workspace features after both corrections.
  - Both dashboard tests and `run_with_cli_executes_select_action_and_tail` passed in the complete workspace run.
  - `just ci` and all 101 `just ui-e2e` tests passed on the complete validation-foundation tree.
- Observability updates:
  - No runtime observability surface changed. Test failures now distinguish timeout, unexpected task exit, task failure, and filesystem errors.
- Status-doc validation:
  - Updated the ADR catalogue and documentation summary; no user-facing runtime documentation changed.
- Risk & rollback plan:
  - Revert both test-boundary changes together if they regress construction or cancellation semantics.
  - Do not restore direct platform probing in tests or fixed-delay cancellation without a deterministic replacement.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and the Rust scoped instructions.
  - The correction tightens the existing dependency-injection rule; no contradiction or stale reference was introduced.
