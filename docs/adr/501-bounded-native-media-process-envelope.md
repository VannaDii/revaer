# Bounded native media process envelope

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Capability probing and inspection already enforce deadlines, bounded output,
    process-group cleanup, and reaping. Transcoding, playback verification, and
    FFmpeg audio analysis use separate process paths with weaker or inconsistent
    limits.
  - Transcoding polls cancellation and workspace capacity but has no absolute
    wall deadline. Audio analysis uses `Command::output`, so it has no active
    cancellation, output cap, descendant cleanup, or deadline.
  - CPU, memory, and ephemeral-storage ceilings are deployment policy. Applying
    ad hoc process limits in Rust would conflict with the separately approved
    container resource boundary and introduce platform-dependent behavior.
- Options:
  - Recommended: route every shipped native media tool through one injected
    process supervisor. Preserve the existing 30-second inspection envelope;
    bound a transcode to `max(30 minutes, 12 * source duration)` with a 24-hour
    ceiling; and bound playback or audio verification to
    `max(10 minutes, 3 * source duration)` with a 4-hour ceiling. Cap stdout at
    16 MiB and stderr at 16 MiB for execution and verification, close stdin,
    terminate the complete process group on cancellation or limit failure, wait
    five seconds, force-kill remaining descendants, and reap before returning.
    Keep CPU, memory, and ephemeral-storage enforcement in validated deployment
    manifests; production outside a validated supervisor remains unsupported.
  - Add only deadlines and output limits to the existing adapters. This is a
    smaller change but leaves descendant cleanup and error semantics inconsistent.
  - Require an external job supervisor for each invocation. This centralizes all
    limits but adds a runtime dependency and complicates local execution.
- Recommendation:
  - Adopt the first option. One supervisor gives every native tool the same
    cancellation, deadline, bounded-diagnostic, cleanup, and error contract
    without duplicating deployment resource policy in application code.
- Consequences:
  - Hung tools, inherited pipes, descendant processes, and unbounded diagnostics
    fail closed with stable reasons.
  - Long transcodes remain possible within an explicit duration-derived ceiling.
  - A process terminated by a deployment memory or storage limit is reported as
    an execution failure; the application does not claim to enforce that limit.
  - Bare-metal production requires an operator-provided supervisor equivalent to
    the validated container boundary.
- Follow-up:
  - After approval, replace direct process paths and add deadline, output,
    descendant, cancellation, escalation, and cleanup-failure regressions.

## Implementation Boundary

- Approval authorizes the shared injected supervisor and exact time, output, and
  five-second termination-grace values above.
- Approval does not authorize per-process CPU or memory limits, new runtime
  dependencies, higher worker concurrency, or relaxation of deployment limits.

## Task Record

- Motivation:
  - Establish one fail-closed native-process contract for the capability and
    inspection paths present in this runtime-foundation slice without importing
    execution, verification, audio-analysis, readiness, or telemetry behavior.
- Design notes:
  - Implemented scope is limited to capability probing and media inspection in
    this runtime-foundation slice. Transcoding, playback verification, and
    FFmpeg audio analysis are not routed through this supervisor here; their
    approved duration-derived envelopes remain unimplemented follow-up work in
    their owning slices.
  - Capability and inspection adapters receive the same injected
    `Arc<dyn NativeProcessSupervisor>`; neither adapter constructs a concrete
    supervisor or launches a process directly.
  - Every invocation requires a live `NativeProcessControl`. The inspection
    adapter's cancellation signal is translated at the supervisor boundary, and
    the capability adapter receives its control from composition.
  - The system supervisor starts its deadline before cancellation, spawn, and
    pipe setup; closes stdin; drains nonblocking stdout and stderr on one thread;
    and retains at most 16 MiB per stream.
  - A typed primary cancellation, deadline, output-limit, exit, or supervision
    failure remains the primary error and stable reason code. Read, cleanup, and
    later-boundary diagnostics are normalized as separate secondary evidence.
    The approved 16 MiB diagnostic budget bounds aggregate data, each item, and
    collection metadata; the inspection adapter retains that structure instead
    of collapsing a typed primary into `Supervision` or `ProbeFailed`.
  - A leader exit is not success until its process group is absent. Surviving
    descendants receive `TERM`, five seconds of grace, `KILL`, and bounded
    reap/absence verification before return. Cancellation and the absolute
    request deadline remain live while descendant cleanup and stream draining
    continue. Reaching the deadline cuts short the graceful wait, forces cleanup,
    and cannot produce a successful result.
- Test coverage summary:
  - Deterministic regressions cover pre-cancellation before spawn, exact and
    maximum-plus-one stream bounds, deadline accounting across pipe setup,
    `TERM` grace followed by forced escalation, successful leaders with
    descendants that retain or close inherited output streams, and one shared
    injected supervisor across capability and inspection adapters. Descendant
    readiness is synchronized through a named pipe before the leader exits; no
    fixed readiness sleep is used.
  - Review regressions additionally cover cancellation and deadline observation
    after leader exit, deadline-driven interruption of graceful cleanup, typed
    primary errors with separate read/cleanup evidence, and typed inspection
    adapter mapping with that evidence intact.
  - Current-tree validation is recorded by ADR 538 and includes the focused
    runtime behavior plus repository check, lint, policy, instruction-drift,
    documentation, diff, and media-cleanup gates.
- Observability updates:
  - The supervisor exposes stable bounded-cardinality primary reason codes and
    normalized secondary diagnostic evidence. Telemetry wiring is intentionally
    outside this transplant, and no command values or unbounded process output
    are persisted.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 318, 427, 429, and 483. This
    implementation does not change a current capability claim.
- Risk & rollback plan:
  - The principal risks are platform process-group semantics and regressions in
    probe error translation. Rollback is one coordinated revert of the shared
    supervisor and the capability/inspection adapter injection; deployment
    resource limits remain in force.
- Residual limits:
  - A descendant that successfully creates a new session or changes to another
    process group before cleanup is outside the original process-group envelope.
    ADR 501 does not authorize host-wide process discovery or containment beyond
    the spawned group, so validated deployment containment remains required.
  - Process-group identifiers are kernel-reused. A narrow race remains between
    process-group existence checks and later signals if the original group
    disappears and its identifier is reused. ADR 501 does not authorize a new
    kernel containment mechanism, pidfd-based group abstraction, or external
    supervisor dependency to close that platform limit.
- Dependency rationale:
  - No dependency was added. Existing Unix process-group support and current
    injected executor patterns cover the recommendation.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift or contradiction was found.
