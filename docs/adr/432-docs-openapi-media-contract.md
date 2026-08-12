# ADR 432: Docs OpenAPI Media Contract

- Status: Accepted
- Date: 2026-08-12
- Context:
  - The canonical docs OpenAPI artifact must represent the media service introduced by ADR 430.
  - Its complete generated delta exceeds the 10,000-change review limit.
- Decision:
  - Update the canonical artifact in two valid layers: component schemas first, then paths and the final canonical document.
  - Require the completed canonical artifact to be byte-identical to the app-local generated copy.
- Consequences:
  - Contract changes remain reviewable and schema references exist before route definitions consume them.
  - The schema-only layer intentionally exposes unused definitions until the route layer lands.
- Follow-up:
  - Keep both tracked OpenAPI copies synchronized whenever the API contract changes.

## Task Record

- Motivation:
  - Publish a reviewable canonical API contract for the PR82 media service.
- Design notes:
  - This change contains generated OpenAPI data and task records only; runtime behavior remains in ADR 430.
- Test coverage summary:
  - Parsed both completed artifacts as JSON and verified byte equality with generator output, including media schemas and routes.
- Observability updates:
  - None; this is a generated contract artifact.
- Status-doc validation:
  - The ADR index and documentation summary include this task record.
- Risk & rollback plan:
  - Invalid or incomplete schemas can affect generated clients. Revert this artifact-only stack layer.
- Dependency rationale:
  - No dependency changes.
- Stale-policy check:
  - Reviewed `AGENTS.md` and the Rust, DevOps, and data scoped instructions. No policy drift was found.
