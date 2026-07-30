# Media data target boundary evidence

- Status: Accepted
- Date: 2026-07-30
- Context:
  - Full media inspection retains opaque data streams so generic graph and preservation logic can account for them.
  - ADR 336 made explicit desired data rows fail closed in the core compiler, but the outer API and stored-procedure regression evidence still named only chapter and attachment rows.
  - A production media service must prove the unsupported data-row contract at every authored configuration boundary, not only in the inner planner.
- Decision:
  - Extend desired-target API validation coverage so `data` rows are rejected with the same unsupported stream-kind path as `attachment` and `chapter`.
  - Extend stored-procedure-backed data-layer coverage so explicit desired `data` rows cannot be appended to immutable target versions.
  - Keep the supported authored desired stream kinds limited to `video`, `audio`, and `subtitle` until data-stream execution and verification semantics exist.
- Consequences:
  - Reviewers can see fail-closed evidence for opaque data rows at the operator-facing API and database boundaries.
  - The change does not enable data-stream target authoring; it only closes a regression-evidence gap.
- Follow-up:
  - Implement a complete data-stream desired-state schema, command contract, and verification path before allowing explicit data rows.

## Task Record

- Motivation:
  - Continue reducing the media desired-graph false-success surface from the outside in.
- Design notes:
  - The runtime/compiler behavior was already fail-closed for `StreamKind::Data`; this change adds boundary evidence without changing accepted configuration semantics.
  - API validation remains the first operator-facing guard, and stored procedures remain the authoritative runtime persistence guard.
- Test coverage summary:
  - Added `data` to API desired-target unsupported-kind validation coverage in `crates/revaer-api/src/http/handlers/media.rs`.
  - Added stored-procedure-backed desired-target append coverage for rejecting explicit `data` stream rows in `crates/revaer-data/src/media/configuration.rs`.
- Observability updates:
  - No new metrics or logs; the existing validation and database error surfaces report the rejection.
- Status-doc validation:
  - Reviewed ADR 318 and ADR 336. Their implementation-incomplete and data-target fail-closed status remains accurate.
- Risk & rollback plan:
  - Risk is limited to test-only assertion broadening around an already unsupported stream kind.
  - Roll back this ADR and the added assertions if a complete data-stream target contract lands in the same stack.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No drift or contradiction was found, and no Sonar, lint, coverage, dependency, or stored-procedure criteria were relaxed.
