# Media transcoding preflight report runtime slice 8/12

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Runtime offered separate primitives for inspect/planning, capability checks, workspace capacity checks, summaries, and execution-step building.
  - Orchestration callers had to compose those checks manually, increasing drift risk and reducing deterministic preflight consistency.
- Decision:
  - Add `JobPreflightReport` and `build_preflight_report` in `revaer-media-runtime::jobs`.
  - The report builder executes one deterministic sequence: inspect -> plan -> capability validity -> workspace capacity -> capability-gated step build -> summary.
  - Extend `JobPreflightError` to classify capability, workspace, and build-step failures.
- Consequences:
  - Positive outcomes: one call yields a complete preflight decision artifact with planned ops, explainability summary, and executable steps.
  - Risks or trade-offs: preflight API now has a larger error taxonomy that downstream mappings must preserve.
- Follow-up:
  - Wire this report into app/API preflight endpoints once execution orchestration is expanded.

## Task Record

- Motivation:
  - Close a composition gap and make preflight behavior deterministic and reusable.
- Design notes:
  - Added `JobPreflightReport { planned, summary, steps }`.
  - Added `build_preflight_report(...)` with explicit injected dependencies and deterministic ordering of checks.
  - Added `JobPreflightError::{Capability, Workspace, Build}` variants.
- Test coverage summary:
  - Added `build_preflight_report_returns_summary_and_steps`.
  - Added `build_preflight_report_rejects_invalid_capabilities`.
  - Re-ran `cargo test -p revaer-media-runtime` (28 passed).
- Observability updates:
  - None in this runtime library increment.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no further policy drift identified for this scope.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to media-runtime preflight composition.
  - Rollback is a single commit revert of jobs runtime changes and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
