# Media direct job source fingerprint

- Status: Accepted
- Date: 2026-07-30
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Media discovery already had a durable source fingerprint table and an atomic enqueue procedure, but API-backed manual discovery and direct job creation could still queue jobs through the lower-level job creation procedure.
  - That bypass made repeated unchanged sources indistinguishable from real changed-source work and allowed durable jobs without a recorded source identity.
- Decision:
  - Route API-backed manual, scheduled, watcher, and direct job admission through the fingerprint-aware enqueue operation.
  - Share the descriptor-bound aggregate fingerprint helper between the service facade and the background discovery runtime.
  - Require lower-level job inserts to snapshot a complete persisted `media_discovery_source_fingerprint` row through the normalized schema trigger before the job row is accepted.
  - Treat unchanged fingerprints as deduplicated discovery results and as direct-create conflicts.
- Consequences:
  - Public job admission now requires the source file to be hashable and stable at admission time.
  - API tests must create real temporary media source files instead of synthetic paths.
  - The lower-level stored procedure remains available for internal composition, but it is no longer a fingerprint bypass.
- Follow-up:
  - Continue the stack with profile-specific capability readiness and worker-time source identity verification.

## Task Record

- Motivation:
  - Close the direct job creation gap identified while splitting the media work into outside-in reviewable PRs.
- Design notes:
  - `media_discovery_fingerprint` owns descriptor-bound source identity, stable aggregate size, nanosecond modification and change times, and aggregate SHA-256 capture for the media file and its sidecars.
  - `MediaService::media_job_create` hashes the source and calls `enqueue_discovered_job` instead of `create_job`.
  - Discovery run helpers now report `media_discovery_source_unchanged` for unchanged sources and `media_discovery_source_unstable` for unhashable or unstable candidates.
  - Migration 0184 makes the database fail closed with `media_job_source_fingerprint_required` if any remaining direct stored-procedure caller tries to insert without a complete fingerprint row.
- Test coverage summary:
  - Added storage coverage for missing fingerprint rejection.
  - Added helper unit coverage for aggregate sidecar ownership and digest changes, sidecar rename/create/delete/symlink revalidation, and sidecar count and byte limits.
  - Updated data, runtime, media runtime, and app tests to create jobs through fingerprint-aware enqueue setup.
  - Updated API E2E to create temporary source files, remove them after each test, and assert unchanged-source deduplication.
  - The full all-feature workspace test suite passed against an isolated PostgreSQL service.
- Observability updates:
  - Existing discovery candidate metrics now distinguish `deduplicated` and `unstable` outcomes for API-backed discovery.
- Status-doc validation:
  - ADR index and book summary were updated for this task record.
- Risk & rollback plan:
  - Risk: direct API callers that previously submitted synthetic or unavailable source paths will now receive validation or conflict responses.
  - Rollback: revert this ADR's admission changes together with the normalized identity-fence migration, restoring direct job insertion behavior; this should only be done with explicit operator consent because it reopens the fingerprint bypass.
- Dependency rationale:
  - No new dependencies were added. The service reuses the existing `sha2`, `tokio`, and `tempfile` dependencies already present in the crate/test graph.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/revaer-data.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No policy drift or contradictions were found while adding this slice.
