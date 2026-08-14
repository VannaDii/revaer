# ADR 464: PR 171 Media Identity Error Coverage

- Status: Proposed
- Date: 2026-08-13
- Operator approval: Pending
- Context:
  - PR 171's Sonar analysis measured 61.5% new-code coverage: 32 of the 52 executable lines in `crates/revaer-data/src/media.rs` were covered.
  - The 20 uncovered lines were the `Display`, `std::error::Error::source`, and `From<std::io::Error>` paths for `MediaRootIdentityError`.
  - Coverage must be raised with executed behavior, without changing Sonar scope, exclusions, thresholds, or analyzer configuration.
- Decision:
  - Exercise every `MediaRootIdentityError` reporting variant through its public error contract, including preservation of the wrapped I/O source.
  - Keep production code, stored procedures, migrations, runtime database access, and Sonar configuration unchanged.
  - This is a nonarchitectural test-coverage correction; it introduces no new system boundary or architectural choice.
  - Final task-record status awaits the operator's decision on the status semantics proposed in ADR 461.
- Consequences:
  - Error diagnostics and source chaining are regression-tested in addition to resolver behavior.
  - The previously uncovered 20 executable lines are exercised by the Rust coverage suite.
  - No runtime behavior or operational interface changes.
- Follow-up:
  - Re-run PR 171's complete GitHub coverage and Sonar checks after this change is incorporated into the stack.

## Task Record

- Motivation:
  - Restore PR 171's strict Sonar quality gate by adding real tests for its measured changed-line coverage gap.
- Design notes:
  - The focused unit test constructs each error variant and verifies its user-facing message.
  - The I/O variant is created through `From<std::io::Error>` and checked through `std::error::Error::source`, covering both conversion and source preservation.
  - Existing database integration tests continue to exercise persisted media invariants through stored-procedure entry points; no raw runtime SQL or database boundary changed.
- Test coverage summary:
  - `just test` passed against an isolated PostgreSQL 16 container, including `media_root_identity_errors_preserve_diagnostics_and_sources` and all stored-procedure-backed `revaer-data` tests.
  - `just cov` passed every package's 90% line threshold and generated the Sonar-compatible Rust LCOV report.
  - The resulting `crates/revaer-data/src/media.rs` record reports `LF:52` and `LH:52`, with no zero-hit executable lines, raising the PR's changed executable code from 61.5% to 100% locally.
- Observability updates:
  - None. This change tests existing error diagnostics and source chaining without changing emitted telemetry.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; the test-only correction does not change the documented media contract or implementation status.
  - Updated the ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk is limited to test portability because messages include rendered paths. The fixtures use stable relative and Unix-style display values without filesystem access.
  - Rollback is to remove the focused test and this task record; production behavior is unaffected.
- Dependency rationale:
  - No dependencies were added. The test uses `std` and existing crate test support only.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No drift or contradiction was found. No operational policy, workflow, Justfile, or Sonar configuration changed.
