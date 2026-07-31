# Media Metadata Rewrite Fail-Closed

- Status: Accepted
- Date: 2026-07-22
- Context:
  - Full inspection retains container and stream metadata, but at the time of this ADR the desired-target model did not yet define arbitrary metadata keys, values, preservation rules, rewrite rules, or post-replacement verification semantics. ADR 373 later adds target-level exact container metadata replacement through persisted desired rows while keeping contextless arbitrary rewrite operations fail-closed.
  - The legacy single-operation FFmpeg command path treated `MetadataRewrite` as executable by passing `-map_metadata -1`, which removes container metadata instead of reconciling it to a verified desired state.
  - The media contract forbids silently dropping metadata.
- Decision:
  - Reject `MetadataRewrite` command construction with a stable `UnsupportedMetadataRewrite` preflight error until a full desired metadata contract exists.
  - Surface the failure as `preflight_build_unsupported_metadata_rewrite` so operators and tests can distinguish the unsupported contract from missing codec, muxer, or malformed operation failures.
  - Keep the operation kind for persisted historical plans and summaries, but do not allow it to materialize a destructive FFmpeg command.
  - Alternatives considered:
    - Continuing to strip metadata with `-map_metadata -1` was rejected because it destroys retained facts without a desired-state rule or verification gate.
    - Inventing a narrow title-only metadata contract was rejected because title, language, and dispositions already have first-class stream fields and do not cover arbitrary metadata.
- Consequences:
  - Metadata rewrite requests now fail closed instead of mutating files unsafely.
  - Full metadata reconciliation remains an explicit open implementation gap that requires target schema, stored procedures, API/YAML representation, command construction, and candidate/final verification.
- Follow-up:
  - Keep contextless arbitrary metadata rewrite operations fail-closed. ADR 373 adds normalized desired container metadata rows and exact replacement verification for target-driven execution only.
  - Add real-media metadata rewrite fixtures once the desired metadata contract is implemented.

## Task Record

- Motivation:
  - Move the media service closer to production safety by preventing a planned metadata operation from silently deleting metadata.
- Design notes:
  - `BuildArgsError::UnsupportedMetadataRewrite` is a first-class build failure.
  - Preflight classification maps the error to a stable operator-facing code and detail.
  - The existing `OperationKind::MetadataRewrite` remains available for plan accounting, but command construction rejects it.
- Test coverage summary:
  - `cargo test -p revaer-media-runtime metadata_rewrite_fails_closed_without_desired_metadata_contract -- --nocapture`
  - `cargo test -p revaer-media-runtime preflight_error_classification_is_deterministic -- --nocapture`
  - `cargo test -p revaer-media-runtime unsupported_metadata_rewrite_preflight_classification_is_stable -- --nocapture`
  - `cargo test -p revaer-media-runtime -- --nocapture`
  - `just ci`
  - `CI=true E2E_BROWSER_CHANNEL=chrome E2E_VIDEO=off just ui-e2e`
- Observability updates:
  - Preflight reports now expose `preflight_build_unsupported_metadata_rewrite` when unsupported metadata rewrite reaches command construction.
- Status-doc validation:
  - Updated ADR 317 to state that arbitrary metadata rewrite now fails closed until a complete contract exists.
- Risk & rollback plan:
  - Risk is limited to callers that attempted a metadata rewrite operation; they now receive an explicit preflight failure instead of a destructive metadata-stripping command.
  - Rollback would restore the previous FFmpeg argument construction, but that would reintroduce silent metadata loss and should not be used without implementing verification.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No drift or contradictions were found.
