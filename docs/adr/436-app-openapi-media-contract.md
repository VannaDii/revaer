# ADR 436: App OpenAPI Media Contract

- Status: Accepted
- Date: 2026-08-12
- Context:
  - The app-local generated OpenAPI copy predates the media service schemas introduced by ADR 430.
  - Regenerating schemas and paths in one diff exceeds the 10,000-change review limit.
- Decision:
  - Update the app-local artifact in two valid layers: component schemas first, then paths and the final canonical document.
  - Keep the completed app-local artifact byte-identical to the canonical generated docs artifact.
- Consequences:
  - Each review remains bounded while schema references are available before route definitions consume them.
  - The schema-only layer intentionally exposes unused definitions until the route layer lands.
- Follow-up:
  - Keep the app-local and canonical docs copies byte-identical when the canonical artifact layer lands.

## Task Record

- Motivation:
  - Preserve a reviewable generated app API contract for the PR82 media service.
- Design notes:
  - This change contains generated OpenAPI data and task records only; runtime behavior remains in ADR 430.
- Test coverage summary:
  - Parsed the completed artifact as JSON and verified byte equality with generator output, including media schemas and routes.
- Observability updates:
  - None; this is a generated contract artifact.
- Status-doc validation:
  - The ADR index and documentation summary include this task record.
- Risk & rollback plan:
  - Invalid or incomplete schemas can affect contract consumers. Revert this artifact-only stack layer.
- Dependency rationale:
  - No dependency changes.
- Stale-policy check:
  - Reviewed `AGENTS.md` and the Rust, DevOps, and data scoped instructions. No policy drift was found.
