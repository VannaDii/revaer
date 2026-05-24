# Media transcoding workspace max enforcement slice 12

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Workspace preflight validated reserve and free-space capacity but did not enforce `WorkspacePolicy.max_bytes` against estimated demand.
  - Slice 12 requires deterministic disk-impact and reserve enforcement guardrails.
- Decision:
  - Enforce `required_workspace_bytes <= max_bytes` inside `WorkspacePolicy::ensure_capacity`.
  - Add explicit `WorkspaceError::ExceedsMaxWorkspace` to distinguish policy-limit violations from ambient free-space shortages.
- Consequences:
  - Positive outcomes: jobs exceeding configured workspace bounds fail deterministically at preflight, independent of current free disk.
  - Risks or trade-offs: callers that matched only older workspace error variants need to account for the new variant.
- Follow-up:
  - Surface `ExceedsMaxWorkspace` through higher-level media service/problem mappings when execution orchestration wiring is added.

## Task Record

- Motivation:
  - Close a guardrail gap in disk policy enforcement for media runtime workspace checks.
- Design notes:
  - Added max-bound check before reserve-adjusted free-space evaluation.
  - Kept existing reserve/capacity semantics unchanged for within-policy demands.
- Test coverage summary:
  - Added `capacity_check_rejects_when_required_exceeds_workspace_max`.
  - Re-ran `cargo test -p revaer-media-runtime` (22 passed).
- Observability updates:
  - None in this library-only increment.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, `AGENTS.md`, and `.github/instructions/rust.instructions.md`; no drift found requiring doc edits beyond ADR/index updates.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none in this scope.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is isolated to media-runtime workspace preflight behavior.
  - Rollback is a single commit revert of `workspace/mod.rs` and ADR/index entries.
- Dependency rationale:
  - No new dependencies added.
