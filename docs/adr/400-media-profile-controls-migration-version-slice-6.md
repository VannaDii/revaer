# Media profile controls migration version slice 6

- Status: Accepted
- Date: 2026-06-01
- Context:
  - Fresh migration replay failed because two media migrations shared version `0130`.
  - SQLx records migration versions independently of filenames, so duplicate versions cannot be replayed deterministically.
- Decision:
  - Renumber the media profile configuration-controls migration from `0130` to `0136`, after the current media diagnostic-bounds migration tail.
  - Keep the SQL body unchanged because later migrations do not depend on these profile-control procedures during migration application.
- Consequences:
  - Positive outcomes:
    - Fresh local and CI database replays can apply the complete media migration set without duplicate migration-version conflicts.
    - The profile configuration controls remain part of the same feature slice without changing runtime behavior.
  - Risks or trade-offs:
    - Developers with the duplicate migration already recorded locally must allow the local reset path to rebuild from the corrected ordered set.
- Follow-up:
  - Keep future media migrations on monotonically increasing versions.
  - Re-run the full local gates after the migration replay fix.

## Task Record

- Motivation:
  - Restore deterministic database migration replay before continuing media transcoding implementation.
- Design notes:
  - Chose a pure migration rename because the failure was version metadata, not SQL behavior.
  - Verified no `0131` through `0135` migration references the profile-control procedures introduced by the renamed migration.
- Test coverage summary:
  - Reproduced the failure with `just ci`, which failed during `db-start` after applying the first version `0130`.
  - The fix will be verified by rerunning `just ci` through a fresh migration replay.
- Observability updates:
  - None. This only corrects migration ordering metadata.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: local databases that saw the duplicate version need a reset, which the local `just db-start` path already performs for local endpoints. Roll back by restoring the old filename only if no migration replay includes both `0130` files.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
