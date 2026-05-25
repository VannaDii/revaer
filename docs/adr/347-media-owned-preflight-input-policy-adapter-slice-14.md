# Media owned preflight input policy adapter slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Preflight step sequencing now supports optional backup and atomic replacement.
  - Callers need a safe way to derive `backup_path` from profile/policy data without ad-hoc path logic or lifetime hazards.
- Decision:
  - Add `PreflightBuildTemplate`, `PreflightPolicyInput`, and `OwnedPreflightBuildInput`.
  - Add `build_preflight_input` to resolve backup path from policy and produce an owned payload.
  - Add `OwnedPreflightBuildInput::as_borrowed()` to feed existing preflight APIs.
- Consequences:
  - Positive outcomes: caller integration can be deterministic and lifetime-safe while preserving existing borrowed preflight interfaces.
  - Risks or trade-offs: preflight caller wiring outside `revaer-media-runtime` remains pending.
- Follow-up:
  - Wire this adapter into first external runtime/app preflight invocation path when orchestration entrypoint lands.

## Task Record

- Motivation:
  - Continue slice 14 with a stable integration seam from policy/config to replacement-aware preflight.
- Design notes:
  - Introduced small adapter data structures rather than expanding function argument lists.
  - Kept backup-path derivation deterministic via file-name projection onto backup root.
- Test coverage summary:
  - Added tests:
    - `build_preflight_input_resolves_backup_path_from_policy`
    - `owned_preflight_input_as_borrowed_exposes_backup_path`
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment advances execution/preflight integration for slice 14.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk, isolated to media-runtime input assembly; rollback is a single-slice revert.
- Dependency rationale:
  - No new dependencies added.
