# Media transcoding codec capability normalization slice 8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Capability-gated execution step checks required exact codec string matches.
  - ffmpeg capability output can vary in casing and incidental whitespace, which can cause false unsupported-codec failures.
- Decision:
  - Normalize capability codec matching in execution preflight by trimming and applying ASCII case-insensitive comparison.
  - Keep required codec identifiers stable (`aac`, `libx265`) while making capability lookups robust.
- Consequences:
  - Positive outcomes: deterministic preflight behavior no longer depends on cosmetic codec formatting in detector output.
  - Risks or trade-offs: matching remains exact-token based after normalization and does not yet include alias mapping.
- Follow-up:
  - Extend matching with explicit codec alias maps when profile-level codec targeting is introduced.

## Task Record

- Motivation:
  - Remove false negatives in capability-gated step construction caused by codec formatting differences.
- Design notes:
  - Added internal helper `capabilities_has_codec` in `execute` module.
  - Updated audio/video transcode capability checks to use normalized matching.
- Test coverage summary:
  - Added `capability_checked_execution_accepts_trimmed_case_insensitive_codec_names`.
  - Re-ran `cargo test -p revaer-media-runtime` (29 passed).
- Observability updates:
  - None in this runtime library increment.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no additional documentation drift found in scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to runtime capability matching behavior.
  - Rollback is a single commit revert of `execute/mod.rs` and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
