# Media preflight template evaluation entrypoint slice 14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Media runtime preflight gained policy-aware backup path derivation and owned input assembly.
  - Callers still needed multiple steps to build and evaluate preflight.
- Decision:
  - Add `evaluate_preflight_from_template(...)` as a single runtime entrypoint that:
    - builds owned preflight input from template + policy
    - borrows it safely into existing preflight evaluation
    - returns structured ready/failed preflight outcome.
- Consequences:
  - Positive outcomes: external runtime/app integration can call one deterministic preflight API without manually stitching input conversion.
  - Risks or trade-offs: app/runtime caller wiring remains a follow-up in broader orchestration work.
- Follow-up:
  - Use this entrypoint in the first execution orchestration path once media job worker loop is added.

## Task Record

- Motivation:
  - Continue slice-14 implementation by closing the gap between preflight input assembly and evaluation orchestration.
- Design notes:
  - Kept existing borrowed APIs intact, adding an additive wrapper API only.
  - Added end-to-end unit coverage for backup-aware ready-path behavior through the new entrypoint.
- Test coverage summary:
  - Added test: `evaluate_preflight_from_template_builds_and_evaluates_ready_path`.
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment advances execution/preflight orchestration coverage for slice 14.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk additive API; rollback is a localized revert.
- Dependency rationale:
  - No new dependencies added.
