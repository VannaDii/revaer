# Media compliance API and UI slice 9

- Status: Accepted
- Date: 2026-05-31
- Context:
  - `MEDIA_TRANSCODING.md` slice 9 requires API and UI exposure for the default media runtime license mode, source offer, third-party notices, SBOM, and license-excluded capabilities.
  - ADR 362 added the runtime image artifacts and labels, but operators still had to inspect the container metadata or filesystem paths manually.
- Decision:
  - Add `MediaComplianceResponse` to the shared API models and serve it from authenticated `GET /v1/media/compliance`.
  - Return stable in-image compliance artifact paths for the source offer, third-party notices, SPDX inventory/SBOM, and ExifTool exception record.
  - Add a media UI compliance panel that fetches the endpoint during refresh and displays the license mode, artifact paths, and excluded capabilities alongside the existing media management surface.
  - Alternatives considered: reading the release files dynamically at request time was deferred because this first slice exposes stable release metadata rather than file contents.
- Consequences:
  - Positive outcomes:
    - Operators can discover the runtime media license posture without shell access to the image.
    - The UI now makes the slice-9 compliance posture visible in the first media management surface.
    - The shared DTO keeps CLI/UI/API consumers on one serialized contract.
  - Risks or trade-offs:
    - The endpoint is static metadata for now; release automation still owns per-image digest source bundle generation.
    - The SBOM and inventory paths currently point at the same SPDX artifact until release packaging emits distinct per-digest artifacts.
- Follow-up:
  - Link these artifact paths to downloadable file-serving endpoints when static compliance asset serving is added.
  - Expand release automation to stamp the endpoint payload with the published image digest once image builds are release-versioned.

## Task Record

- Motivation:
  - Continue closing `MEDIA_TRANSCODING.md` slice 9 by exposing the compliance artifacts added in ADR 362 through application surfaces.
- Design notes:
  - Kept the compliance payload deterministic and dependency-free with compile-time constants.
  - Protected the endpoint with the same API-key middleware used by the rest of `/v1/media`.
  - Stored API transport state in the media feature state instead of adding UI-only fields to the shared DTO layer.
- Test coverage summary:
  - Added `media_compliance_returns_release_artifact_links`; it failed before the handler existed and passes after the endpoint payload was added.
  - Updated the media UI E2E spec to assert the license mode appears on the media page.
- Observability updates:
  - No new telemetry was added. The endpoint exposes release metadata already represented in image labels and compliance files.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: static paths drift from release image layout. Roll back by reverting the DTO, handler, route, UI panel, E2E assertion, and this ADR, then rerun focused API/UI checks.
- Dependency rationale:
  - No dependencies were added. The slice uses existing Axum/Yew/API-client infrastructure.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: no instruction drift was found for this API/UI metadata exposure.
  - Contradictions or stale references removed: none.
