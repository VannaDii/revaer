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
  - Make native media execution fail closed under the same bounded contract used
    by capability detection and inspection.
- Design notes:
  - Source duration is already available from bounded inspection and becomes an
    immutable input to deadline calculation. Checked arithmetic must saturate at
    the stated ceilings.
- Test coverage summary:
  - Existing coverage proves bounded inspection, direct-child cancellation,
    workspace reserve loss, and verifier cleanup. The follow-up matrix above is
    required before this proposal is implemented.
- Observability updates:
  - Add bounded-cardinality termination-reason metrics and stable error codes;
    do not persist command values or unbounded diagnostics.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 318, 427, 429, and 483. This proposal
    does not change a current capability claim.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. After implementation,
    rollback is one coordinated revert to the prior injected adapters;
    deployment resource limits remain in force.
- Dependency rationale:
  - No dependency is proposed. Existing Unix process-group support and current
    injected executor patterns cover the recommendation.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift or contradiction was found.
