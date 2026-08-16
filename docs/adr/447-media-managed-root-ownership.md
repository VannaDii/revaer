# Media managed-root ownership

- Status: Accepted
- Date: 2026-08-13
- Operator approval: Approved as written by the operator on 2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The v1 contract requires validated source, output, workspace, backup, and
    quarantine roots, with local path mappings kept outside portable YAML.
  - The normalized root table accepts all five root kinds, but authored profile
    APIs currently create and return only source and output roots.
  - Production bootstrap injects one process-wide workspace root from
    `REVAER_MEDIA_WORKSPACE_ROOT`; runtime backup is disabled with
    `backup_root: None`, and quarantine is projected inside workspace
    diagnostics.
  - Root ownership determines isolation, deployment configuration, portable
    profile behavior, and how immutable jobs resolve local paths.
- Decision:
  - Select Option C, instance allowlisted roots with per-profile bindings, as
    approved by the operator.
  - **Option A: process-global managed roots.** Bootstrap injects workspace,
    backup, and quarantine roots for the entire instance. Policies only enable
    behavior and set retention/reserve limits. This is operationally simple but
    prevents per-profile storage placement and diverges from the current
    profile-root schema and specification wording.
  - **Option B: per-profile roots.** Every profile owns verified roots of each
    enabled kind, and jobs snapshot those exact paths and filesystem identities.
    This maximizes per-library control but expands API/UI/YAML mapping and can
    duplicate common deployment configuration.
  - **Option C: instance allowlisted roots with per-profile bindings.** Bootstrap
    or stored instance configuration defines trusted root identities; profiles
    bind each root kind to one allowlisted root, and jobs snapshot the resolved
    binding. This separates deployment trust from portable behavior and permits
    profile placement, at the cost of an additional normalized catalog and
    binding workflow.
  - Option C keeps host-specific
    trust under operator control while allowing deterministic per-profile job
    snapshots.
- Consequences:
  - The selected model becomes the source of truth for capacity checks, backup,
    quarantine, workspace retention, YAML import mapping, Helm values, API/UI
    remediation, and job snapshot fields.
  - Option C requires explicit overlap and filesystem-identity validation across
    the instance catalog and profile bindings.
  - No option may permit raw profile paths to bypass validated server policy.
- Follow-up:
  - Reconcile the schema, stored procedures, API, UI, YAML,
    bootstrap, Helm, runtime, and immutable job snapshots in one outside-in
    deliverable sequence.
  - Specify backup collision naming and retention against the selected ownership
    model.

## Task Record

- Motivation:
  - Resolve the contradictory root ownership models before making backup and
    workspace behavior production-reachable.
- Design notes:
  - This ADR intentionally separates path trust and binding from runtime policy
    snapshot transport in ADR 446.
  - Option C is approved; exact schema and bootstrap fields remain implementation
    details constrained by the decision.
- Test coverage summary:
  - Evidence gathered from `MEDIA_TRANSCODING.md`, the normalized root schema,
    authored profile procedures and Rust rows, bootstrap, Helm, and runtime
    preflight construction.
  - No behavior has changed yet.
- Observability updates:
  - A future implementation must expose selected root ids and identity-check
    outcomes without logging sensitive host paths unnecessarily.
- Status-doc validation:
  - The documented v1 configuration model and semantic validation rules were
    checked against current runtime and API reachability.
- Risk & rollback plan:
  - No runtime risk before approval.
  - After implementation, rollback must preserve existing job snapshot path
    resolution and must not silently redirect artifacts to a different root.
- Dependency rationale:
  - No dependency proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`; no scoped rule chooses a root
    ownership model.
