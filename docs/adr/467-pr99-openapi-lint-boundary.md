# ADR 467: PR 99 OpenAPI Lint Boundary Correction

- Status: Proposed
- Date: 2026-08-13
- Operator approval: Pending; this is a nonarchitectural corrective task record awaiting ADR 461 status semantics.
- Context:
  - PR 99 first increases the OpenAPI media schema inventory enough for the existing test function to exceed the repository's 100-line Clippy limit.
  - A later stack commit extracts the same inventory into a module-level constant, leaving PR 99 independently red even though the integrated leaf passes.
- Decision:
  - Move the existing behavior-preserving schema-name extraction to PR 99, the first affected integration boundary.
  - Keep the schema inventory and assertions unchanged.
  - Remove the redundant extraction from the later descendant while replaying the stack.
- Consequences:
  - PR 99 and its descendants satisfy the same strict lint policy independently.
  - The correction changes no API, schema, runtime behavior, or architectural boundary.
- Follow-up:
  - Replay the single linear stack and verify every current-head GitHub check.

## Task Record

- Motivation:
  - Current-head GitHub validation found `clippy::too_many_lines` in `openapi_document_exports_media_schemas` on PR 99.
- Design notes:
  - The extraction matches the already-integrated descendant implementation and introduces no new abstraction.
- Test coverage summary:
  - Run formatting, `revaer-api` lint, and the focused OpenAPI schema test before replay; run the complete leaf gates after replay.
- Observability updates:
  - None; test organization only.
- Status-doc validation:
  - The correction does not alter the documented media contract.
- Risk & rollback plan:
  - Risk is limited to accidentally changing the expected schema list; the focused test and diff review preserve exact membership.
  - Roll back the extraction if it changes behavior, then use an equivalent sub-100-line organization.
- Dependency rationale:
  - No dependency added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`; no drift or contradiction was found.
