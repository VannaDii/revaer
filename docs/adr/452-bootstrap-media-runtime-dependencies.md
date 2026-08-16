# Bootstrap media runtime dependencies

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - Root policy and the media v1 specification already require runtime logic to
    receive collaborators from callers and reserve concrete infrastructure
    construction for bootstrap and wiring code.
  - `MediaJobRuntime::new` constructed the production inspector, command runner,
    capacity probe, replacement committer, verification executor, audio analyzer,
    and attachment digest adapter inside the runtime module.
  - The runtime already had an internal collaborator bundle used by tests, so
    correcting the production wiring does not require a new ownership model.
- Decision:
  - Treat this as conformance to the existing dependency-injection architecture,
    not as a new architectural choice.
  - Require `MediaJobRuntime::new` to receive a complete collaborator bundle.
  - Construct every production system adapter in `bootstrap.rs` and pass the
    resulting trait-object collaborators to the runtime.
  - Keep concrete adapter implementations and runtime defaults unchanged.
- Consequences:
  - Runtime orchestration no longer selects infrastructure implementations.
  - Production and tests use the same constructor boundary while retaining
    independently injected test doubles.
  - Crate-internal traits and system adapters used by bootstrap gain only the
    visibility required for same-crate wiring.
- Follow-up:
  - Keep future media runtime collaborators mandatory at construction.
  - Add a mechanical architecture check only if similar regressions recur.

## Task Record

- Motivation:
  - Repair a direct violation of the existing root dependency-injection rule
    without advancing any pending media architecture proposal.
- Design notes:
  - No crate boundary, persistence contract, policy behavior, adapter behavior,
    environment variable, or dependency changes.
  - Runtime-owned default tick and workspace values remain unchanged pending the
    separate worker-policy decisions.
- Test coverage summary:
  - Updated production runtime test setup to construct the same concrete bundle
    at its test wiring boundary.
  - Added source-level regression tests proving the runtime module does not
    construct system adapters and bootstrap owns every production adapter.
  - `just check`, `just lint`, `just test`, and `just ci` pass.
  - `just ui-e2e` passes all 104 API and Chromium tests.
- Observability updates:
  - None; construction location changes without changing runtime events, metrics,
    or errors.
- Status-doc validation:
  - Rechecked `MEDIA_TRANSCODING.md`; this change enforces its existing
    dependency-injection requirement and changes no documented capability.
- Risk & rollback plan:
  - Compile-time constructor completeness is the primary guardrail. Rollback is a
    mechanical revert, but would restore the known policy violation.
- Dependency rationale:
  - No dependency added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `MEDIA_TRANSCODING.md`; no drift or contradiction was found and no instruction
    update is required.
