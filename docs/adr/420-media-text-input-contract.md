# Media text input contract

- Status: Accepted
- Date: 2026-08-11
- Context:
  - Media profile keys and other persisted media identifiers were bounded only by the aggregate HTTP body limit.
  - Profile-change events copy the operator-facing key into the in-memory replay ring, so unbounded identifiers could retain disproportionate memory.
  - Stored procedures are independently callable and therefore must enforce the same contract as HTTP handlers.
- Decision:
  - Define one public API-model contract for media keys and display values.
  - Limit keys to 128 UTF-8 bytes and 128 characters with a lowercase ASCII identifier grammar.
  - Limit display values to 256 UTF-8 bytes and 128 characters and reject control characters.
  - Apply matching database check functions and constraints to persisted media keys, display names, stream keys, titles, and job snapshots.
- Consequences:
  - Invalid or oversized media text fails before persistence and cannot inflate replay events beyond the documented bound.
  - Existing noncanonical data must be corrected before this migration can be applied.
- Follow-up:
  - Keep future persisted media text fields on this shared contract or document a narrower domain-specific contract.

## Task Record

- Motivation:
  - Address the PR review finding that authenticated upserts could retain body-sized profile keys in the event replay buffer.
- Design notes:
  - Byte and character limits are independent so multibyte UTF-8 values have deterministic storage and UI bounds.
  - HTTP validation provides field-specific client errors; database constraints remain the authoritative bypass guard.
- Test coverage summary:
  - Added exact maximum and maximum-plus-one key and multibyte display tests in the API model.
  - Added migration-function and stored-procedure tests for direct database callers.
- Observability updates:
  - No new logs or metrics. Existing RFC 9457 responses report the rejected field and contract reason.
- Status-doc validation:
  - Media API and status documentation were reviewed; no endpoint or capability status changed.
- Risk & rollback plan:
  - The migration can fail when preexisting rows violate the new contract; correct those rows before retrying rather than weakening constraints.
  - Roll back the commit before deployment if compatibility analysis finds an intentional noncanonical key.
- Dependency rationale:
  - No dependencies were added; validation uses `std` and PostgreSQL built-ins.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - No policy drift or stale references were found.
