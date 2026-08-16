# Worker policy snapshot boundary

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The v1 contract requires relational runtime configuration and immutable job
    snapshots for runtime limits, output behavior, workspace behavior, backup,
    verification, maintenance windows, operation costs, retention rules, and
    subtitle discovery.
  - Job creation persists these values in normalized snapshot tables.
  - The worker claim procedure returns only source identity, target fields,
    unmatched actions, and verification fields. No authored Rust reader exposes
    the remaining behavior or child snapshots to the worker.
  - The runtime therefore uses process constants and `backup_root: None` instead
    of the complete enqueue-time policy contract.
- Decision:
  - Select Option B, claim plus bounded snapshot readers, as approved by the
    operator.
  - **Option A: one expanded claim result.** Return every scalar behavior field
    from the claim procedure and all bounded child rows as additional claim
    result sets. This minimizes round trips but creates a broad claim contract
    and duplicates scalar values already normalized in snapshot tables.
  - **Option B: claim plus bounded snapshot readers.** Keep job ownership and
    lease acquisition in the claim procedure, then load one scalar behavior row
    and bounded child collections through stored procedures keyed by job public
    id and claim generation. This preserves normalized ownership and isolates
    future policy growth, at the cost of several post-claim reads that must fail
    closed and remain generation-bound.
  - **Option C: read the live profile after claim.** This has the smallest schema
    surface but violates enqueue-time immutability and deterministic retry.
  - Option B keeps claiming
    narrow, makes every persisted snapshot worker-reachable, and supports
    bounded evolution without changing a single oversized claim row.
- Consequences:
  - Option B permits execution, retry, audit, and verification to use the
    exact policy version selected when the job was queued.
  - Option B requires generation-aware stored procedures, typed data rows, and a
    fail-closed aggregate loader before planning begins.
  - Option C leaves the documented determinism and immutable-snapshot contract
    unsatisfied.
- Follow-up:
  - Define the exact stored-procedure and Rust aggregate
    contracts before changing runtime behavior.
  - Keep backup-root path binding separate from portable policy behavior; local
    path resolution requires its own approved decision.

## Task Record

- Motivation:
  - Surface the missing boundary that prevents persisted v1 policy from driving
    production execution.
- Design notes:
  - This decision approves bounded, generation-aware snapshot readers after the
    claim boundary; implementation remains pending.
  - All options preserve the stored-procedure-only runtime database boundary.
- Test coverage summary:
  - Evidence gathered by tracing schema snapshot insertion, the v8 claim
    procedure, `ClaimedMediaJobRow`, and runtime preflight construction.
  - No implementation or behavioral test has been added yet.
- Observability updates:
  - A future implementation must expose stable failure codes for missing,
    incomplete, or generation-mismatched snapshots and record the selected
    snapshot versions in job audit evidence.
- Status-doc validation:
  - `MEDIA_TRANSCODING.md` explicitly requires the affected policy families and
    immutable normalized job snapshots.
- Risk & rollback plan:
  - No runtime risk before approval because this ADR changes no behavior.
  - After implementation, rollback must preserve claimed-job recoverability and
    must not fall back to live policy reads or permissive constants.
- Dependency rationale:
  - No dependency proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; no instruction permits
    bypassing stored procedures or immutable job snapshots.
