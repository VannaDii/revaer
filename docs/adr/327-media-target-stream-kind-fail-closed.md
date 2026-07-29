# Media target stream kind fail-closed guard

- Status: Accepted
- Date: 2026-07-22

## Context

- ADR 317 keeps chapter timelines, attachments, and arbitrary metadata in the open media-transcoding gap list.
- The desired-target tables and request validators still accepted `attachment` and `chapter` stream kinds even though the target graph only projected stream-level shape and did not define payload, timeline, metadata, execution, or verification semantics for those kinds.
- Accepting unsupported target rows created a false-success risk: a profile could look configured for chapter or attachment correctness while the runtime could only copy or remux generic streams without proving the richer contract.

## Decision

- Fail closed on desired target `attachment` and `chapter` stream rows until their complete acceptance contracts exist.
- Keep `video`, `audio`, and `subtitle` as the only accepted desired target stream kinds in API requests, YAML import validation, core target compilation, and database constraints.
- Add a migration that aborts if unsupported rows already exist, then tightens desired-target catalog and job-snapshot check constraints.

## Consequences

- Operators cannot configure chapter or attachment targets that the service cannot execute and verify end to end.
- Full inspection still retains chapter and attachment evidence for future implementation work.
- This does not complete the chapter or attachment feature. It prevents unsupported configuration from masquerading as production support.

## Task Record

- Motivation:
  - Remove a false-success path while preserving the broader production-completeness gap honestly.
- Design notes:
  - The validation is duplicated deliberately at API/YAML, core compiler, and database layers so runtime stored-procedure callers and imported bundles agree.
  - The database migration raises an explicit app error if existing unsupported rows would make the constraint change unsafe.
- Test coverage summary:
  - Added core target compilation coverage for unsupported desired chapter rows.
  - Added API validation coverage rejecting attachment and chapter desired streams.
  - Added data-layer coverage proving unsupported attachment and chapter rows cannot be appended after migration.
- Observability updates:
  - No new metrics or events. Rejections reuse existing structured API/import/database error surfaces.
- Risk and rollback plan:
  - Risk is rejecting an operator configuration that previously persisted but was not verifiably honored. Roll back by restoring the broader stream-kind constraints only after implementing chapter and attachment execution plus verification semantics.
- Dependency rationale:
  - No new dependencies.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No quality gate, Sonar, lint, dependency, or stored-procedure criteria were relaxed.
