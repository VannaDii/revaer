# Media transcoding preflight error classification slice 4/8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight failures had typed Rust enums, but downstream callers still needed ad hoc pattern matching to derive stable error codes and failed stages.
  - Explainability and API mapping need deterministic machine-readable classification.
- Decision:
  - Add `preflight_error_code(&JobPreflightError) -> &'static str`.
  - Add `preflight_failed_stage(&JobPreflightError) -> &'static str`.
  - Map all current preflight error branches to stable code/stage pairs.
- Consequences:
  - Positive outcomes: consumers can map preflight failures without parsing human-readable error strings.
  - Risks or trade-offs: future error variants must update classification helpers to keep mappings complete.
- Follow-up:
  - Use these helpers in app/API error mapping once media preflight endpoints are expanded.

## Task Record

- Motivation:
  - Provide deterministic preflight failure classification for downstream transport/UI layers.
- Design notes:
  - Added explicit branch mapping for workspace and build sub-variants.
  - Kept helper outputs static string constants for stable public semantics.
- Test coverage summary:
  - Added `preflight_error_classification_is_deterministic`.
  - Re-ran `cargo test -p revaer-media-runtime` (33 passed).
- Observability updates:
  - Preflight error classification is now machine-readable via helper functions.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional docs drift identified in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to jobs runtime helper behavior.
  - Rollback is a single commit revert of helper additions and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
