# ADR 471: PR 126 Chapter Ordering Predicate Correction

- Status: Proposed
- Operator approval: Pending
- Date: 2026-08-13
- Context:
  - PR 126 introduced a chapter ordering predicate with a logically redundant comparison.
  - Its concurrent append test also relied on an implicit tuple-to-array conversion rejected by Rust 1.96 strict linting.
  - The lint failures prevented affected intermediate stack boundaries from passing independently.
- Decision:
  - Reject a new chapter when its start precedes the previous chapter's end.
  - Convert the pair of concurrent results into the fixed-size array explicitly.
  - Add focused cases proving overlapping chapters fail and exactly abutting chapters succeed.
  - Treat this as a nonarchitectural expression of the existing non-overlap invariant.
- Consequences:
  - The predicate and concurrent result collection are lint-clean and preserve their intended contracts.
  - Adjacent chapters remain valid.
- Follow-up:
  - Keep the focused chapter conversion tests and full stack lint gate green.

## Task Record

- Motivation:
  - Repair the earliest stack boundary containing both strict lint findings.
- Design notes:
  - For an already valid previous chapter, comparing against its end subsumes comparing against its start.
  - The explicit tuple conversion preserves both concurrent results and their order.
  - No persistence, API, runtime architecture, or scanner criteria change.
- Test coverage summary:
  - Added explicit overlapping and abutting chapter cases.
  - Ran formatting, lint, focused tests, fixture cleanup, and retained-media verification.
- Observability updates:
  - No logging, tracing, metrics, health, or event-surface changes.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; no user-facing documentation changes are required.
- Risk & rollback plan:
  - The risk is accepting an invalid ordering edge; the focused boundary cases cover both sides.
  - Rollback would restore the redundant predicate and lint failure.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No policy drift or contradiction was found.
