# Media audio channel constraint verification

- Status: Accepted
- Date: 2026-08-01
- Context:
  - Desired audio targets already carry channel count and channel layout into the compiled graph and FFmpeg command construction.
  - Detailed audio constraint verification carried bitrate, sample rate, and measured loudness policy, but not channel count or channel layout.
  - That left channel mismatches to the generic graph check, which was correct but less specific than the rest of the constrained audio verification path.
- Decision:
  - Add channel count and channel layout to the immutable audio stream constraint record used by execution policy and candidate/final verification.
  - Treat any target audio row with channel count or channel layout as a constrained audio row so the constraint mapping path also proves it binds to the compiled desired stream.
  - Compare constrained channel count and canonical supported channel layout against the inspected graph stream before bitrate, sample-rate, and measured loudness checks.
  - Alternatives considered:
    - Leave channel verification only in generic graph comparison: rejected because constrained audio rows should carry all authored audio constraints through the same fail-closed verification path.
    - Add per-channel signal-position analysis now: deferred because the current normalized inspection model exposes FFprobe's channel count/layout labels, not independent decoded channel-position signal measurements.
- Consequences:
  - Positive outcomes:
    - Candidate and final verification now carry declared `channel_count` and `channel_layout` through the audio constraint path, allowing precise diagnostics when the constrained row and inspected graph stream disagree.
    - Audio channel-only target rows now participate in constraint-to-desired-stream mapping, matching bitrate/sample-rate/loudness target behavior.
  - Risks or trade-offs:
    - This still verifies the supported FFprobe layout label/count contract, not semantic audio content in each physical channel.
- Follow-up:
  - Evaluate whether decoded per-channel signal analysis is needed for channel-position verification beyond the supported layout label/count contract.

## Task Record

- Motivation:
  - Move the media service closer to production readiness by making declared audio channel constraints first-class in the detailed verification path.
- Design notes:
  - Reused the existing normalized channel-layout contract from `revaer-media-core`.
  - Kept command construction unchanged because FFmpeg channel flags already derive from the desired graph.
  - Used the inspected graph stream for channel count/layout and the full stream inspection for bitrate, sample rate, and measured policy fields.
- Test coverage summary:
  - Added focused app tests for channel-count mismatch, channel-layout mismatch, supported layout alias acceptance, and channel-only constraint mapping.
- Observability updates:
  - Detailed audio constraint failures can now report `stream:N:channel_count=...` and `stream:N:channel_layout=...` when that verifier observes a mismatch.
- Status-doc validation:
  - Reviewed ADR 318 and retained its current media-transcoding gap statement.
- Risk and rollback plan:
  - Risk: malformed legacy snapshots with unsupported channel-layout strings now fail the detailed constraint verifier instead of relying on the generic graph check.
  - Roll back by removing the two audio constraint fields and restoring channel-only audio rows to generic graph-only verification.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Reviewed `docs/adr/318-media-transcoding-foundation.md` for the current media-transcoding gap statement.
  - No policy relaxation or stale instruction contradiction was introduced.
