# Managed-root catalog and job binding contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Accepted ADR 447 selects instance allowlisted roots with per-profile bindings,
  but it does not choose the catalog authority, exact root identities, or how a
  job captures source, output, workspace, backup, and quarantine state.
- Raw profile paths and one process-global workspace conflict with deterministic
  job snapshots and portable YAML. Database-authored arbitrary paths would also
  let a remote configuration request expand host filesystem authority.
- Backup and quarantine are optional behaviors, while ADR 517 needs an exact
  five-kind snapshot shape rather than ambiguous missing rows.

## Options

1. **Database-authored path catalog.** Authorized API callers create absolute
   host paths and profiles bind them. This is ergonomic but lets application
   configuration expand deployment filesystem authority.
2. **Deployment-authoritative path slots with database attestation and profile
   bindings.** Bootstrap supplies the only path allowlist; the database records
   verified identities and logical bindings; profiles and YAML reference keys.
3. **Environment-only roots.** Bootstrap injects one path per kind without a
   normalized catalog. This is simple but cannot support per-profile placement,
   immutable associations, or operator mapping workflows.

## Recommendation

- Adopt option 2.
- An injected bootstrap `RootCatalogSource` is the sole authority allowed to add
  or change absolute path bytes. Container, Helm, service-manager, or local
  configuration supplies bounded logical slots; API, YAML, policy, discovery,
  and worker requests cannot introduce a path.
- A slot contains a normalized key, allowed root-kind set, absolute path, and
  declared durability and sole-writer class. Keys are 1-64 lowercase ASCII
  letters, digits, and hyphens. An instance has at most 256 slots and each path
  is at most 4,096 UTF-8 bytes.
- Bootstrap resolves and validates each slot through descriptor-relative,
  no-follow operations, then calls stored procedures to record one immutable
  attestation generation. Database rows are evidence and binding targets; they
  cannot expand the injected allowlist.

### Root Attestation

- A valid attestation records catalog public id, logical key, allowed kinds,
  canonical path, filesystem device, inode, mount identity where supported,
  filesystem type, read/write capability, durability class, sole-writer class,
  owner and mode summary, validation timestamp, attestation generation, and a
  SHA-256 identity digest.
- Every path component must resolve without symlinks, the final object must be a
  directory, and the service must have only the access required by its allowed
  kinds. Group- or world-writable ancestry outside an explicitly attested
  deployment boundary is rejected.
- `source` requires read and descriptor-safe identity access. `output`,
  `workspace`, `backup`, and `quarantine` require create-new, fsync, rename,
  deletion, and capacity-probe support appropriate to their use. Destructive
  readiness additionally requires durable and sole-writer attestations.
- Different physical slots may not overlap by equality, ancestor, descendant,
  canonical identity, bind mount, or equivalent filesystem identity. One slot
  may explicitly allow both `source` and `output`; that exact same-slot alias is
  the only permitted source/output overlap for first-release in-place
  replacement.
- Discovery source slots are mutually non-overlapping. Workspace, backup, and
  quarantine slots cannot overlap source/output or one another.

### Profile And Association Bindings

- A profile version binds one output slot, one workspace slot, and optional
  backup and quarantine slots by catalog public id. A discovery association
  binds one source slot plus one normalized root-relative path to exactly one
  profile version.
- First-release destructive output is in-place. Its output binding must be the
  same catalog slot as the association's source binding. A distinct final-output
  root or multiple output files requires a separate architecture decision.
- Portable YAML carries logical keys only. Import resolves keys against the
  current catalog; unresolved or kind-incompatible keys remain disabled drafts.
  Local backup export may include paths only through a separate explicit
  operator action and never makes those paths authoritative on import.
- Updating bootstrap path configuration creates a new attestation generation.
  Existing profile versions continue to reference their recorded catalog ids,
  but new jobs cannot use a stale or unavailable attestation.

### Exact Five-Kind Job Snapshot

- Every job stores exactly one row for each ordered kind: `source`, `output`,
  `workspace`, `backup`, and `quarantine`.
- Each row contains `binding_state = bound | not_required`. Source, output, and
  workspace must be `bound`. Backup is `bound` exactly when backup policy is
  enabled; quarantine is `bound` exactly when quarantine policy is enabled.
  `not_required` carries no catalog id or path identity and is still included in
  aggregate identity.
- A bound row captures catalog public id, logical key, attestation generation,
  allowed kind, canonical path, device, inode, mount identity, filesystem type,
  durability and sole-writer class, root-relative association prefix where
  applicable, and identity digest.
- Enqueue resolves all five rows in the same transaction as policy and target
  snapshots. Missing, stale, disabled, kind-incompatible, overlapping, or
  behavior-incomplete bindings reject job creation.
- At first claim, before workspace admission, and immediately before the first
  mutation, the injected root resolver reopens each required root and compares
  exact snapshot identity. A mismatch fails closed; it does not rewrite the job,
  select a newer attestation, or fall back to an environment path.
- Open root handles, not reconstructed absolute child strings, anchor traversal,
  workspace, backup, quarantine, and replacement operations. Root-relative paths
  reject absolute prefixes, empty components where not allowed, `.` and `..`,
  separators inside a component, NUL, and any symlink traversal.

### Stable Errors

- Use bounded codes for `media_root_slot_unknown`, `media_root_kind_forbidden`,
  `media_root_attestation_invalid`, `media_root_attestation_stale`,
  `media_root_overlap`, `media_root_unsafe_ancestry`,
  `media_root_durability_unproven`, `media_root_writer_control_unproven`,
  `media_root_binding_incomplete`, `media_root_identity_mismatch`, and
  `media_root_relative_path_invalid`.
- General job and metrics surfaces expose logical root kind and reason only. Full
  canonical paths are restricted to the authenticated instance-root
  administration surface and bounded diagnostic evidence.

## Consequences

- Remote configuration cannot expand filesystem authority; deployment config and
  attestation establish trust, while normalized bindings preserve operator
  workflows and immutable jobs.
- Every job has one deterministic five-row shape even when backup or quarantine
  is disabled.
- Path changes and mount replacement become explicit new attestations and block
  stale work rather than redirecting it silently.
- Requiring same-slot source/output constrains first-release output placement but
  matches verified in-place replacement and avoids an unapproved publish model.

## Implementation Boundary

- This accepted ADR authorizes only deployment-authoritative slots, database
  attestation, validation and overlap rules, profile/association binding, exact
  five-kind snapshots, identity revalidation, and stable errors described above.
- Accepted ADRs 446, 447, 451, 484, 500, and 501 remain binding. ADR 517 owns the
  snapshot reader and ADR 525 separately owns backup layout and retention.
- This ADR does not authorize arbitrary API paths, distinct final-output
  roots, path migration, symlink following, ephemeral destructive work,
  automatic watcher activation from ADR 516, or lifecycle behavior owned by ADR
  514.
- Schema, bootstrap, Helm, API, YAML, UI, filesystem, and runtime changes must
  remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only.
- During implementation, test missing, duplicate, nested, aliased, bind-mounted,
  symlinked, non-directory, permission-invalid, ephemeral, and writer-uncontrolled
  slots on every supported platform.
- Test all five kinds, optional states, source/output same-slot alias, forbidden
  overlaps, YAML mapping, disabled drafts, attestation replacement, and exact
  snapshot round trips.
- Race-test root and mount replacement at enqueue, claim, admission, and
  pre-mutation boundaries using descriptor-relative operations.
- Add packaged persistent and disposable volume tests and prove destructive
  readiness never follows from writability alone.
- An accepted implementation is not complete until focused filesystem/database
  tests, `just ci`, and `just ui-e2e` pass.

## Follow-up

- Implement accepted ADRs 517, 521, and 525 with this record so snapshot, operator mapping,
  and backup path ownership share the same root ids.
- Reconcile accepted ADR 514's packaged provisioning text in the same
  implementation sequence.

## Task Record

- Motivation:
  - Complete catalog authority and five-kind binding details deferred by accepted
    ADR 447.
- Design notes:
  - Deployment config grants path authority; the database stores attestations,
    immutable bindings, and audit evidence.
- Test coverage summary:
  - The ADR-only change added no schema, package, filesystem, API, or runtime test.
- Observability updates:
  - Future metrics use root kind and bounded reason only. Paths, root keys,
    identities, devices, mounts, profiles, and jobs must not be labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 446-451, 484, 500, and 501,
    and accepted ADRs 507-516. This ADR imports no unrelated lifecycle or
    discovery behavior and preserves unresolved exact-value holds.
- Risk & rollback plan:
  - Any reversal requires a superseding ADR. A later rollback must preserve
    bound-job identity evidence and hold roots an older runtime cannot attest.
- Dependency rationale:
  - No new dependency is required. Existing bootstrap injection, filesystem
    descriptor, hashing, and stored-procedure facilities are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`; no drift or relaxation was
    found.
