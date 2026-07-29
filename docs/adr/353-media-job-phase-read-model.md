# Media job phase read model

- Status: Accepted
- Date: 2026-07-29
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Media jobs already persist ordered lifecycle phases through `media_job_phase_append_v1`.
  - Operators and API clients could inspect operations, violations, plan reasons, checks, artifacts, and compact audits, but not the persisted phase timeline.
  - The missing read path made job lifecycle state less auditable and forced callers to infer phase progress from other records.
- Decision:
  - Use the normalized schema's `media_job_phase_list_v1` stored-procedure read model for ordered
    phase rows; do not reintroduce the obsolete pre-attempt function definition.
  - Expose the phase list through the runtime media store, app media facade, HTTP handler, router, and shared API response models.
  - Document `GET /v1/media/jobs/{media_job_public_id}/phases` as a read-only OpenAPI route.
    Phase mutation remains claim-fenced and worker-internal.
  - Move `ApiState` tests into a test-only submodule so affected-crate Clippy can run with the repository's strict `items_after_test_module` rule.
  - Keep PR UI E2E coverage deterministic by making the workflow shard the `ui-chromium` Playwright project explicitly through the Justfile's validated project selector, without running the API projects as Playwright dependencies.
  - Add a dedicated PR API E2E coverage job so API route coverage stays mandatory while UI shards remain UI-only and deterministic. Construct both database client URLs from the same run-scoped credentials as that job's Postgres service.
  - Count `/media` as a required UI route now that PR UI sharding runs the media page test directly instead of relying on broad dependency-project execution.
  - Keep the media UI smoke assertions independent of pre-existing catalog seed rows; the test now verifies empty catalogs are attached and then creates its own compatibility target and policy for visible readback coverage.
- Consequences:
  - Clients can read persisted phase history directly through the public API.
  - The read path stays consistent with the existing job record endpoints and stored-procedure boundary.
  - API callers cannot append or overwrite worker-owned lifecycle evidence.
  - The PR includes a small unrelated test-layout cleanup required by strict local linting.

## Task Record

- Motivation:
  - Close the review gap where worker-owned phase mutation existed without public phase readback.
- Design notes:
  - The SQL function joins from public job id through attempts to phase evidence. The API projects
    the stable phase fields while the database retains attempt metadata for internal consumers.
  - Response DTOs use explicit phase fields rather than leaking storage rows across crates.
  - The route is GET-only. The existing application facade keeps claim-generation-fenced phase
    writes for worker orchestration, not HTTP callers.
  - The API E2E job derives its database URLs from the run id, run attempt, and job identity used by `POSTGRES_PASSWORD`; mutable repository variables cannot drift from the service credential.
- Test coverage summary:
  - Database coverage proves ordered phase rows retain index, name, status, details, and creation
    time through the stored-procedure read model.
  - API coverage proves the GET route returns the typed phase list and the default facade fails
    closed instead of manufacturing an empty success.
  - Router coverage proves POST requests to every evidence collection, including phases, fail with
    `405 Method Not Allowed` so worker-owned mutations are not exposed over HTTP.
  - OpenAPI route and schema tests cover the read-only route and generated response models.
  - Runtime and app tests cover database-error propagation and phase-response mapping.
  - PR workflow coverage separates UI-only shards from API route coverage, retains both artifacts,
    and evaluates them together in the aggregate gate.
  - Workflow mutation tests reject a missing API dependency, a non-failing API artifact upload,
    and an aggregate that downloads the wrong API artifact.
  - Node and Playwright execution stays behind `scripts/with-node.sh` so NVM's pinned Node version
    is used locally and in CI.
- Observability updates:
  - No new telemetry was added; this change exposes existing persisted job phase state for operator inspection.
- Status-doc validation:
  - Updated `docs/adr/index.md`, `docs/SUMMARY.md`, and `docs/api/openapi.json`.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: the first API E2E draft used repository database URL variables despite provisioning run-scoped service credentials.
  - Contradictions or stale references removed: the API E2E URLs now use the exact service identity required by the credential-coherence rule.
- Risk & rollback plan:
  - Risk: clients may begin depending on phase order and field names once the GET endpoint is published.
  - Rollback: revert the facade methods, handler, route, OpenAPI export, and API model additions;
    the existing phase read and append procedures remain independent.
- Dependency rationale:
  - No dependencies were added.
