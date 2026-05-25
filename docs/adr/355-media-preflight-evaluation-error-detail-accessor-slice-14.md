# Media preflight evaluation error-detail accessor slice 14

- Status: Accepted
- Date: 2026-05-25
- Context:
  - Preflight failure payload now includes deterministic `error_detail`.
  - `JobPreflightEvaluation` exposed accessor helpers for failed stage/code, but not detail.
- Decision:
  - Add `JobPreflightEvaluation::error_detail()` returning deterministic detail text when failed.
  - Extend evaluation accessor tests to assert `error_detail` behavior for both ready/failed outcomes.
- Consequences:
  - Positive outcomes: callers can retrieve all core failure diagnostics via uniform evaluation accessors.
  - Risks or trade-offs: none; additive API helper.
- Follow-up:
  - Use this accessor in future API wiring to keep error projection logic centralized.

## Task Record

- Motivation:
  - Continue slice 14 with consistent preflight evaluation ergonomics and reduce caller pattern-matching.
- Design notes:
  - Added a const accessor mirroring existing `failed_stage()` and `error_code()` helpers.
  - Updated existing accessor test to include detail assertions.
- Test coverage summary:
  - Updated test: `preflight_evaluation_ready_flag_is_deterministic`.
  - Re-ran:
    - `cargo test -p revaer-media-runtime`
    - `cargo clippy -p revaer-media-runtime --all-targets --all-features -- -D warnings`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this increment improves slice-14 preflight diagnostics API ergonomics.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Very low risk additive method; rollback is localized revert.
- Dependency rationale:
  - No new dependencies added.
