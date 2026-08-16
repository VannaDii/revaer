# Preemptible native process startup boundary

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Accepted ADR 501 requires the absolute native-process deadline to start
    before spawn and pipe setup. The current supervisor starts its monotonic
    clock before `std::process::Command::spawn` and includes all elapsed startup
    time when it next evaluates the deadline.
  - `Command::spawn` and the file-descriptor setup calls are synchronous. The
    standard library provides no operation that can interrupt a thread blocked
    in spawn, and a `Child` handle is unavailable until spawn returns.
  - Moving spawn to a Rust thread does not close the gap: Rust cannot terminate
    that thread, returning at the deadline could leave a later-created process
    unsupervised, and waiting for the thread would retain the unbounded startup
    wait.
  - The existing `rustix` 1.1.4 dependency provides process signaling and pidfd
    operations but no portable spawn primitive that returns a controllable
    process handle before synchronous startup completes.
  - Therefore the existing architecture can account startup time against ADR
    501's deadline, but it cannot prove an interruptible hard upper bound for a
    blocked spawn/setup operation while also guaranteeing cleanup before return.
- Options:
  - Recommended: introduce a long-lived, independently supervised native-process
    broker. Start and validate the broker during application bootstrap, place it
    in a containment boundary known to the parent before each media child is
    created, exchange bounded immutable requests over bounded local IPC, and
    terminate the broker containment boundary if startup misses the request
    deadline. This gives the parent a pre-existing control handle but adds a
    packaged runtime component, protocol, lifecycle, health, and recovery model.
  - Implement platform-specific low-level process creation. On each supported
    platform, create pipes and a controllable process identity before exec, then
    supervise an explicit startup handshake. This avoids a broker but introduces
    new FFI or unsafe platform boundaries, divergent implementations, and a much
    larger correctness and portability burden.
  - Run `Command::spawn` on a worker thread and stop waiting at the deadline.
    Reject this option because the blocked thread cannot be cancelled and a
    process created later could escape the supervisor's cleanup-before-return
    guarantee.
  - Keep only elapsed-time accounting around synchronous startup. Reject this as
    the final solution because it does not prove the hard startup bound required
    by ADR 501.
- Decision:
  - Select the independently supervised native-process broker.
- Recommendation:
  - Prefer the independently supervised broker because it establishes a control
    handle before per-request process creation and can meet the deadline without
    introducing unsafe process-creation code into the application process.
- Consequences:
  - The selected recommendation adds a new packaged runtime component and
    make broker availability, protocol compatibility, containment, and restart
    behavior part of media readiness.
  - The ADR 501 startup guarantee remains unvalidated until the separately
    approved broker contract is implemented and passes its exact validation.
- Follow-up:
  - Specify the bounded IPC protocol, containment ownership, readiness contract,
    platform support, failure reasons, and deterministic startup-timeout tests in
    a separate Proposed ADR before implementation.

## Implementation Boundary

- Approval selects the broker architecture but does not by itself authorize an
  unspecified protocol, dependency, packaged process, readiness behavior, or
  change to accepted ADR 501 text.
- No prototype may be committed or pushed until the operator explicitly approves
  the detailed broker contract.
- The ADR 501 implementation cannot be described as fully validated against a
  hard startup deadline until the broker is implemented and validated.

## Task Record

- Motivation:
  - Independent review of the ADR 501 port found that starting the clock before
    synchronous spawn/setup does not make those calls interruptible.
- Design notes:
  - The analysis distinguishes elapsed-time accounting from preemption and keeps
    cleanup-before-return as a mandatory invariant rather than detaching a stuck
    spawn thread.
- Test coverage summary:
  - No runtime test can validate an unimplemented architecture. Documentation
    link/build checks cover this decision record; implementation tests remain
    pending approval and implementation of the detailed broker contract.
- Observability updates:
  - None. An accepted implementation would need bounded broker lifecycle and
    request-outcome signals defined in its own task record.
- Status-doc validation:
  - ADR 501 remains accepted and unchanged. ADR 538 records the broker contract
    as an implementation blocker rather than narrowing the accepted requirement.
- Risk & rollback plan:
  - Recording acceptance changes no runtime behavior. Changing the selected
    architecture requires a superseding Proposed ADR and decision-specific
    operator approval.
- Dependency rationale:
  - No dependency is added. A broker or low-level spawn implementation would
    require separate dependency and platform review after operator approval.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, ADR 461, and ADR 501.
  - No policy relaxation is introduced. Architectural implementation remains
    blocked on explicit approval of the detailed broker contract.
