# Media Stream Metadata Diff Planning

- Status: Accepted
- Date: 2026-07-22
- Context:
  - The desired media graph already carries stream language, title, and disposition state, and the desired-graph `ffmpeg` builder emits those values during a mutating execution.
  - Before this change, `diff_graphs` could still return a no-op when only those stream fields differed, so a metadata-only target change could skip execution and rely on later verification to catch a state that planning never scheduled.
  - Arbitrary container metadata, authored chapter edits, and attachments remain outside this slice because the persisted desired-target graph does not yet define a complete verified contract for them.
- Decision:
  - Extend `GraphDiff` with stream metadata mismatch and stream disposition mismatch dimensions.
  - Generate `LabelRewrite` operations for language/title mismatches and `DispositionRewrite` operations for disposition mismatches.
  - Deduplicate rewrite operations when the same stream is already being transcoded, because the desired-graph command writes the requested stream tags and dispositions in that mutating pass.
  - Score metadata and disposition mismatches as explicit medium-severity compliance violations instead of allowing them to disappear inside a nominally compliant diff.
- Consequences:
  - Desired-target changes to stream language, title, and default/forced disposition now produce an executable plan.
  - Metadata-only desired-target changes no longer complete as no-ops.
  - The legacy arbitrary `MetadataRewrite` operation remains fail-closed until a complete desired metadata schema and verification contract exists.
- Follow-up:
  - Define persisted desired-state contracts for arbitrary stream/container metadata before enabling broad metadata rewrite.
  - Define desired-state contracts for authored chapters and attachments before removing the current fail-closed target-kind guard.

## Task Record

- Motivation:
  - Close a concrete planning gap in the media-transcoding desired graph without relaxing any Sonar, CI, or media verification criteria.
- Design notes:
  - Metadata comparison normalizes language case, trims optional title text, and keeps title comparison case-sensitive so authored labels remain exact desired state.
  - Disposition comparison trims, lowercases, deduplicates, and sorts flags so order-only differences do not create false mutations.
  - `GraphDiff` now derives `Default` so focused tests can opt into only the diff dimension they exercise.
- Test coverage summary:
  - Added media-core diff tests for language/title and disposition-only mismatches.
  - Added media-core planner tests for `LabelRewrite`, `DispositionRewrite`, and transcode deduplication.
  - Added a media-runtime argv test proving desired-graph rewrite operations emit target `language`, `title`, and `disposition` flags.
  - Reran focused core and runtime tests for the changed modules.
- Observability updates:
  - Compliance reports now expose `StreamMetadataMismatch` and `StreamDispositionMismatch` violations with stream ids.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `MEDIA_TRANSCODING.md`, and ADR 317 before changing this slice.
  - Updated ADR 317, `docs/adr/index.md`, and `docs/SUMMARY.md` to reflect the implemented stream metadata/disposition planning contract.
- Risk & rollback plan:
  - Risk is bounded to media planning: metadata-only desired-target updates now schedule a low-cost mutation instead of no-op.
  - Roll back by reverting this ADR and the associated diff/planner/compliance changes; existing post-execution verification remains intact.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Reviewed root and Rust scoped instructions for task-record, lint, panic-free, and verification requirements.
  - No stale instruction contradictions were found in the files touched by this change.
