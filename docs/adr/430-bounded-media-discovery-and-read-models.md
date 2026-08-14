# ADR 430: Bounded Media Discovery And Read Models

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Automatic discovery accepted unbounded watcher and scan work, ignored subtitle sidecars, and repeated full-file hashing for canonical aliases.
  - Desired-target and media-job UI reads materialized unbounded graphs or issued queries proportional to row count.
- Decision:
  - Coalesce native events in a fixed-capacity keyed buffer; convert overflow into one profile rescan and publish overflow through the existing discovery outcome metric.
  - Use deterministic bounded scan batches with resumable in-memory cursors and cancellation, entry, depth, file, byte, and elapsed-time limits.
  - Fingerprint a container with its deterministically owned subtitle sidecars, including VobSub pairs, and reject ambiguous ownership.
  - Persist that aggregate identity on the immutable job snapshot and return it in the worker claim so execution and replacement can revalidate the same aggregate.
  - Canonicalize and deduplicate candidates before aggregate hashing.
  - Add stored-procedure-backed bounded desired-target graph and recent-job read models. The recent-job API uses an opaque keyset cursor, defaults to 10 rows, permits at most 100, and includes all six diagnostic counts set-wise.
  - Expose one bounded diagnostics payload per job so UI callers can load details independently and retain partial results when another job fails.
- Consequences:
  - Discovery work and API response memory now have explicit ceilings; overflow and scan limits remain observable.
  - Scan cursors are process-local and restart from the root after process restart, while durable fingerprint claims preserve enqueue idempotency.
- Follow-up:
  - PR87 consumes `GET /v1/media/jobs/recent` and `GET /v1/media/jobs/{id}/diagnostics` using the DTO names recorded below.

## Task Record

- Motivation:
  - Close PR82 discovery and target-graph review findings and provide the outside-in PR87 UI data contract.
- Design notes:
  - Runtime database reads call stored procedures only. No scanner or quality criterion was relaxed.
  - Route DTOs are `MediaRecentJobPageResponse`, `MediaRecentJobSummaryResponse`, `MediaJobDiagnosticCounts`, and `MediaJobDiagnosticsResponse`.
- Test coverage summary:
  - Covers watcher pressure/coalescing, sidecar identity and ambiguous ownership, canonical alias deduplication, every scan budget and resume, page limits, child limits, and cursor behavior.
  - Clean-chain migration validation proves the bounded v3 claim row type is installed through an explicit drop-and-recreate boundary.
- Observability updates:
  - Watcher overflow and each scan-budget stop use `media_discovery_candidates_total` outcomes.
- Status-doc validation:
  - API and ADR indexes were updated; no roadmap claim changed.
- Risk & rollback plan:
  - Main risks are missed filesystem ownership and cursor ordering regressions. Revert this commit and migration 0187; no persisted application state is transformed.
- Dependency rationale:
  - No new third-party dependency was added. `chrono` moved from runtime dev-dependencies to dependencies because the runtime facade now exposes an existing workspace timestamp type.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `rust.instructions.md`, `revaer-data.instructions.md`, and the operational Justfile contract. No drift or contradiction was found.
