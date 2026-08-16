# Effective media policy compilation

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - ADR 446 addresses how normalized immutable policy snapshot rows reach a
    claimed worker. The runtime still needs one validated effective policy model
    before discovery, planning, scheduling, execution, verification, retention,
    and audit behavior can consume those rows consistently.
  - Today, policy interpretation is split across stored procedures, data rows,
    application constants, planner inputs, and runtime branch logic.
  - The ownership boundary for precedence, defaults, cross-field validation, and
    compilation is an architectural decision because it determines domain
    coupling, database contract breadth, and test strategy.
- Decision:
  - Select Option A, an application/domain compiler, as approved by the operator.
  - **Option A: application/domain compiler.** Stored procedures return bounded
    normalized snapshot rows. A pure Rust compiler validates and combines them
    into one typed immutable `EffectiveMediaPolicy` consumed by injected runtime
    services. This keeps persistence relational and domain semantics testable
    without a database.
  - **Option B: database-produced execution contract.** One stored procedure
    applies precedence and defaults and returns a flattened typed execution
    contract. This centralizes configuration semantics near persistence but makes
    domain evolution and non-database tests depend on a broad SQL contract.
  - **Option C: distributed interpretation.** Each runtime component reads or
    interprets the fields it needs. This minimizes an initial compiler but permits
    inconsistent defaults and incomplete policy consumption.
  - The compiler may not read the environment or construct concrete adapters.
- Consequences:
  - Option A creates one fail-closed point for completeness, bounds, precedence,
    cross-field invariants, and stable error codes before side effects begin.
  - The compiler input remains a persistence DTO; the effective policy becomes a
    domain value and must not expose database or process-environment concerns.
  - Option C does not satisfy the requirement that every persisted policy be
    consistently applied and audited.
- Follow-up:
  - Enumerate every v1 snapshot family and require exhaustive
    compilation with no permissive default for absent required rows.
  - Coordinate the transport contract with ADR 446 and root resolution with ADR
    447.

## Task Record

- Motivation:
  - Prevent the worker-policy transport repair from reproducing the current split
    and partially ignored policy semantics in a different shape.
- Design notes:
  - This ADR separates effective-policy ownership from persistence transport and
    host-specific managed-root trust.
  - Every option retains normalized storage and stored-procedure-only runtime
    access.
- Test coverage summary:
  - Evidence gathered by tracing current persisted snapshot families to their
    Rust readers and runtime consumers.
  - No implementation or behavioral test has been added yet.
- Observability updates:
  - A future implementation must record effective-policy version identity and
    stable compilation failures without logging sensitive host paths.
- Status-doc validation:
  - Rechecked the policy, persistence, determinism, and dependency-injection
    requirements in `MEDIA_TRANSCODING.md`.
- Risk & rollback plan:
  - No runtime risk before approval.
  - After implementation, rollback must not restore distributed permissive
    defaults or live-profile reads for immutable jobs.
- Dependency rationale:
  - No dependency proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; no scoped rule chooses the
    effective-policy compilation owner.
