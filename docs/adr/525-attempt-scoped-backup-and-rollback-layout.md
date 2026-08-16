# Attempt-scoped backup and rollback layout

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Accepted ADR 447 requires backup roots to use the managed-root model, and
  accepted ADR 500 requires attempt-and-generation-scoped replacement evidence.
  Neither decision defines backup names, collision behavior, manifest layout,
  rollback ownership, or retention ordering.
- Mirroring source-relative names into a shared backup tree can collide across
  jobs, retries, renamed sources, or profiles. Timestamp suffixes are not unique
  or idempotent and make recovery guess which copy belongs to an attempt.
- Backup retention must never delete the only rollback source while replacement,
  terminal reconciliation, recovery, or a prior cleanup acknowledgement remains
  uncertain.

## Options

1. **Browseable source-relative backup paths with timestamp suffixes.** These are
   easy to inspect but permit collisions, path traversal mistakes, and ambiguous
   retry ownership.
2. **Opaque attempt-scoped bundles with canonical manifests.** Allocate one
   database identity, copy bounded members under ordinal names, atomically
   publish the bundle, and retain it through a fenced cleanup transaction.
3. **Content-addressed global blob storage.** Deduplicate members by digest and
   reference them from attempts. This can save space but introduces reference
   counting, garbage collection, and shared-corruption scope beyond v1.

## Recommendation

- Adopt option 2.
- Every backup is one immutable aggregate bundle owned by `(job UUID, plan
  generation, attempt number, claim generation, source aggregate identity)` and
  one ADR 523 backup-root binding. A database-generated public UUID is the only
  filesystem name component derived from job state.
- The normalized database row has a unique constraint over that ownership tuple
  and records backup public id, root binding and attestation generation, state,
  manifest version and digest, checked byte estimate, actual bytes, member count,
  retention deadline, and lifecycle timestamps.

### Filesystem Layout And Naming

- Under the open backup-root handle, reserve this package-owned namespace:

```text
.revaer-backups/
  v1/
    staging/b-<lowercase-uuid>/
    ready/<first-two-uuid-hex>/b-<lowercase-uuid>/
    deleting/b-<lowercase-uuid>/
    held/b-<lowercase-uuid>/
```

- Directories are created descriptor-relatively with create-new semantics and
  owner-only access. No source path, profile key, filename, timestamp, job text,
  or operator value appears in a directory or member name.
- A staging bundle contains `manifest.rbm1` and `members/00000000.bin` through
  `members/99999999.bin`, using the zero-based manifest ordinal. Member count is
  bounded by the separately accepted source-aggregate contract; eight decimal
  digits are a namespace ceiling, not permission for that many files.
- `manifest.rbm1` is a domain-separated canonical binary frame, not JSON, YAML,
  a native serialization dump, or mutable application state.
- `manifest_version = 1` reuses ADR 517's fixed-width, big-endian, presence, and
  length primitives. It begins with the ASCII bytes
  `revaer-media-backup-manifest`, a zero byte, and an unsigned 32-bit version.
  UUIDs are fixed 16-byte values and SHA-256 digests are fixed 32-byte values.
- After the header, top-level fields occur in this fixed order: backup public
  UUID, job UUID, plan generation, attempt number, claim generation, backup-root
  binding identity, root-attestation generation, policy identity,
  source-aggregate identity, member count, total bytes, member records in
  ascending ordinal, and a SHA-256 footer over all preceding frame bytes.
- Each member record has this fixed order: ordinal, kind, normalized
  source-root-relative path, byte length, SHA-256 digest, descriptor
  observations, mode, optional UID, optional GID, and modification time.
  Nullable UID and GID use ADR 517's one-byte presence marker and are present
  only when policy and platform authorize their capture and restoration.
- V1 restoration metadata is exactly mode, authorized optional UID and GID, and
  modification time. A policy requiring ACL or extended-attribute preservation
  is unsupported in v1 and fails closed before backup admission pending a
  separate approved ADR. If supported required state cannot be captured and
  restored exactly, backup admission fails before replacement.

### Durable Publication And Collision Handling

1. A generation-fenced stored procedure allocates the unique ownership row in
   `staging` after exact capacity admission against the snapshotted backup reserve.
2. The injected filesystem service opens every enrolled source member under the
   cooperative ADR 500 aggregate lease, copies it to its create-new ordinal file,
   hashes while copying, verifies source identity did not change, and fsyncs each
   file and directory.
3. It writes and fsyncs the canonical manifest last, then reopens and verifies
   every member count, size, digest, and required restoration field.
4. It atomically renames the staging directory to the sharded `ready` path,
   fsyncs the namespace, and acknowledges the exact manifest digest and bytes in
   a generation-fenced stored procedure.
5. Replacement preparation cannot begin until the database reports `ready` for
   the current ownership tuple.
- If any expected staging or ready path already exists, an exact manifest and
  member verification may resume the same database identity idempotently. A
  missing, malformed, different, extra, symlinked, or unverifiable entry returns
  `media_backup_namespace_collision`, moves no source file, adds no numeric or
  timestamp suffix, and holds the root from destructive work pending recovery.
- Partial staging that cannot be tied exactly to its database identity is moved
  only by an approved recovery owner to `held`; ordinary execution and retention
  never guess or overwrite it.

### Rollback Contract

- Rollback reads only a database-owned `ready` bundle whose manifest, root
  attestation, source aggregate, and replacement transaction identities all
  match. It verifies the complete bundle before restoring any destination.
- Restore is an aggregate transaction: materialize all members in an
  attempt-scoped recovery workspace, verify bytes and metadata, then use the
  accepted replacement transaction to restore the complete enrolled aggregate.
  Partial member-by-member in-place restore is forbidden.
- The backup bundle remains immutable and `ready` after successful rollback. A
  normalized rollback event records the manifest digest and result; the bundle
  remains subject to its original retention deadline so recovery evidence is not
  destroyed by the act of using it.
- A failed or interrupted rollback preserves the bundle and replacement evidence
  and blocks further destructive work for that aggregate until recovery
  classifies it.

### Retention Transaction

- A backup is protected while any attempt, checkpoint, replacement, rollback,
  outbox, root recovery, compact audit, operator hold, or unresolved cleanup row
  references it. Terminal status alone is insufficient.
- `eligible_at` is the later of the snapshotted backup-retention deadline and the
  durable replacement/rollback reconciliation timestamp. Retention days remain
  bounded to 1 through 3,650. Policy changes never shorten an existing bundle's
  snapshotted deadline.
- Cleanup is per bundle and preservation-biased:
  1. A stored procedure takes a coherent protection snapshot and claims one
     eligible `ready` bundle without deleting evidence.
  2. The filesystem cleaner atomically renames it to `deleting`, fsyncs the
     namespace, removes only that exact descriptor-owned tree, and fsyncs again.
  3. A stored procedure acknowledges absence and prunes only backup detail whose
     compact destructive audit is already durable.
- Crash after rename or deletion resumes idempotently from the database claim.
  A missing tree is success only when the exact cleanup row and manifest identity
  prove ownership. One failed bundle does not block unrelated eligible bundles.
- Proposed ADR 513 recommends the same filesystem-before-evidence-pruning order
  for workspaces. This ADR does not approve ADR 513 or its timings; if both are
  accepted, their cleanup claims and root recovery barriers must use one ordering
  discipline without sharing namespaces.

### Stable Errors

- Use bounded codes for `media_backup_binding_missing`,
  `media_backup_capacity_insufficient`, `media_backup_source_changed`,
  `media_backup_copy_incomplete`, `media_backup_manifest_invalid`,
  `media_backup_namespace_collision`, `media_backup_publish_failed`,
  `media_backup_not_ready`, `media_backup_restore_mismatch`,
  `media_backup_restore_incomplete`, `media_backup_protected`, and
  `media_backup_cleanup_incomplete`.
- Metrics expose state and reason only. Paths, names, UUIDs, digests, member
  ordinals, jobs, attempts, and generations are not metric labels.

## Consequences

- Backup retries and replacement recovery have one unambiguous idempotent bundle
  rather than timestamp or path heuristics.
- Opaque member names reduce operator browseability; authenticated APIs and
  bounded manifests provide inspection without weakening namespace safety.
- Whole-aggregate verification and fsync add I/O before destructive work, but a
  backup claim is meaningless without durable exact rollback evidence.
- Retention can reclaim independent bundles safely while preserving uncertain or
  referenced evidence.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only attempt-scoped ownership, exact namespace,
  canonical manifest, publication and collision rules, aggregate rollback,
  retention protection and cleanup order, and stable errors described above.
- Accepted ADRs 447, 448, 500, and 501 remain binding. ADR 517 must be accepted
  before its framing primitives can be shared, and ADR 523 must be accepted
  before a backup root can be authoritative. Proposed ADRs 512 and 513 remain
  pending and are not imported by this record.
- Acceptance would not authorize content-addressed sharing, timestamp suffixes,
  source-relative backup trees, partial rollback, automatic deletion of unknown
  entries, shorter existing retention, or unbounded metadata capture.
- No schema, filesystem, runtime, API, UI, workflow, or deployment behavior may
  change before explicit decision-specific approval.

## Validation

- Proposal validation is documentation-only.
- After approval, test every crash boundary before, during, and after member
  copy, manifest write, fsync, publish rename, database acknowledgement,
  rollback, deleting rename, removal, and cleanup acknowledgement.
- Test same-attempt idempotency and every collision shape: extra file, missing
  file, symlink, wrong manifest, wrong bytes, wrong root, wrong generation, and
  UUID namespace pre-creation.
- Add complete media-and-sidecar rollback fixtures for mode, authorized UID/GID
  presence and absence, modification time, unsupported ACL/xattr fail-closed
  admission, reserve loss, cancellation, and source change under cooperative
  lease.
- Prove protected bundles survive retention, independent failures do not block
  other bundles, and all temporary media is removed after tests.
- An accepted implementation is not complete until focused filesystem/database
  tests, `just ci`, and `just ui-e2e` pass.

## Follow-up

- Obtain explicit operator approval before implementation or status change.
- Decide ADR 523 first and decide proposed ADRs 512 and 513 before selecting
  recovery-leader and cleanup-claim timings.
- Reconcile API diagnostics and operator restore controls with accepted ADR 484
  workflows without exposing raw host paths or permitting manual partial restore.

## Task Record

- Motivation:
  - Complete backup naming, collision, rollback, and retention details deferred
    by accepted ADRs 447 and 500.
- Design notes:
  - Database identity owns one immutable bundle; the filesystem layout contains
    no user-controlled names and every mutation is acknowledgement-backed.
- Test coverage summary:
  - Proposal only; no schema, filesystem, backup, rollback, or media test was
    added.
- Observability updates:
  - No telemetry changes are made now. Future telemetry is bounded to state and
    reason enums as described above.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 446-451, 484, 500, and 501,
    and proposed ADRs 507-516 and 523. No pending recovery model is claimed.
- Risk & rollback plan:
  - This record can be removed with its catalogue entries. A later rollback must
    preserve every bundle an older runtime cannot verify and must not rename it
    into a legacy layout heuristically.
- Dependency rationale:
  - No dependency is proposed. Existing hashing, descriptor, fsync, replacement,
    stored-procedure, and managed-workspace primitives are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and
    `.github/instructions/devops.instructions.md`; no drift or relaxation was
    found.
