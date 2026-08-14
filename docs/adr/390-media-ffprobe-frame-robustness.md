# Media FFprobe Frame Robustness

- Status: Accepted
- Date: 2026-08-11
- Context:
  - FFprobe can emit subtitle and other frame records without `stream_index`, including WebVTT-in-WebM inputs.
  - Frame side-data supplements stream-level side-data, but it is only safe to attribute when FFprobe identifies the owning stream.
  - The modular inspector retained stream side-data parsing but no longer requested or modeled the bounded frame sample from the earlier inspector.
- Decision:
  - Request one bounded frame sample with the source probe alongside streams, format, and chapters.
  - Model frame `stream_index` as optional and accept frame records that omit it.
  - Merge frame side-data only into the stream named by an available index; discard unindexed frame side-data rather than guessing ownership.
  - Keep this change inside the modular ffprobe DTO, parser, and adapter boundaries without replaying the earlier monolithic inspector or unrelated fixture-loop expansion.
- Consequences:
  - Indexed frame side-data remains available to technical inspection and verification.
  - Unindexed frame records no longer make otherwise valid media inspection fail.
  - A small bounded frame sample increases source-probe output, which remains constrained by the existing per-process and aggregate output budgets.
  - Side-data on an unindexed frame is intentionally unavailable because no deterministic stream association exists.
- Follow-up:
  - Exercise additional licensed edge fixtures in separate evidence slices when their materialization contracts are ready.
  - Preserve the one-frame read bound when extending technical metadata inspection.

## Task Record

- Motivation:
  - Restore the unindexed-frame robustness from commit `2e02d464` on the current rebuilt modular inspector without restoring obsolete architecture.
- Design notes:
  - `FfprobeOutput` owns the optional frame list, `parse_source` owns safe stream attribution, and `source_probe_args` owns the bounded ffprobe request.
  - Unknown and unindexed frame fields remain accepted through Serde's normal forward-compatible object parsing.
  - The old Chromium fixture-loop and graph-assertion changes were intentionally excluded because they are separate acceptance-evidence deliverables.
- Test coverage summary:
  - Added parser coverage proving unindexed frame records inspect successfully and their side-data is not misattributed.
  - Added parser coverage proving indexed frame side-data is merged only into its owning stream.
  - Added adapter coverage proving the source probe requests `-show_frames` with the `%+#1` read interval.
  - Full validation status is recorded in the task handoff.
- Observability updates:
  - No logs, metrics, traces, health checks, or event surfaces changed.
- Status-doc validation:
  - Rechecked `MEDIA_TRANSCODING.md` and ADR 318; this parser hardening does not change the documented implementation-completeness boundary.
- Risk & rollback plan:
  - Risk is limited to the bounded increase in ffprobe output and merging correctly indexed frame side-data.
  - Roll back by removing frame DTO parsing and attribution, restoring the prior source probe arguments, removing the focused tests, and deleting this ADR entry.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - No scoped policy drift or contradiction was found.
