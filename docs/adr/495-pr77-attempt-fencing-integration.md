# PR 77 attempt-fencing integration

- Status: Accepted
- Date: 2026-08-15
- Context:
  - Database-backed replay of PR 77 exposed that its Rust adapters still called
    pre-ADR-419 worker and evidence procedure signatures after migration 0182
    introduced immutable attempts and claim-generation fencing.
  - The same replay showed stale profile tests enabling automation without
    verified roots and a regression where completed-job retention cascaded away
    compact audit facts.
- Decision:
  - Thread the active claim generation through every worker-owned mutation and
    evidence append, and return attempt and claim generations from worker claim.
  - Keep legacy profile creation safe: new profiles remain dry-run-only,
    automation without verified normalized roots fails closed, and legacy root
    mutation remains prohibited.
  - Archive compact audit rows immediately before completed parent jobs are
    deleted, and expose live plus archived rows through the existing stored
    procedure.
  - This task implements the already accepted ADR 419 boundaries. It makes no
    new architectural decision and does not broaden their scope.
- Consequences:
  - Stale workers cannot append evidence or advance lifecycle state through the
    Rust adapter without the active database-issued claim generation.
  - Completed-job cleanup retains its compact audit evidence while detailed
    attempt rows remain bounded by the configured retention policy.
  - Discovery fingerprint tests use a manually invoked, nonautomated profile;
    later normalized-root layers remain responsible for enabling watchers and
    schedules after identity verification.
- Follow-up:
  - Thread the claim generation through dependent runtime and application
    layers as they are replayed.
  - Fold the v0 schema, including the compact-audit archive, into the single
    initialization script before final stack publication.

## Task Record

- Motivation:
  - Make the PR 77 stored-procedure boundary executable against the durable
    schema instead of allowing no-database test skips to conceal stale calls.
- Design notes:
  - Procedure names remain stable; only the claim token required by migration
    0182 is added to adapter inputs.
  - Audit archival is idempotent on job, attempt, and audit index and occurs in
    the same retention transaction before cascading deletion.
- Test coverage summary:
  - Added database-backed coverage for claim-fenced phase, operation,
    violation, plan-reason, verification, artifact, audit, heartbeat,
    cancellation, and status paths.
  - Rechecked immutable policy snapshots, discovery fingerprints, profile
    fail-closed identity rules, and retained compact audits.
- Observability updates:
  - Attempt and claim generations are now available to adapter callers.
  - No log, metric, or event changes were required.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`; its accepted attempt-fencing and retained
    evidence statements already describe the corrected behavior.
- Risk & rollback plan:
  - The primary risk is incorrect argument ordering across adapter and stored
    procedure boundaries. Database-backed tests execute every corrected path.
  - Rollback is a direct revert before dependent runtime layers adopt the
    corrected signatures; after integration, rollback must preserve attempt
    evidence and cannot restore unfenced worker writes.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`.
  - No drift or contradiction was found; no operational policy changed.
