# Media stream-kind-aware recode planning slice 4/14

- Status: Accepted
- Date: 2026-05-24
- Context:
  - PR #31 follow-up feedback noted that recoded streams were always emitted as `VideoTranscode`, even when stream kind was audio.
  - Execution planning requires deterministic operation kinds that match stream semantics.
- Decision:
  - Extend media graph diff results to carry recoded stream kind.
  - Update planner generation to map recoded streams by kind:
    - audio -> `AudioTranscode`
    - video -> `VideoTranscode`
  - Keep operation generation deterministic and unit-tested.
- Consequences:
  - Positive outcomes: planner output now reflects actual stream type for recode operations and avoids incorrect transcode intent.
  - Risks or trade-offs: subtitle/attachment/chapter recode handling remains limited and will need fuller operation modeling in later slices.
- Follow-up:
  - Continue execution/planner hardening for non-audio/video recode semantics and multi-operation composition.

## Task Record

- Motivation:
  - Close a specific planner correctness gap before adding additional execution complexity.
- Design notes:
  - Added `RecodedStream { stream_id, kind }` in `revaer-media-core::diff::GraphDiff`.
  - Planner now uses stream kind when selecting transcode operation kind.
  - Runtime execute changes in this pass scope transcode argv to a selected stream id (`-map 0:{stream_id}`) to align with stream-scoped planning.
- Test coverage summary:
  - Added and validated unit tests:
    - `plan::tests::recoded_audio_stream_yields_audio_transcode`
    - `plan::tests::recoded_video_stream_yields_video_transcode`
    - `execute::tests::transcode_targets_only_selected_input_stream`
  - Re-ran:
    - `cargo test -p revaer-media-core -p revaer-media-runtime`
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and existing ADR chain for slices 4/8/14 alignment; no additional status-doc edits required beyond ADR index updates.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions/stale references removed: none.
- Risk & rollback plan:
  - Risk is low-to-moderate and localized to planner/argv generation; rollback is a single commit revert.
- Dependency rationale:
  - No new dependencies added.
