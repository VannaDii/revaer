# Native process supervisor port

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Accepted ADR 501 defines one injected, bounded process envelope for native
    media tools. The current integration branch still used separate capability
    and inspection process implementations.
  - The authorized supervisor work in commits `7dbbba38` and `3a4bced9` was
    developed against an earlier stack state and required a narrow current-tree
    port.
  - Review found that the source implementation retained secondary cleanup
    evidence in unbounded `Vec<String>` constructors and accumulators.
- Decision:
  - Port only ADR 501's capability and inspection process-envelope slice.
  - Construct one `SystemNativeProcessSupervisor` in application bootstrap and
    inject it into capability probing and media-job inspection. Capability
    probes receive an explicit never-stop control because their existing API has
    no cancellation source; inspections translate their live cancellation
    signal at the supervisor boundary.
  - Start the wall deadline before spawn and pipe setup, close stdin, capture
    stdout and stderr without blocking, retain at most ADR 501's approved 16 MiB
    per stream, and keep cancellation and deadlines live through descendant
    cleanup and stream settlement.
  - Preserve typed primary failures while retaining bounded secondary read and
    cleanup evidence. The evidence type uses the approved 16 MiB diagnostic
    budget as both its aggregate and per-item byte ceiling. Its maximum item
    count is derived from that same byte budget divided by the in-memory
    `String` header size, which also bounds collection metadata without adding a
    new product limit. Excess evidence is marked truncated.
  - Keep transcoding, playback verification, audio analysis, production
    readiness, source identity, persistence, UI, GitHub, and Sonar behavior
    unchanged.
- Consequences:
  - Capability and inspection processes now share one injected process-group
    supervisor with stable primary errors and bounded cleanup diagnostics.
  - A successful leader cannot return while its process group or inherited
    output pipes remain live; descendants receive `TERM`, the approved
    five-second grace, `KILL`, and bounded reap verification.
  - The remaining ADR 501 execution and verification integrations stay explicit
    follow-up work and no production-readiness claim is added.
- Follow-up:
  - Route the remaining approved native-tool paths only in their owning
    deliverables, without importing ADR 519, 526, or 527 behavior into this task.

## Task Record

- Motivation:
  - Bring the reviewed ADR 501 supervisor foundation onto the current integration
    tree while closing the secondary-evidence allocation gap found during audit.
- Design notes:
  - The process model, stream capture, cleanup monitor, and error combiner carry
    `NativeProcessSecondaryEvidence` end to end, so an intermediate unbounded
    cleanup collection cannot bypass the public type's limits.
  - Existing capability and inspection APIs retain their error semantics, with
    secondary cleanup evidence preserved separately from the typed primary.
- Test coverage summary:
  - Process regressions cover pre-cancel no-spawn, exact and plus-one stream
    limits, setup-inclusive deadlines, TERM/KILL/reap behavior, descendants with
    inherited pipes, post-leader cancellation and deadlines, typed error mapping,
    and bounded secondary-evidence bytes and count.
  - Focused media-runtime tests, `just check`, `just lint`, `just policy`,
    `just instruction-drift`, documentation checks, `git diff --check`, and the
    complete `just ci` gate pass on this tree.
  - `just ui-e2e` is not green on base `638e23ff`: the checked-in Playwright
    configuration references an undefined `envDir`. Supplying that value
    externally for diagnosis runs the existing suite until downstream media API
    gaps return 405 for job creation and 400 for the profile fixture (44 passed,
    2 failed, and 60 dependency-blocked). Correcting those out-of-scope UI and
    downstream API gaps is not authorized by this task.
- Observability updates:
  - Stable process outcome codes and bounded secondary evidence remain available
    to callers. No metric, log schema, readiness signal, or persisted record was
    added in this task.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 318, 427, 429, 483, 501, 519, 526,
    and 527. No capability, readiness, or production-completeness claim changes.
- Risk & rollback plan:
  - Primary risks are Unix process-group semantics, adapter error translation,
    and cleanup-evidence truncation. Revert this single port to restore the prior
    capability and inspection runners; deployment resource limits remain in
    force.
- Dependency rationale:
  - No dependency was added. Existing `std`, `rustix`, and injected adapter
    patterns implement the approved envelope.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift or contradiction was found; no policy file changed.
