# Dry-run workspace isolation

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The media worker created and populated a managed workspace before production
    preflight, even when the claimed job was an effective dry run.
  - Dry runs must inspect, plan, estimate disk impact, and persist audit state
    without creating candidate files, backups, quarantine artifacts, sidecars,
    replacement files, or managed workspace directories.
  - Planning still needs deterministic future workspace paths and capacity data
    for the configured workspace filesystem.
- Decision:
  - Project deterministic workspace paths without touching the filesystem for
    dry-run jobs; retain trusted managed workspace creation for non-dry jobs.
  - Revalidate the claimed source fingerprint in place for dry-run inspection.
    Revalidate it again after inspection and preflight compilation so a source
    mutation during planning fails before audits or plans are persisted. Only
    non-dry execution captures the source aggregate into managed input.
  - Probe the configured workspace root for dry-run capacity estimation. Keep
    probing the created output directory for non-dry execution.
  - Preserve the existing preflight, plan persistence, compact audit,
    verification-check, telemetry, and terminal event paths for dry runs.
- Consequences:
  - A dry run can complete planning without creating or cleaning a workspace or
    writing any candidate, backup, quarantine, sidecar, or replacement artifact.
  - A configured dry-run workspace root must already exist for the production
    filesystem capacity probe; a missing or unreadable root fails closed without
    creating it.
  - Non-dry execution keeps the existing trusted workspace handles, stable source
    capture, execution, verification, replacement, and cleanup behavior.
- Follow-up:
  - Apply the operator approval recorded on 2026-08-15 when advancing and
    pushing the implementation.
  - Keep future planning adapters read-only and explicitly classify any new
    filesystem operation as planning-safe or execution-only.
  - Extend dry-run mutation assertions when backup, quarantine, or sidecar policy
    snapshots become worker-reachable.

## Task Record

- Motivation:
  - Close the v1 contract gap where a dry run mutated the managed workspace before
    it reached the existing execution-skipping branch.
- Design notes:
  - Reused `project_managed_workspace`, which validates root and job-key path
    syntax and returns the same deterministic paths as creation without filesystem
    mutation.
  - Kept collaborators injected and retained the existing fail-closed source
    fingerprint, capability, capacity, and preflight checks.
  - Did not add a fallback workspace, temporary directory, or alternate source
    copy for dry-run planning.
- Test coverage summary:
  - Strengthened the successful dry-run runtime test to require an unchanged
    filesystem tree and an absent workspace root while still persisting a plan,
    audits, and verification checks.
  - Added a dry-run test using the production filesystem capacity probe; it
    proves planning and persistence complete while the pre-existing workspace
    remains empty and no command executes.
  - Strengthened failed capability preflight coverage to prove failure persistence
    also leaves the filesystem unchanged and creates no diagnostic workspace.
  - Added a source-mutation test proving dry-run inspection revalidates its input
    before persisting a plan, executes no command, and creates no workspace.
  - `just fmt`, `just check`, `just lint`, `just instruction-drift`, `just ci`,
    and `just ui-e2e` are required before handoff; final results are recorded in
    the pull request evidence.
- Observability updates:
  - No metric, event, or log schema changed. Existing dry-run outcome, phase,
    planned-operation, verification-check, and failure telemetry remains active.
  - Workspace cleanup telemetry is not incremented when no workspace was created.
- Status-doc validation:
  - Rechecked `MEDIA_TRANSCODING.md`; the implementation now matches its explicit
    dry-run sequencing and no-artifact contract.
  - No README, roadmap, API, UI, or operator-facing contract required a content
    change.
- Risk & rollback plan:
  - Production dry-run capacity checks now require an existing configured root;
    this is intentional fail-closed validation rather than implicit provisioning.
  - Roll back this change if dry-run planning regresses, then restore it only with
    a read-only planning boundary; do not restore preflight workspace mutation as
    a workaround.
- Dependency rationale:
  - No dependency was added. The change uses the existing workspace projection,
    source fingerprint, capacity probe, preflight, and persistence abstractions.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`.
  - No policy drift, contradiction, or stale operational reference was found;
    no instruction update is required for this runtime-only behavior.
