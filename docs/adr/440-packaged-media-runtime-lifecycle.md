# Packaged media runtime lifecycle

- Status: Accepted
- Date: 2026-08-13
- Context:
  - Application bootstrap requires an absolute private media workspace, but the
    production image and Helm chart did not provide one.
  - Kubernetes used the database health endpoint for every probe and normal
    termination did not drive Axum graceful shutdown before worker cleanup.
- Decision:
  - Default the packaged workspace to `/data/media-workspaces`, expose it as a
    schema-validated Helm value below the chart's `/data` mount, and document
    that destructive operation requires persistent data storage.
  - Add dependency-independent liveness and database-backed readiness routes.
  - Record startup media capability discovery failures as degraded media
    readiness while preserving application availability, as corrected by
    [ADR 441](441-degraded-media-capability-startup.md), and use SIGTERM/SIGINT
    to gracefully stop the API before the existing cooperative worker cleanup.
  - Inject the capability detector through bootstrap dependencies; production
    wiring supplies the system FFmpeg detector and tests supply deterministic
    capability snapshots.
  - Keep the Kubernetes termination grace above the worker's 30-second bound.
- Consequences:
  - The packaged deployment boots without an undocumented environment repair.
  - Pods do not advertise readiness without database access or startup media
    capabilities, and normal termination reaches managed cleanup.
  - Operators must enable data persistence before destructive jobs when job
    workspaces need to survive pod replacement.
- Follow-up:
  - Add live worker supervision to readiness in a subsequent bounded slice.
  - Add packaged-image termination integration coverage with a real media job.

## Task Record

- Motivation:
  - Close the highest-risk packaging gap found by the v1 implementation audit.
- Design notes:
  - Reuse the existing data volume instead of introducing a second persistence
    lifecycle or dependency.
  - Preserve `/health` compatibility while adding probe-specific routes.
- Test coverage summary:
  - Added liveness and readiness handler tests.
  - Added a Helm render/schema regression gate for workspace, probes, and
    termination grace.
  - Covered degraded capability startup and occupied-listener behavior with
    injected capabilities under the no-default-features matrix.
  - Bootstrap integration tests prepend deterministic FFmpeg probe executables
    so host package inventory cannot change the lifecycle behavior under test.
  - Graceful shutdown selection and typed media error translation have direct
    unit coverage, including both Unix termination signals.
  - Shared torrent catalog routing and degraded-config assertions are factored
    once so the lifecycle change does not add duplicated maintenance paths.
  - API and UI E2E runners use the guarded database-media setup profile so
    degraded startup validates and reports real FFmpeg capabilities in CI.
  - Media execution tests use RAII temporary directories so repeated runs
    cannot reuse stale quarantine artifacts and cleanup survives assertions.
  - Helm lint, focused Rust tests, `just ci`, and `just ui-e2e` passed.
- Observability updates:
  - Signal receipt is logged once and startup capability failure remains an
    emitted event while bootstrap continues in degraded media mode.
- Status-doc validation:
  - Rechecked the chart README, `.env.example`, and media specification against
    the packaged runtime contract.
- Risk & rollback plan:
  - Readiness is intentionally stricter. Roll back this commit if startup media
    discovery unexpectedly rejects a supported packaged environment.
- Dependency rationale:
  - No dependency was added; Tokio's existing signal support and Axum's native
    graceful-shutdown API are used.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust instructions, and DevOps instructions.
  - DevOps guidance was updated because chart and image lifecycle changed; no
    contradictory or stale requirement was retained.
