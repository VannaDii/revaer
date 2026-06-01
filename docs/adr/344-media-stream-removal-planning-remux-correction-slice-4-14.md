# Media stream-removal planning remux correction slice 4/14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - Media plan generation mapped `removed_streams` to `AudioTranscode` operations.
  - Stream removal is container-level graph shaping, not codec recoding, so this produced incorrect operation intent.
- Decision:
  - Update plan generation so any non-empty `removed_streams` set emits a remux operation.
  - Keep recode operation selection stream-kind-aware for audio and video recodes.
- Consequences:
  - Positive outcomes: planner output now models stream removals with the correct container-level operation family.
  - Risks or trade-offs: fine-grained remove/reorder operation modeling is still pending later execution slices.
- Follow-up:
  - Extend execution planning to represent explicit remove/reorder operations once operation taxonomy expands beyond remux/transcode primitives.

## Task Record

- Motivation:
  - Resolve an execution-intent correctness gap while continuing slice 14 implementation incrementally.
- Design notes:
  - `generate_plan` now emits a single `Remux` operation when removal is needed.
  - Added unit coverage to lock this behavior.
- Test coverage summary:
  - Added `plan::tests::removed_streams_yield_remux_operation`.
  - Re-ran:
    - `cargo test -p revaer-media-core -p revaer-media-runtime`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; behavior now better aligns with execution operation intent for stream removals.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Low risk due localized planner change; rollback is a single commit revert.
- Dependency rationale:
  - No new dependencies added.
