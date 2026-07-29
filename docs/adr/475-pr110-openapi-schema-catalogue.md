# ADR 475: PR 110 OpenAPI Schema Catalogue Correction

- Status: Proposed
- Operator approval: Pending
- Date: 2026-08-14
- Context:
  - PR 99 now introduces a bounded media schema-name catalogue to satisfy strict function-complexity analysis.
  - PR 110 originally introduced a second constant with the same name while extending the schema assertions for job phase read models.
- Decision:
  - Remove the superseded earlier declaration at PR 110 and retain the updated catalogue adjacent to the schema assertion.
  - Preserve every schema name required at the PR 110 boundary.
  - Treat this as a nonarchitectural integration correction.
- Consequences:
  - The test module has one canonical schema-name catalogue and compiles without duplicate definitions.
  - PR 99 remains independently lint-clean, while PR 110 extends the same test contract.
- Follow-up:
  - Keep OpenAPI schema tests, strict linting, and generated schema guardrails green after replay.

## Task Record

- Motivation:
  - Restore independent compilation at the first descendant where the earlier lint correction overlaps later schema work.
- Design notes:
  - The retained catalogue includes the phase list and response schemas introduced by PR 110.
  - No API route, generated schema, runtime behavior, architecture, or scanner criterion changes.
- Test coverage summary:
  - Run formatting, strict linting, OpenAPI tests, generated-schema checks, full CI, and UI E2E.
- Observability updates:
  - No logging, tracing, metrics, health, or event-surface changes.
- Status-doc validation:
  - Rechecked the ADR catalogue and documentation summary; no user-facing documentation changes are required.
- Risk & rollback plan:
  - Risk is omission of a schema assertion; the retained list and OpenAPI test fail closed on missing entries.
  - Rollback would restore the duplicate-definition compilation failure.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - No policy drift or contradiction was found.
