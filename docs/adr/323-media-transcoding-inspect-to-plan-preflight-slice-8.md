# Media transcoding inspect-to-plan preflight slice 8

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Runtime had discrete inspect and planning primitives but no direct preflight path that composes them.
  - Slice 8 requires deterministic inspection-driven planning behavior through injected adapters.
- Decision:
  - Add `plan_job_from_inspect` in `revaer-media-runtime::jobs` to inspect source media via `InspectAdapter`, then run deterministic diff/plan/verify with workspace estimation.
  - Add `JobPreflightError` to preserve inspect versus planning failure classes.
  - Keep `plan_job` as the source-graph path and route it through a shared graph-based helper.
- Consequences:
  - Positive outcomes: runtime preflight now has a single adapter-driven entrypoint that can be used by higher-level orchestration without shell coupling.
  - Risks or trade-offs: error surface now has another public enum and must remain stable as orchestration layers adopt it.
- Follow-up:
  - Wire `plan_job_from_inspect` into app/runtime job orchestration once execution pipeline slices consume inspect adapters.
  - Extend tests as additional planning operation kinds are introduced.

## Task Record

- Motivation:
  - Close the next integration gap between inspection and planning within media-runtime slice 8.
- Design notes:
  - Added `JobPreflightError::{Inspect, Plan}`.
  - Added `plan_job_from_inspect(inspector, source_path, desired, source_file_bytes)`.
  - Added shared `plan_job_from_source_graph` helper and reused it from `plan_job`.
- Test coverage summary:
  - Added `plan_job_from_inspect_uses_inspected_graph`.
  - Added `plan_job_from_inspect_propagates_inspect_error`.
  - Re-ran `cargo test -p revaer-media-runtime` (21 passed).
- Observability updates:
  - None in this slice; this change is pure runtime library behavior.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no further docs drift identified for this increment.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is limited to media-runtime planning call paths.
  - Rollback is a single commit revert of `crates/revaer-media-runtime/src/jobs/mod.rs` and ADR/index updates.
- Dependency rationale:
  - No new dependencies added.
