# Media PR feedback capability and portability pass

- Status: Accepted
- Date: 2026-05-30
- Context:
  - PR feedback identified overly broad media test skips, GNU-only shell version comparison, Python-only database socket probes, imprecise capability modeling, and two stale ADR/documentation issues.
  - Capability refresh persisted encode/decode support as always true, while execution fallback selection checked codec names instead of encoder names.
- Decision:
  - Treat unavailable local Postgres as the only media data-test skip path and propagate migration/query failures.
  - Replace GNU `sort -V` comparisons with portable `awk` version comparison and make `db-start` TCP probes use `nc` with a Python fallback.
  - Add explicit codec support and encoder lists to runtime capability snapshots, persist detected encode/decode flags, and select video encoders from detected encoder names.
  - Correct stale media ADR documentation and remove the accidental placeholder ADR file.
- Consequences:
  - Positive outcomes: PR feedback is addressed with behavior-level fixes, capability records are more truthful, and local gates are less sensitive to GNU/Python availability.
  - Risks or trade-offs: capability snapshots now carry additional fields that callers must populate in tests and adapters.
- Follow-up:
  - Continue the remaining media transcoding implementation slices after PR feedback gates are green.

## Task Record

- Motivation:
  - Address active PR review feedback before continuing feature work so the branch stays reviewable and gates test the intended behavior.
- Design notes:
  - `CapabilitySnapshot` now keeps codec names, per-codec support flags, and encoder names as separate data.
  - Execution planning validates transcode encoders against encoder names, not codec names.
  - Capability refresh records the detector-provided support flags for each persisted codec row.
  - Data tests skip only when the local test Postgres bootstrap is unavailable.
- Test coverage summary:
  - Added/updated capability parser, execution selection, app refresh, and media data test coverage.
  - Re-ran targeted Rust checks/tests while developing; full `just ci` and `just ui-e2e` are required before handoff.
- Observability updates:
  - None.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; this change aligns the capability-discovery slice with the plan's codec/encoder/decoder reporting requirement.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: `justfile` portability expectations needed to mention Python-independent probes and non-GNU version comparison.
  - Contradictions/stale references removed: accidental placeholder ADR `356-media-transcoding-slices.md` was removed.
- Risk & rollback plan:
  - Moderate risk in capability interpretation; rollback by reverting this ADR and the associated capability/Justfile/test changes.
- Dependency rationale:
  - No new dependencies added.
