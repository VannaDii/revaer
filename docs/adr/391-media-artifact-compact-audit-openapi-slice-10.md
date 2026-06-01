# Media artifact and compact audit OpenAPI slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Artifact and compact-audit HTTP endpoints now exist, but generated OpenAPI output must advertise those routes and schemas.
  - The repository serves and exports OpenAPI from `crates/revaer-api/src/openapi.rs` plus `docs/api/openapi.json`.
- Decision:
  - Add artifact and compact-audit media job diagnostic paths to the OpenAPI generator.
  - Add request/list/response schemas for artifact references and compact audit facts.
  - Regenerate `docs/api/openapi.json` through `just api-export`.
- Consequences:
  - Positive outcomes:
    - API consumers can discover the new diagnostic endpoints from the generated contract.
    - OpenAPI coverage tests now fail if these media diagnostic schemas or paths disappear.
  - Risks or trade-offs:
    - Generated OpenAPI output grows with the new schemas and path objects.
- Follow-up:
  - Wire UI job diagnostics to fetch and render artifact references and compact audit facts.

## Task Record

- Motivation:
  - Keep the API contract aligned with the artifact and compact-audit endpoint slice.
- Design notes:
  - Reused the existing media job record path helper for both new route families.
  - Added schemas with optional artifact size/content-type fields and required compact audit fact fields.
- Test coverage summary:
  - Added failing OpenAPI route/schema expectations before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api openapi_document_exports_media_routes -- --test-threads=1`.
  - Ran `just api-export`.
- Observability updates:
  - None. This updates generated API contract artifacts only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: contract consumers may rely on the new OpenAPI paths once published. Roll back by removing generator entries, regenerated JSON changes, tests, and this ADR before release.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
