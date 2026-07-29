# Media replacement recovery manifest validation

- Status: Accepted
- Date: 2026-07-29
- Context:
  - Durable replacement recovery reads a job-scoped manifest before rolling back or finalizing source-root mutations after a restart.
  - Primary media and source-adjacent sidecar mutations share one manifest, so a malformed entry can otherwise describe an impossible recovery shape.
- Decision:
  - Validate replacement manifests before applying recovery actions.
  - Reject committed-entry counts that cannot match the recorded phase, duplicate destinations, relative source or destination components, and entry shapes that lack the stage or recovery file needed for deterministic rollback.
  - Keep invalid manifests fail-closed so startup recovery reports the inconsistency instead of silently preserving uncertain output.
- Consequences:
  - Startup recovery refuses to mutate source files from incomplete or inconsistent transaction evidence.
  - Operators must resolve invalid managed transaction state before job claiming can safely resume.
- Follow-up:
  - Continue closing the desired-state verification gaps tracked in the consolidated media foundation record.

## Task Record

- Motivation:
  - Move the destructive publication boundary closer to production correctness by ensuring recovery only acts on complete rollback evidence.
- Design notes:
  - Validation runs after manifest read and job-key matching, before destination paths are materialized into recovery entries.
  - Valid entry shapes are limited to replace, remove, and create operations with the exact recovery/stage files those operations require.
  - No public replacement API changed.
- Test coverage summary:
  - Added recovery regressions for a committed manifest missing the required recovery file reference and a destination path containing a parent component.
  - Reran focused replacement tests for `revaer-media-runtime`.
- Observability updates:
  - Existing startup recovery failure surfaces now receive `InvalidManifest` before any rollback attempt when the manifest shape is unsafe.
- Status-doc validation:
  - Updated the consolidated media foundation record, ADR index, and mdBook summary.
- Stale-policy check:
  - Reviewed instruction files: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
- Risk & rollback plan:
  - Risk: previously tolerated malformed managed transaction directories now stop recovery until the operator repairs or removes them.
  - Rollback: revert this ADR and the manifest-validation change, then rerun the replacement tests before restoring the previous permissive behavior.
- Dependency rationale:
  - No dependencies were added.
