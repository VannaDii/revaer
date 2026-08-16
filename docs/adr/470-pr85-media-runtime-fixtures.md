# ADR 470: PR 85 Media Runtime Fixture Correction

- Status: Proposed
- Operator approval: Pending
- Date: 2026-08-13
- Context:
  - PR 85 retained test fixtures for older stream-constraint and inspection field names.
  - The stale fixtures prevented the intermediate stack boundary from compiling even though later commits happened to repair the same construction sites.
- Decision:
  - Update the fixtures to initialize `max_bitrate_bps` and `max_bit_rate` exactly as the current types require.
  - Treat this as a nonarchitectural integration correction; no runtime behavior, public contract, or architectural boundary changes.
- Consequences:
  - PR 85 compiles and tests independently instead of relying on a descendant commit.
  - The fixtures remain explicit when fields are absent.
- Follow-up:
  - Keep PR 85 check, lint, and focused video-level tests green after the stack replay.

## Task Record

- Motivation:
  - Restore independent buildability at the earliest stack commit that introduced the stale fixtures.
- Design notes:
  - The correction uses `None` and therefore preserves the original test scenario.
  - No production code or scanner criterion changes.
- Test coverage summary:
  - Ran `just fmt`, `just check`, `just lint`, and the focused video-level tests.
  - Ran `just clean-test-fixtures` and verified no media files remained.
- Observability updates:
  - No logging, tracing, metrics, health, or event-surface changes.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; no product or operator documentation changes are required.
- Risk & rollback plan:
  - Risk is limited to an incorrect fixture shape; compilation and focused tests validate the exact type contract.
  - Rollback would restore the compilation failure and is not appropriate while these fields remain in the type.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or contradiction was found.
