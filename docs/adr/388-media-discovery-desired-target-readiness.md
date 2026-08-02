# Media Discovery Desired Target Readiness

- Status: Accepted
- Date: 2026-08-01
- Context:
  - Direct non-dry-run media job admission already checks the execution capability snapshot, compatibility target readiness, and desired target readiness before accepting work.
  - The background discovery runtime checked the execution capability snapshot and compatibility target readiness, but it did not check pinned desired target readiness before watcher or schedule enqueue.
  - A watcher or scheduled profile pinned to an unsupported desired target could therefore persist work that the worker would later reject during preflight.
- Decision:
  - Reuse the application desired target loader as a shared internal helper so direct admission and background discovery evaluate the same immutable desired target graph.
  - Expose desired target readiness verification within the app crate and call it from discovery readiness for non-dry-run profiles with a pinned desired target.
  - Keep the existing discovery not-ready logging and metric outcome so unsupported desired targets skip enqueue instead of becoming doomed jobs.
  - Add watcher regression coverage for a desired target that requires an unsupported `hevc` video stream while the synthetic capability snapshot supports only `h264` and `mp3`.
- Consequences:
  - Positive outcomes:
    - Watcher and scheduled discovery now fail closed on unsupported desired target muxer, stream kind, or codec requirements before job creation.
    - Direct and background admission paths share the same target graph materialization and readiness semantics.
  - Risks or trade-offs:
    - Discovery now loads desired target catalogs and policy profiles for non-dry-run profiles that pin a desired target.
    - The service still has the broader incomplete areas recorded in ADR 318, including authored attachment/data replacement and remaining full technical-property verification gaps.
- Follow-up:
  - Continue closing the remaining ADR 318 gaps through separate target-contract, runtime, verifier, and fixture changes.
  - Consider exposing the exact readiness failure code in discovery metrics if operators need reason-level dashboards beyond logs.

## Task Record

- Motivation:
  - Close a background intake gap found during the continued production-readiness audit: non-dry-run watcher and scheduled profiles must not enqueue work unless their pinned desired target is executable by the current capability snapshot.
- Design notes:
  - The existing desired target response builder was extracted into `load_media_desired_targets` and reused by the facade method.
  - The existing desired target readiness function remains the single validator and is reused by discovery with the current policy profile catalog.
  - Dry-run discovery keeps its existing behavior because dry-run jobs are intentionally allowed to plan without proving executable output capability.
- Test coverage summary:
  - Added a watcher runtime regression that creates a supported Matroska target with an unsupported `hevc` stream and verifies no media job is queued.
  - Full validation status is recorded in the current task handoff.
- Observability updates:
  - No new telemetry dimensions were added.
  - Existing discovery warning logs continue to include the readiness failure reason, and the existing `capability_not_ready` metric remains the skip outcome.
- Status-doc validation:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - Reviewed ADR 318 and ADR 357. ADR 318 remains accurate that the media transcoding service is not yet complete in the absolute sense.
- Risk & rollback plan:
  - Risk is limited to stricter background admission for non-dry-run profiles with desired targets.
  - Roll back by removing the discovery desired-target readiness block, restoring the private helper visibility, deleting the regression test, and removing this ADR/index entry.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Instruction files reviewed: `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found: none in scoped instructions for this readiness-gating change.
