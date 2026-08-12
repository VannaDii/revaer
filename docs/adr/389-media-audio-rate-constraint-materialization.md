# Media Audio Rate Constraint Materialization

- Status: Accepted
- Date: 2026-08-02
- Context:
  - Authored desired-target audio bitrate and sample-rate fields are persisted, carried into job snapshots, emitted as FFmpeg arguments when an audio stream is encoded, and verified against candidate and final inspection.
  - The desired-graph command builder could still select stream copy when the source and desired audio codec and channel shape already matched and the only authored audio changes were bitrate or sample rate.
  - That path deferred the problem to candidate verification instead of materializing the selected target contract.
- Decision:
  - Treat authored audio bitrate and sample-rate constraints as encoder-requiring constraints.
  - Keep loudness and speech dynamic-range policies on the same encoder-required path because they need filtergraph execution.
  - Add focused command-builder regression coverage proving bitrate/sample-rate constraints force `aac` encoding and emit `-b:a` and `-ar` even without a loudness filter.
- Consequences:
  - Positive outcomes:
    - A profile that asks for a concrete audio bitrate or sample rate now builds a command that attempts to produce that output rather than copying the source stream unchanged.
    - Candidate and final verification remain the fail-closed proof that the encoder actually produced the requested inspected values.
  - Risks or trade-offs:
    - Jobs that previously copied an audio stream when only rate constraints were set will now perform an audio encode.
    - The broader ADR 318 incompleteness remains: authored attachment/data creation and additional technical-property contracts are still outside the completed service boundary.
- Follow-up:
  - Continue closing remaining ADR 318 gaps through dedicated target-contract and verifier slices.
  - Add real-media fixture evidence if a later PR broadens the supported audio policy vocabulary beyond the current bitrate, sample-rate, channel layout, `dialog-normalized`, and `speech` subset.

## Task Record

- Motivation:
  - Close an execution/materialization gap found during the continued production-readiness audit: authored audio rate constraints must drive FFmpeg output construction, not only post-output verification.
- Design notes:
  - Replaced the filter-only encoder gate with an audio-constraint encoder gate that includes bitrate and sample-rate constraints.
  - Left channel-count and channel-layout behavior unchanged because those constraints are already represented in the desired graph and already force encoding when they differ from the inspected source shape.
  - Left verification behavior unchanged; it remains responsible for proving inspected bitrate tolerance and exact sample-rate compliance.
- Test coverage summary:
  - Added `desired_graph_audio_rate_constraints_force_audio_encoder_without_filters` in `crates/revaer-media-runtime/src/execute/mod.rs`.
- Observability updates:
  - No new telemetry, logs, metrics, or events were added.
- Status-doc validation:
  - Reviewed ADR 318 and the current execution and verification code paths.
  - ADR 318 remains accurate that the media transcoding service is not yet complete in the absolute sense.
- Risk and rollback plan:
  - Risk is limited to audio encode selection for targets that explicitly declare bitrate or sample-rate constraints.
  - Roll back by restoring the filter-only encoder gate, removing the focused regression test, and removing this ADR/index entry.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none for this scoped runtime command materialization change.
