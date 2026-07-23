# Media capability codec support validation

- Status: Accepted
- Date: 2026-07-22
- Context:
  - Media job planning and execution depend on the latest persisted ffmpeg capability snapshot to prove that required codecs and encoders are available before a replacement transcode is prepared.
  - The detector parses per-codec encode/decode support from `ffmpeg -codecs`, and persistence stores those flags, but the runtime validity predicate still accepted snapshots that advertised codec names without matching support rows.
  - A name-only snapshot can make preflight appear ready while losing the evidence required to distinguish encode-capable, decode-only, and unsupported codec rows.
- Decision:
  - Require every advertised codec in `CapabilitySnapshot::is_valid` to have a matching `CodecCapability` row with at least one supported direction.
  - Re-run the same validity check after the app runtime reconstructs a capability snapshot from stored database rows.
  - Keep test fixtures explicit about codec support so valid fixtures mirror production detector output.
  - Do not relax Sonar configuration or hide binary asset scanner warnings as part of this change.
- Consequences:
  - Positive: stale, manually assembled, or partially persisted capability snapshots fail closed before media jobs plan or execute.
  - Positive: decode-only codecs remain valid inventory entries, while codecs with neither encode nor decode support are rejected.
  - Risk: deployments with old incomplete capability records must refresh capabilities before jobs can run.
- Follow-up:
  - Continue PR 31 Sonar/CI monitoring after the amended branch push.
  - Address committed binary asset scanner warnings only through an approved asset migration or explicit operator consent for a scoped scanner setting.

## Task Record

- Motivation:
  - Close a production correctness gap found during the PR 31 readiness pass: capability validation needed to prove codec support evidence, not just codec names.
- Design notes:
  - Added a private `codec_support_covers_codecs` helper in the media runtime capability model.
  - Kept the helper case-insensitive and direction-aware so ffmpeg naming differences do not reject valid rows.
  - Reused `CapabilitySnapshot::is_valid` in the app job runtime after rebuilding persisted capability state.
- Test coverage summary:
  - Added unit coverage for missing codec support, unsupported codec support, and decode-only valid codec support.
  - Updated job preflight fixtures so valid capability snapshots include codec support rows.
  - Corrected the app media capability detector fixture so it no longer advertises blank or duplicate codec names as valid detector output.
  - Reran targeted capability, job, and app media job runtime tests.
- Observability updates:
  - No new logs, metrics, or events were required; invalid snapshots continue through the existing `media_capability_snapshot_invalid` failure path.
- Status-doc validation:
  - `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md` were reviewed for this scoped media/Sonar validation change.
  - No stale instruction drift was found in the reviewed policy surface.
- Risk & rollback plan:
  - Roll back the capability validation changes and this ADR if a valid production ffmpeg snapshot is proven to omit support rows despite successful detection.
  - Operational rollback is to refresh capabilities so persisted codec support rows match the current runtime toolchain.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed root and scoped Rust/devops/Sonar instructions for stricter validation and scanner posture requirements.
  - No contradictions were introduced or removed.
