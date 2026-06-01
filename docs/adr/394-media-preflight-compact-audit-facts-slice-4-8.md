# Media preflight compact audit facts slice 4/8

- Status: Accepted
- Date: 2026-06-01
- Context:
  - The media plan requires dry-run and execution preflight jobs to persist compact audit facts that explain planning, capacity, and failure outcomes.
  - Persistence, API, and UI surfaces already accept compact audit facts, but runtime preflight outcomes did not yet have a deterministic projection into rows.
- Decision:
  - Add a runtime `CompactAuditFact` projection from `JobPreflightEvaluation`.
  - Emit deterministic timeline facts first, followed by ready plan/capacity facts or a failed-stage fact.
  - Keep the projection text compact, stable, and free of unbounded structured payloads.
- Consequences:
  - Positive outcomes:
    - Runtime preflight results now have a direct shape that can be persisted through the compact-audit stored procedures.
    - Failed dry-run and blocked execution preflights retain the stage, code, and detail needed for later explanation.
  - Risks or trade-offs:
    - The first projection is intentionally terse; future UI or release evidence may need additional normalized fact kinds.
- Follow-up:
  - Wire job orchestration to persist these facts when creating dry-run and execution preflight job records.
  - Add retention tests that preserve compact audit facts after pruning larger diagnostics.

## Task Record

- Motivation:
  - Close the runtime gap between structured preflight outcomes and the compact-audit persistence/API/UI surfaces.
- Design notes:
  - Kept fact ordering deterministic by assigning sequential indices after timeline traversal.
  - Represented missing stage codes and capacity rejection reasons as explicit `none` text to avoid ambiguous empty values.
- Test coverage summary:
  - Added failing runtime tests for ready and failed preflight compact-audit projections before implementation.
  - Ran `CARGO_TARGET_DIR=target/media-compliance-red cargo test -p revaer-media-runtime preflight_compact_audit_facts -- --test-threads=1`.
- Observability updates:
  - Added no logging or metrics. The new compact facts provide deterministic audit payloads for existing persistence surfaces.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `MEDIA_TRANSCODING.md`.
  - Updated ADR index and documentation summary for this task record.
- Risk & rollback plan:
  - Risk: downstream consumers may rely on exact fact text once orchestration persists it. Roll back by removing the projection, tests, and this ADR before wiring orchestration to durable storage.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`.
  - Drift found: none.
  - Contradictions or stale references removed: none.
