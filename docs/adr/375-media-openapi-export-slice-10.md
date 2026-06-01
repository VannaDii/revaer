# Media OpenAPI export slice 10

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media HTTP routes were implemented, but the embedded and generated OpenAPI document did not describe any `/v1/media/*` endpoints.
  - The media plan requires API and OpenAPI coverage for profiles, jobs, operations, violations, capabilities, compliance, and YAML import/export.
- Decision:
  - Add an OpenAPI document augmentation layer that injects the currently implemented media routes and shared DTO schemas into the embedded document.
  - Regenerate `docs/api/openapi.json` through the Justfile `api-export` recipe.
  - Add an OpenAPI unit test that verifies all implemented media routes and component schemas are exported.
- Consequences:
  - Positive outcomes:
    - API consumers can discover the current media surface from `/docs/openapi.json` and the generated artifact.
    - Future media route additions have a targeted regression test location.
  - Risks or trade-offs:
    - The media OpenAPI section remains hand-authored and must stay aligned as the remaining plan endpoints are added.
    - Regenerating the canonical JSON artifact can create broad diff noise because object keys are serialized canonically.
- Follow-up:
  - Extend the OpenAPI augmentation when target, policy, compatibility, retention, discovery, planning, and dry-run override endpoints land.

## Task Record

- Motivation:
  - Continue slice 10 by making the implemented media HTTP surface visible in the OpenAPI export.
- Design notes:
  - Kept generation deterministic by augmenting the parsed embedded document before serving or exporting it.
  - Described only implemented routes to avoid documenting unavailable behavior.
- Test coverage summary:
  - Added a failing OpenAPI export test for media routes and DTO schemas, then implemented the augmentation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-api openapi_document_exports_media_routes -- --test-threads=1`.
  - Ran `just api-export`.
- Observability updates:
  - None. This changes API contract metadata only.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: schema details can drift if DTO fields change without updating OpenAPI helpers. Roll back by removing the augmentation, generated JSON changes, test, and this ADR.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
