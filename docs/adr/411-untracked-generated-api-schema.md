# Untracked Generated API Schema

- Status: Accepted
- Date: 2026-08-04
- Context:
  - `tests/support/api/schema.ts` was an 8,532-line generated TypeScript artifact tracked as authored source even though the E2E workflow already regenerated it from `docs/api/openapi.json`.
  - Keeping the generated output in Git inflated review and analysis scope and allowed the committed artifact to drift from the OpenAPI document or generator lockfile.
- Decision:
  - Ignore and stop tracking only `tests/support/api/schema.ts`.
  - Add `just api-test-client` as the canonical test-time generation entrypoint. It uses `npm ci` against `tests/package-lock.json`, invokes the existing `gen:api-client` script, and rejects empty output.
  - Run an index-and-ignore guardrail plus fixture tests from `just policy` so the generated file cannot silently return as tracked source.
  - Retain `openapi-typescript` and the committed OpenAPI document as the existing generator and input rather than introducing a second generation path.
- Consequences:
  - Reviews and static analysis no longer treat generated API client output as authored source.
  - E2E setup performs an exact lockfile installation before generation, so it requires registry access when dependencies are not already available in the npm cache.
- Follow-up:
  - Keep OpenAPI changes covered by `just api-test-client` and the existing E2E suite.
  - Preserve the generated-file guardrail when test tooling is reorganized.

## Task Record

- Motivation:
  - Remove generated-source noise before the quality-baseline deliverable while preserving an outside-in API contract generated from the actual OpenAPI document.
- Design notes:
  - The generated file remains at its established import path, so E2E callers require no source changes.
  - The guardrail verifies Git-index state independently from ignore state and tests both failure modes in an isolated temporary repository.
- Test coverage summary:
  - Run the generated-source guardrail against the real worktree.
  - Run fixture tests proving tracked and non-ignored schemas are rejected.
  - Generate the API schema twice from a clean lockfile install and compare hashes, then run focused policy and instruction-drift gates.
- Observability updates:
  - No runtime observability surfaces change. Guardrail failures identify whether tracking or ignore state violated policy.
- Status-doc validation:
  - Rechecked the test README and updated its generation instructions. Runtime status and roadmap documents are unaffected.
- Risk & rollback plan:
  - The primary risk is a missing generated file before TypeScript execution. `just ui-e2e` now calls the explicit generation recipe and fails on empty output. Rollback is to revert this ADR and lifecycle change together.
- Dependency rationale:
  - No dependency was added. The existing exact `openapi-typescript` lockfile resolution remains authoritative.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md` because the Justfile test lifecycle changed.
  - Drift was found in the absence of an explicit untracked-generated-source rule; the devops instruction now records the required lifecycle and guardrail.
