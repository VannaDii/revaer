# Media Chapter Preservation Verification

- Status: Accepted
- Date: 2026-07-22

## Context

- Full media inspection already retained source chapter timelines and chapter metadata.
- Existing FFmpeg command construction mapped media streams explicitly but did not request source chapter mapping explicitly.
- Desired target rows for authored chapter state still fail closed because chapter creation, deletion, rewrite, and metadata edit semantics are not modeled.

## Decision

- Preserve source chapters by default for every FFmpeg command that writes a primary media output by emitting `-map_chapters 0`.
- Carry the inspected source chapter timeline through worker preflight and verify source, candidate, and final full inspections against that immutable expected timeline.
- Reject and quarantine or roll back any candidate or committed replacement that drops or changes a source chapter timeline while the current target contract has no authored chapter-edit semantics.

## Consequences

- Jobs can no longer silently lose chapters during remux, stream rewrite, subtitle embedding, or desired-graph materialization.
- Chapter loss now surfaces as `media_job_output_chapter_timeline_mismatch` through the durable verification-check/event path.
- Authored chapter desired-state rows remain rejected until a complete schema, execution, and verification contract exists.

## Task Record

- Motivation:
  - Close a concrete false-success path where a graph-compatible output could pass while losing source chapters retained by full inspection.
- Design notes:
  - Keep preservation command construction centralized with a shared argv helper.
  - Compare normalized chapter start/end times and sorted metadata key/value pairs, ignoring probe-assigned chapter identifiers.
  - Reuse existing candidate quarantine, final rollback, telemetry failure, and verification-check infrastructure instead of adding parallel state.
- Test coverage summary:
  - Added execution-builder coverage proving ordinary remux, desired-graph materialization, staged multi-operation commands, and sidecar embedding emit `-map_chapters 0`.
  - Added worker regressions proving a candidate that drops chapters fails before replacement and a committed replacement that drops chapters rolls back to original source bytes.
  - Reran `cargo test -p revaer-media-runtime execute::tests -- --nocapture`.
  - Reran `cargo test -p revaer-app media_job_runtime::tests -- --nocapture`.
- Observability updates:
  - Added durable `source_chapters`, `candidate_chapters`, and `final_chapters` verification checks.
  - Chapter mismatches publish `MediaJobVerificationFailed` with `media_job_output_chapter_timeline_mismatch` and increment the existing verification failure telemetry.
- Risk and rollback plan:
  - Risk: an otherwise playable candidate from a muxer or encoder path that cannot preserve chapters now fails closed.
  - Roll back by removing `-map_chapters 0` and the chapter timeline verifier, accepting the prior risk of silent chapter loss.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`, and ADR 317.
  - No policy drift or criteria relaxation was introduced.
