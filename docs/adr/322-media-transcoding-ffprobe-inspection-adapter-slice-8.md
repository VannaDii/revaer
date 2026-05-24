# Media transcoding ffprobe inspection adapter slice 8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Media runtime had normalization helpers and an inspect trait, but no concrete injected `ffprobe` inspection adapter.
  - Slice 8 requires full media inspection through an injected ffprobe-compatible adapter with deterministic argument construction.
- Decision:
  - Implement `FfprobeInspectAdapter` in `revaer-media-runtime` with an injected `InspectProbeExecutor` and a system executor implementation.
  - Parse `ffprobe` JSON stream output into probe models and reuse existing normalization for deterministic stream graph output.
  - Alternatives considered: parse plain-text ffprobe output (rejected for fragility) and hard-coded static inspection (rejected because it does not satisfy runtime inspection requirements).
- Consequences:
  - Positive outcomes: runtime now has a concrete inspect adapter that does not rely on shell-string command construction and is unit-testable through dependency injection.
  - Risks or trade-offs: parser currently maps a bounded subset of ffprobe fields needed by current model; future normalization fields may require extending the parsed schema.
- Follow-up:
  - Wire this adapter into app bootstrap/execution flow once runtime execution pipeline reaches inspect integration.
  - Extend parser coverage for additional fields (chapters/attachments sidecar details) as later slices land.

## Task Record

- Motivation:
  - Complete the next missing capability-discovery/inspection foundation unit for media transcoding slice 8.
- Design notes:
  - Added `InspectProbeExecutor` trait (`Send + Sync`) for deterministic tests and runtime injection.
  - Added `SystemInspectProbeExecutor` for production command execution with explicit non-zero exit handling.
  - Added typed ffprobe JSON structs and narrow disposition mapping (`default`, `forced`, `hearing_impaired`) into normalized stream dispositions.
- Test coverage summary:
  - Added `ffprobe_adapter_builds_expected_argv_and_maps_streams`.
  - Added `ffprobe_adapter_rejects_malformed_json`.
  - Re-ran `cargo test -p revaer-media-runtime` (19 passed).
- Observability updates:
  - None in this slice; errors surface via typed `InspectError` variants.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no contradiction updates required for this incremental runtime slice.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none for this change scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to media inspect adapter internals.
  - Rollback is a single commit revert of the adapter and dependency line in `revaer-media-runtime`.
- Dependency rationale:
  - Added `serde_json` (workspace dependency already used across the repo) to parse ffprobe JSON robustly.
  - Alternative considered: custom string parsing without dependency; rejected due to fragility and higher maintenance risk.
