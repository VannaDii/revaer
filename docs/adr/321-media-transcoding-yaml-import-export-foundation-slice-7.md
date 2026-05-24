# Media transcoding YAML import and export foundation slice 7

- Status: Accepted
- Date: 2026-05-23
- Context:
  - Media profiles and jobs had DB/runtime/API foundations but lacked a portable exchange format and import validation flow.
  - `MEDIA_TRANSCODING.md` requires versioned YAML import/export with semantic validation and safe defaults.
- Decision:
  - Add media YAML export, validate, and apply operations via the media app facade.
  - Keep parse/validation logic in `revaer-app::media` and expose API endpoints:
    - `GET /v1/media/export`
    - `POST /v1/media/imports/validate`
    - `POST /v1/media/imports/apply`
  - Force imported profiles to `dry_run_only=true` on apply.
- Consequences:
  - Positive outcomes: operators can export portable media profiles, run semantic checks, and apply validated payloads safely.
  - Risks/trade-offs: current validation is foundation-level (version/key/root checks) and needs expansion for full plan contradictions.
- Follow-up:
  - Extend validator for deeper policy/target contradiction rules from `MEDIA_TRANSCODING.md`.
  - Add UI import/export flows and E2E coverage for media screens.

## Task Record

- Motivation:
  - Deliver required portable configuration exchange and import safety checks for media transcoding rollout.
- Design notes:
  - Added media YAML request/response DTOs in `revaer-api-models`.
  - Extended `revaer-api::app::media::MediaFacade` with `media_yaml_export`, `media_yaml_validate`, and `media_yaml_apply`.
  - Implemented YAML parse/serialize and semantic checks in `revaer-app::media` using existing `serde_yaml` dependency.
  - Added HTTP handlers and routes under `/v1/media/*` import/export paths.
- Test coverage summary:
  - Kept existing media handler tests passing and reran targeted API tests.
  - Reran `just lint` and `just ui-e2e` after migration/API changes.
- Observability updates:
  - No new telemetry in this slice.
- Status-doc validation:
  - Updated ADR index and docs summary.
- Stale-policy check:
  - Reviewed `AGENTS.md` and existing API handler patterns; no policy suppressions added.
- Risk & rollback plan:
  - Roll back by reverting media YAML DTO/facade/service/handler/route changes.
- Dependency rationale:
  - No new dependencies introduced; reused existing `serde_yaml` in `revaer-app`.
