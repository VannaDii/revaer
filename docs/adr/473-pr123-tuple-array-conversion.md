# ADR 473: PR 123 Tuple Array Conversion Correction

- Status: Proposed
- Operator approval: Pending
- Date: 2026-08-14
- Context:
  - PR 123 constructed a two-element expected argument array directly from tuple bindings.
  - Rust 1.96 strict linting identifies that expression as an implicit tuple-to-array conversion.
- Decision:
  - Convert the tuple explicitly with `Into` before comparing command argument windows.
  - Treat this as a nonarchitectural lint correction with unchanged test semantics.
- Consequences:
  - PR 123 and its descendants pass the tuple-array lint at the introduction boundary.
  - The expected array type is explicit and remains readable.
- Follow-up:
  - Keep the media runtime tests and full lint gate green after replay.

## Task Record

- Motivation:
  - Restore independent lintability at the first stack boundary containing the finding.
- Design notes:
  - The explicit conversion produces the same two borrowed string elements in the same order.
  - No production code, runtime behavior, public interface, or scanner criterion changes.
- Test coverage summary:
  - Run formatting, lint, media runtime tests, fixture cleanup, and retained-media verification.
- Observability updates:
  - No logging, tracing, metrics, health, or event-surface changes.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; no user-facing documentation changes are required.
- Risk & rollback plan:
  - Risk is limited to test expression typing; compilation and the existing assertion validate equivalence.
  - Rollback would restore the strict lint failure.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or contradiction was found.
