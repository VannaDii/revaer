# Degraded media capability startup

- Status: Accepted
- Date: 2026-08-13
- Context:
  - Media capability discovery can fail because the installed toolchain is
    absent, malformed, or temporarily unable to execute.
  - The media specification requires that failure to block media execution
    while the rest of Revaer, including health and remediation APIs, starts.
  - Packaged runtime lifecycle work incorrectly propagated the refresh failure
    through application bootstrap.
- Decision:
  - Supersede only the fatal capability-startup decision in
    [ADR 440](440-packaged-media-runtime-lifecycle.md); its remaining runtime
    lifecycle decisions remain accepted.
  - Treat startup capability refresh as a handled media degradation boundary.
  - Preserve the warning, failure metric, typed refresh-failure event, and
    degraded-health event while allowing API and non-media runtimes to start.
  - Continue to rely on capability readiness checks to fail media discovery and
    execution closed until a valid persisted snapshot exists.
- Consequences:
  - Operators retain access to health and capability refresh surfaces when the
    media toolchain is broken.
  - A running application is not evidence that media execution is ready;
    readiness remains explicit in media capability state and observability.
- Follow-up:
  - Add packaged-image coverage that repairs a failed startup probe through the
    on-demand capability refresh endpoint.

## Task Record

- Motivation:
  - Remove a release-blocking contradiction between bootstrap behavior and the
    first-release degraded-mode contract.
- Design notes:
  - The startup helper now owns and reports its expected failure instead of
    translating it into an application-fatal error.
  - No capability fallback or fabricated snapshot is introduced.
- Test coverage summary:
  - Added a deterministic failing detector test that proves startup refresh
    returns normally and emits both failure and degraded-health events.
  - The test also proves the existing Prometheus failure counter is incremented.
- Observability updates:
  - Existing warning, metric, capability-refresh failure event, and health event
    are retained unchanged.
- Status-doc validation:
  - Rechecked `MEDIA_TRANSCODING.md`; its degraded startup requirement remains
    authoritative and required no wording change.
- Risk & rollback plan:
  - The risk is an application running with unavailable media execution, which
    is intentional and observable. Roll back this commit to restore fatal
    startup only if the operator explicitly changes the product contract.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/rust.instructions.md`.
  - Corrected the stale fatal-startup language in ADR 440; no instruction
    change was required for this runtime behavior fix.
