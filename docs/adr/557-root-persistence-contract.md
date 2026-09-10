# Root persistence, binding, and administration contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: 2026-09-09: "ADRs 557-559 are approved according to the resolution"
- Review revision: 2026-09-09; accepted as reconciled in ADR 559

## Approval Resolution

The operator approved the reconciled R1-R4 contract, including the full appendix,
and the shared G1 boundary in ADR 559. This records architectural authority, not
implemented behavior or passing validation. The unresolved holds below remain
in force. Historical proposal and approval-request wording describes the review
leading to this decision; this resolution controls its approval status.

## Decision Review

This is an accepted contract, not a new implementation. ADRs
523 and 550 already select deployment-owned path authority, verified roots,
logical bindings, and immutable job evidence. The remaining decisions are
listed here so approving those principles is not confused with accepting every
internal identifier in the implementation appendix.

| Decision | Recommendation | Alternative and consequence |
| --- | --- | --- |
| R1: catalog change and recovery | Retain one catalog-wide generation fence, explicit profile/association rebinding, and no automatic revival of queued jobs. | Per-slot continuity could avoid unrelated interruptions, but needs a separate design for mixed generations and job authority. It is not selected here. |
| R2: whole-root discovery | Allow an explicit empty association prefix to select the whole already-approved root; candidate file paths remain nonempty and descriptor-relative. | The August draft required a child prefix and could not select a complete library root. Keeping that restriction would be a deliberate product limitation. |
| R3: path visibility | Retain the existing authenticated API principal for read-only root administration, with no paths in ordinary profiles, jobs, or portable YAML. | A distinct administrator permission would reduce who can inspect paths but introduces an authorization model outside the accepted API boundary. |
| R4: association capacity | Retain the proposed cap of 128 active associations per profile version. | Another bound needs explicit selection and load evidence; 128 is a proposed capacity limit, not measured production throughput. |

R1 means even adding an unused slot changes the catalog and makes prior bindings
stale. A committed source-unavailable transition also breaks continuity: restoring
identical bytes does not revive old bindings. Restart reuses a generation only
when the active source and all attestation evidence still match. Existing jobs
retain their snapshots, stop at their next applicable fence, and require the
approved re-plan/recovery workflow; this is not deletion of job history.

The recommendation favors an auditable single-generation v0 model over
unapproved per-slot continuity. The operator must accept its interruption and
explicit-rebinding cost. In-place source/output, Linux support limits, the
256-slot catalog bound, and the five-kind snapshot are already-approved
constraints, not new capabilities offered by this revision.

### Approval Boundary

- The operator approved R1-R4 according to the reconciled resolution. Implement
  within this contract; do not claim working behavior from approval alone.
- The accepted G1 distinction in [the approval delta](559-media-approval-delta.md)
  separates private internal naming/layout choices from authority, persistence
  semantics, public interfaces, limits, compatibility, and quality criteria.
  Existing exact accepted-contract constraints remain fixed under G1. No change
  to an accepted predecessor ADR is implied beyond the named resolution.
- The appendix below remains the concrete implementation baseline. Its relational
  invariants, transaction/fencing semantics, public contracts, visibility rules,
  and bounds are substantive parts of this contract. G1 permits only
  semantics-preserving internal refinements through normal reviewed changes,
  with the appendix and tests kept synchronized.
- ADR 516 still withholds automatic discovery and destructive aggregate use.
  Approving this root contract alone does not approve those features, complete
  the init cutover, establish runtime evidence, or replace the current goal.

## Implementation Appendix

## Problem

- Accepted ADR 523 selects deployment-authoritative root slots, immutable
  attestations, logical profile and discovery bindings, and exact five-kind job
  snapshots. Accepted ADR 550 selects the startup-only JSON source, source
  digest, evidence classes, and supported Linux package boundary. Neither ADR
  selects the normalized database shape, generation identity, reconciliation
  ABI, HTTP representation, or portable logical-key mapping.
- The current schema does not implement those decisions. Migration 0182 and the
  frozen init candidate make `media_profile_root` a profile-owned path row,
  accept requested and canonical paths from profile procedures, attach watcher
  and schedule rows directly to those paths, and copy an arbitrary number of
  rows into `media_job_root_snapshot`.
- Current Rust, HTTP, YAML, and UI models expose `source_root` and `output_root`
  as caller-authored strings. Discovery requests use profile ids plus absolute
  paths, profile mutation uses `PATCH`, and YAML draft persistence stores path
  placeholders. Those surfaces conflict with ADRs 521, 523, and 550.
- A relational implementation cannot persist `allowed_kinds` as JSON, JSONB, or
  a PostgreSQL array. It also cannot use a successful path write, an old
  database row, or a profile-supplied path as root authority.
- The root source is loaded and proved before database reconciliation, but the
  complete catalog must become current atomically. Partial generations, partial
  allowed-kind rows, and mixed old/new profile bindings must never become job
  admission inputs.
- New catalog generations can make otherwise valid profile and association
  versions stale. The contract must decide whether those versions rebind
  automatically, whether queued jobs follow a new generation, and how an
  operator sees and repairs the state.
- This proposal closes those choices before SQL, Rust, API, YAML, or UI behavior
  is implemented. It does not modify the frozen migration corpus and does not
  authorize implementation by its own existence.

## Current Evidence

- `media_profile` currently owns mutable `source_root`, `output_root`,
  `watcher_enabled`, `schedule_enabled`, `schedule_interval_minutes`, and
  `configuration_version` columns.
- `media_profile_root` currently owns path bytes, device and inode values, one
  `root_kind`, and mutable `enabled` and `identity_verified_at` state. Its add
  and revalidate procedures accept host paths from application calls.
- `media_discovery_schedule` and `media_discovery_watcher` reference one
  `media_profile_root` row. `media_discovery_source_fingerprint` is keyed by a
  mutable profile id and an absolute `source_path`.
- `media_job_root_snapshot` currently copies profile-local rows and has no
  catalog source digest, generation identity, binding state, durability
  evidence, writer-control evidence, or exact five-row constraint.
- `revaer-data::media::profiles` and `revaer-data::media::jobs` call those
  profile-local procedures. `MediaRootIdentityResolver` canonicalizes a path
  and reads device and inode values, but does not implement ADR 550's
  descriptor, source, mount, evidence, or generation contract.
- The authenticated HTTP surface currently exposes profile paths and profile
  booleans. The UI asks an operator to type source and output paths. Portable
  YAML stores path fields or path-shaped placeholders rather than exact logical
  root keys and separate discovery associations.
- The generated 1,624-statement init candidate has SHA-256
  `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`
  and reproduces these legacy structures. It is transition evidence, not
  authority for the final root contract.
- The candidate's `media_key_valid_v1` helper accepts one through 128 bytes and
  underscores. Reusing it would widen the accepted ADR 521/523 root-key grammar
  of one through 64 lowercase ASCII letters, digits, and hyphens. The final init
  therefore needs the root-specific validator named below; the legacy helper is
  not root-contract authority.

## Options

1. **Extend `media_profile_root`.** Add catalog and generation columns while
   retaining profile ownership and path-taking profile procedures. This reduces
   cutover work but preserves two authorities, duplicates one physical slot per
   profile, and cannot represent one slot with several allowed kinds cleanly.
2. **Persist one catalog row with arrays or JSON.** Store the source document or
   its slots as a database aggregate. This mirrors the wire format but violates
   normalized persistence policy and makes constraints, references, and grants
   less precise.
3. **Separate immutable catalog generations, normalized slot attestations,
   versioned logical bindings, and exact job snapshots.** Bootstrap reconciles
   one proved generation; profiles and associations bind logical keys to that
   exact generation; jobs copy five ordered rows without following later state.
4. **Keep the catalog entirely in memory.** This preserves deployment authority
   but cannot support immutable profile versions, portable mapping, job
   evidence, restart diagnostics, or stored-procedure-only admission.

## Recommendation

- Adopt option 3 with the authority, immutable lifecycle, transaction, API,
  bound, and cutover semantics below. Internal names and layouts remain the
  accepted implementation baseline subject only to the narrow G1 distinction.
- Place every relation, helper, trigger, and procedure named by this proposal in
  schema `public`, matching the accepted media procedure surface and using the
  ADR 551 owner/grant model. Names below omit `public.` only for readability;
  changing schema placement or privilege boundaries remains an architectural
  change requiring approval and corresponding fixed search paths.
- Absolute paths remain authored only by ADR 550's startup source. Database
  rows are normalized evidence and references. They never become sufficient
  authority for opening a path; every filesystem use still requires the active
  injected source and descriptor-owned identity proof.
- Catalog generations and their slot attestations are immutable. Only one
  singleton pointer states which complete generation is current. A source
  failure clears that pointer without deleting historical evidence.
- Profile and discovery-association versions store logical keys and the exact
  attestation rows resolved when each version was created. A later generation
  never rewrites or silently reactivates those versions.
- Application-state tables use scalar columns and normalized join rows. JSON,
  JSONB, PostgreSQL arrays, comma-separated kinds, bit fields used as the sole
  persisted kind representation, and generic extension columns are forbidden.
- The accepted ADR 521 phrase "five root binding keys on a profile" is resolved
  as a family bound, not five profile-owned rows: a profile version owns output,
  workspace, optional backup, and optional quarantine; a discovery-association
  version alone owns source. Every job still owns all five ordered snapshot rows.

## Exact Root Kind Relation

- Create ungranted helper
  `media_root_logical_key_valid_v1(value_input text) RETURNS boolean` as
  `LANGUAGE sql IMMUTABLE PARALLEL SAFE` with fixed search path `pg_catalog`.
  It returns true only when the UTF-8 byte length is one through 64 and the value
  matches `^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$`. The expression admits a
  one-character alphanumeric key and rejects whitespace, underscore, non-ASCII,
  uppercase, and leading or trailing hyphens. It is the exact validator for slot
  logical keys, profile root-binding keys, association keys, and association
  source keys. Runtime receives no direct execute grant on this helper.
- Create ungranted helper
  `media_root_relative_path_valid_v1(value_input text) RETURNS boolean` as
  `LANGUAGE sql IMMUTABLE PARALLEL SAFE` with fixed search path `pg_catalog`.
  It accepts exactly one through 4,096 UTF-8 bytes, rejects an initial or final
  `/`, `//`, NUL, backslash, and any `/`-separated component equal to `.` or
  `..`. Runtime receives no direct execute grant on this helper.
- Create ungranted helper
  `media_root_relative_prefix_valid_v1(value_input text) RETURNS boolean` with
  the same language, immutability, and fixed-search-path rules. It accepts the
  empty string or a value accepted by `media_root_relative_path_valid_v1`.
  Empty means the entire descriptor-owned root, never the process working
  directory. SQL NULL is not a prefix and is never converted to empty. This
  helper is for association prefixes only; candidate files keep the nonempty
  path validator. No slash, dot, dot-dot, or other alternate spelling denotes
  the whole root.

- Create `media_root_kind` with exactly these columns:

  | Column | Type | Contract |
  | --- | --- | --- |
  | `media_root_kind_id` | `smallint` | primary key and canonical ordinal |
  | `root_kind` | `text not null` | unique, nonempty name |

- Seed exactly these immutable rows in this order:

  | `media_root_kind_id` | `root_kind` |
  | --- | --- |
  | 1 | `source` |
  | 2 | `output` |
  | 3 | `workspace` |
  | 4 | `backup` |
  | 5 | `quarantine` |

- Use constraint names `media_root_kind_pkey`,
  `media_root_kind_root_kind_key`, and `media_root_kind_closed_v1`. The closed
  constraint requires the exact id/name pair. Trigger
  `media_root_kind_immutable_trigger`, backed by
  `media_root_kind_immutable_v1`, rejects update and delete.
- The numeric id is the ADR 517 root-family order. No other ordinal, enum,
  array position, lexical sort, or caller-provided order is authoritative.

## Exact Catalog Relations

### `media_root_catalog_generation`

- Create this immutable generation table with columns in this order:

  | Column | Type | Nullability and meaning |
  | --- | --- | --- |
  | `media_root_catalog_generation_id` | `bigint generated always as identity` | primary key and positive attestation generation |
  | `media_root_catalog_generation_public_id` | `uuid` | not null, `gen_random_uuid()`, unique public identity |
  | `contract_version` | `smallint` | not null, exactly `1` |
  | `source_format_version` | `smallint` | not null, exactly `1` |
  | `source_sha256` | `bytea` | not null, exact 32-byte ADR 550 semantic digest |
  | `attestation_sha256` | `bytea` | not null, exact 32-byte aggregate attestation digest |
  | `generation_sha256` | `bytea` | not null, exact 32-byte generation identity |
  | `slot_count` | `smallint` | not null, zero through 256 |
  | `activated_at` | `timestamptz` | not null, transaction timestamp of first activation |

- Use these constraints and indexes exactly:
  - `media_root_catalog_generation_pkey`;
  - `media_root_catalog_generation_public_id_key`;
  - `media_root_catalog_generation_contract_v1`;
  - `media_root_catalog_generation_source_format_v1`;
  - `media_root_catalog_generation_source_sha256_length`;
  - `media_root_catalog_generation_attestation_sha256_length`;
  - `media_root_catalog_generation_generation_sha256_length`;
  - `media_root_catalog_generation_slot_count_bounds`; and
  - `ix_media_root_catalog_generation_generation_sha256` on
    `(generation_sha256, media_root_catalog_generation_id DESC)`.
- Do not make any digest globally unique. An idempotent restart reuses only the
  currently active generation when all three digests and the slot count match.
  Returning to older source and evidence bytes after another generation creates
  a new, higher attestation generation and cannot reactivate old bindings.

### `media_root_catalog_slot`

- Create one stable identity row per logical key with:
  - `media_root_catalog_slot_id bigint generated always as identity`;
  - `media_root_catalog_slot_public_id uuid not null default gen_random_uuid()`;
  - `logical_key text not null`; and
  - `created_at timestamptz not null default transaction_timestamp()`.
- Use `media_root_catalog_slot_pkey`,
  `media_root_catalog_slot_public_id_key`,
  `media_root_catalog_slot_logical_key_key`, and
  `media_root_catalog_slot_logical_key_contract`. The key constraint calls the
  exact `media_root_logical_key_valid_v1` contract rather than the wider legacy
  helper.
- Slot identity and key are immutable. A key removed from a later source remains
  historical; reintroducing the same key reuses its stable slot identity but
  receives a new attestation generation.

### `media_root_catalog_slot_attestation`

- Create one immutable attestation row per slot in each complete generation:

  Every column below is `NOT NULL`; defaults and caller timestamps are forbidden.

  | Column | Type | Contract |
  | --- | --- | --- |
  | `media_root_catalog_slot_attestation_id` | `bigint generated always as identity` | primary key |
  | `media_root_catalog_generation_id` | `bigint` | not null, restrictive FK |
  | `media_root_catalog_slot_id` | `bigint` | not null, restrictive FK |
  | `requested_path` | `text` | exact decoded ADR 550 path |
  | `canonical_path` | `text` | descriptor-proved absolute path |
  | `filesystem_device` | `bytea` | exactly eight unsigned big-endian bytes |
  | `filesystem_inode` | `bytea` | exactly eight unsigned big-endian bytes |
  | `mount_id` | `bigint` | Linux mount identity from zero through 9,223,372,036,854,775,807 |
  | `filesystem_type` | `text` | 1-64 UTF-8 bytes |
  | `read_capable` | `boolean` | exact probe result |
  | `write_capable` | `boolean` | exact probe result |
  | `create_new_capable` | `boolean` | exact probe result |
  | `fsync_capable` | `boolean` | exact probe result |
  | `rename_capable` | `boolean` | exact probe result |
  | `delete_capable` | `boolean` | exact probe result |
  | `capacity_probe_capable` | `boolean` | exact probe result |
  | `durability_class` | `text` | ADR 550 closed value |
  | `durability_evidence` | `text` | ADR 550 closed value |
  | `sole_writer_class` | `text` | ADR 550 closed value |
  | `sole_writer_evidence` | `text` | ADR 550 closed value |
  | `owner_uid` | `bigint` | zero through 4,294,967,295 |
  | `owner_gid` | `bigint` | zero through 4,294,967,295 |
  | `mode_bits` | `integer` | zero through 4,095 |
  | `validated_at` | `timestamptz` | database transaction timestamp after proof |
  | `root_identity_sha256` | `bytea` | exact 32-byte slot-attestation identity |

- Use these exact constraints:
  - `media_root_catalog_slot_attestation_pkey`;
  - `media_root_catalog_slot_attestation_generation_fkey`;
  - `media_root_catalog_slot_attestation_slot_fkey`;
  - `media_root_catalog_slot_attestation_generation_slot_key` on
    `(media_root_catalog_generation_id, media_root_catalog_slot_id)`;
  - `media_root_catalog_slot_attestation_generation_path_key` on
    `(media_root_catalog_generation_id, canonical_path)`;
  - `media_root_catalog_slot_attestation_generation_identity_key` on
    `(media_root_catalog_generation_id, filesystem_device, filesystem_inode)`;
  - `media_root_catalog_slot_attestation_generation_digest_key` on
    `(media_root_catalog_generation_id, root_identity_sha256)`;
  - `media_root_catalog_slot_attestation_paths` for absolute, non-root,
    NUL-free, 1-4,096 byte requested and canonical paths;
  - `media_root_catalog_slot_attestation_filesystem_identity` for both exact
    eight-byte values, nonnegative mount id, and bounded filesystem type;
  - `media_root_catalog_slot_attestation_owner_mode_bounds`;
  - `media_root_catalog_slot_attestation_durability_pair` for exactly
    `disposable/none`,
    `restart_persistent/linux_dedicated_mount`, or
    `restart_persistent/kubernetes_persistent_volume_claim`;
  - `media_root_catalog_slot_attestation_writer_pair` for exactly
    `uncontrolled/none`,
    `revaer_exclusive/linux_dedicated_service`, or
    `revaer_exclusive/kubernetes_read_write_once_pod`; and
  - `media_root_catalog_slot_attestation_digest_length`.
- Add `ix_media_root_catalog_slot_attestation_slot_generation` on
  `(media_root_catalog_slot_id, media_root_catalog_generation_id DESC)` and
  `ix_media_root_catalog_slot_attestation_generation_path` on
  `(media_root_catalog_generation_id, canonical_path,
  media_root_catalog_slot_attestation_id)`.

### `media_root_catalog_slot_kind`

- Persist allowed kinds only as join rows:
  - `media_root_catalog_slot_attestation_id bigint not null`;
  - `media_root_kind_id smallint not null`; and
  - primary key
    `(media_root_catalog_slot_attestation_id, media_root_kind_id)`.
- Name the foreign keys `media_root_catalog_slot_kind_attestation_fkey` and
  `media_root_catalog_slot_kind_kind_fkey`, both restrictive. Add
  `ix_media_root_catalog_slot_kind_kind` on
  `(media_root_kind_id, media_root_catalog_slot_attestation_id)`.
- Finalization requires one through five kind rows per attestation. It validates
  capability and evidence coherence for every kind. A source-only attestation
  may be `uncontrolled`; output, workspace, backup, and quarantine require every
  write capability and `revaer_exclusive`. Any profile or job usable for
  destructive replacement additionally requires `restart_persistent` for all
  bound rows.
- Kind-level `binding_ready` means source has read capability or a writing kind
  has every write capability plus `revaer_exclusive`. `destructive_ready` also
  requires `restart_persistent`; source is destructive-ready only when the same
  attestation has an output kind row and satisfies the output requirements.

### `media_root_catalog_state`

- Create exactly one mutable pointer row with:
  - `media_root_catalog_state_id smallint primary key` equal to `1`;
  - `active_media_root_catalog_generation_id bigint null` with a restrictive
    FK;
  - `source_state text not null`;
  - `source_reason_code text null`;
  - `attestation_state text not null`;
  - `attestation_reason_code text null`; and
  - `reconciled_at timestamptz not null`.
- Final init seeds exactly
  `(1, NULL, 'missing', 'media_root_catalog_source_missing',
  'not_evaluated', NULL, transaction_timestamp())`. This fail-closed
  pre-bootstrap value is replaced in the first reconciliation transaction and is
  never interpreted as evidence that the packaged source was already read.
- `source_state` is exactly `ready`, `missing`, `untrusted`, `invalid`,
  `bound_exceeded`, or `unsupported`. `attestation_state` is exactly `ready`,
  `not_evaluated`, or `invalid`. Use constraints
  `media_root_catalog_state_pkey`,
  `media_root_catalog_state_active_generation_fkey`,
  `media_root_catalog_state_singleton`,
  `media_root_catalog_state_source_known`,
  `media_root_catalog_state_attestation_known`,
  and `media_root_catalog_state_coherent`.
- A non-ready source requires a null active generation, its exact ADR 550 reason,
  and `attestation_state = 'not_evaluated'` with no attestation reason:
  `media_root_catalog_source_missing`,
  `media_root_catalog_source_untrusted`,
  `media_root_catalog_format_invalid`,
  `media_root_catalog_bound_exceeded`, or
  `media_root_platform_unsupported`.
- A ready source has no source reason. Ready attestation requires a nonnull active
  generation and no attestation reason. Invalid attestation requires a null
  active generation and exactly one of `media_root_attestation_invalid`,
  `media_root_overlap`, `media_root_unsafe_ancestry`,
  `media_root_durability_unproven`,
  `media_root_writer_control_unproven`, or
  `media_root_identity_mismatch`. No other source/attestation/pointer combination
  satisfies `media_root_catalog_state_coherent`.
- A valid version 1 document with zero slots creates a ready zero-slot generation;
  it is observably different from a missing document, which has no active
  generation and the source-missing reason.
- Source or attestation failure clears only the current pointer. It never deletes
  a generation, slot, binding, or job snapshot. Catalog generation, slot, attestation, and kind
  rows are insert-only and retained indefinitely in v0 while any reference or
  audit may exist. Pruning requires a later approved decision.

### Immutability

- One trigger function `media_root_catalog_immutable_v1` rejects update and
  delete on `media_root_catalog_generation`, `media_root_catalog_slot`,
  `media_root_catalog_slot_attestation`, and
  `media_root_catalog_slot_kind`.
- Use trigger names formed as `<table>_immutable_trigger`. The singleton state
  table is the only mutable root-catalog relation.
- No application principal receives table or sequence privileges. Immutability
  triggers are defense in depth behind stored-procedure-only access and owner
  separation.

## Digest And Generation Identity

- Continue to compute `source_sha256` exactly as ADR 550 specifies. Raw JSON
  bytes, object order, whitespace, and array order never become database
  identity.
- Slot attestation contract version 1 uses ADR 517's fixed-width, big-endian,
  presence, and length primitives. It begins with ASCII
  `revaer-media-root-slot-attestation`, one zero byte, and unsigned 32-bit
  version `1`.
- The slot frame then contains, in order: logical key, requested path, canonical
  path, one allowed-kind mask byte, eight device bytes, eight inode bytes,
  unsigned 64-bit mount id, filesystem type, one capability-mask byte,
  durability class byte, durability evidence byte, sole-writer class byte,
  sole-writer evidence byte, unsigned 32-bit owner UID, unsigned 32-bit owner
  GID, and unsigned 32-bit mode bits. Allowed-kind bits zero through four map to
  root-kind ids one through five; bits five through seven are zero. Capability
  bits zero through six represent the seven boolean columns in table order and
  bit seven is zero. Enum bytes are exactly ADR 550's assignments. Timestamps
  and database ids are excluded.
- `root_identity_sha256` is SHA-256 over that complete slot frame.
- Aggregate attestation version 1 begins with ASCII
  `revaer-media-root-attestation`, one zero byte, unsigned 32-bit version `1`,
  the 32 source-digest bytes, unsigned 32-bit slot count, then every slot sorted
  by decoded logical-key bytes. Each slot contributes its length-prefixed logical
  key and 32-byte `root_identity_sha256`. `attestation_sha256` is SHA-256 over
  that frame.
- Generation identity version 1 begins with ASCII
  `revaer-media-root-generation`, one zero byte, unsigned 32-bit version `1`,
  the 32 source-digest bytes, and the 32 attestation-digest bytes.
  `generation_sha256` is SHA-256 over that frame.
- Rust and PostgreSQL independently recompute every digest from normalized
  arguments before activation. A byte-count, order, kind-mask, field, digest,
  or slot-count mismatch rolls back the complete reconciliation.
- `media_root_catalog_generation_id` is the positive, monotonic
  `attestation_generation` exposed by procedures and APIs. Public UUID and
  numeric id identify one generation occurrence. `generation_sha256` identifies
  semantic source-plus-attestation content and may repeat when old bytes return;
  it does not replace the exact numeric fence stored in bindings and jobs.

## Atomic Bootstrap Reconciliation

- Root reconciliation runs once after ADR 541 baseline verification and before
  media profiles, discovery, workers, retention, or media-ready health are
  constructed. The source and every slot are already held by stable descriptors
  and root locks while reconciliation executes.
- The data adapter opens one `SERIALIZABLE` transaction and invokes only the
  procedures below. The begin procedure obtains
  `pg_advisory_xact_lock(hashtextextended('media_root_catalog_reconcile_v1', 0))`.
  Every later reconciliation procedure requires the same transaction and locked
  singleton row.
- Reconciliation is ordered: begin generation, append slots in logical-key
  order, append each slot's kind rows in root-kind order, finalize and activate,
  commit. A failure before commit leaves no partial generation and retains the
  prior state.
- If the current generation already has exact source, attestation, generation
  digests, and slot count, begin returns `already_current = true`. The caller
  appends nothing; finalize verifies the active rows and updates only the
  singleton `reconciled_at`. A digest found only in an older generation is not
  idempotent and creates a new generation.
- Any committed unavailable transition ends current-generation continuity. A
  later source with identical semantic and attestation digests creates a new
  occurrence and leaves prior bindings stale; it does not restore the cleared
  pointer to the historical row.
- On that idempotent path, each immutable `validated_at` remains the first proof
  time for its generation occurrence; singleton `reconciled_at` is the latest
  successful complete descriptor revalidation time. Administration exposes both
  and never relabels the older value as the latest probe.
- Database lock acquisition and transaction commit order define reconciliation
  order; digest values do not imply recency. Every caller must already hold the
  accepted package-level single-instance and root locks. If differently sourced
  callers nevertheless serialize, the final successful lock holder creates and
  activates a new occurrence even when its semantic digest appeared before; an
  unavailable result acquired last clears the pointer. No historical row is
  reactivated, and queued work remains fenced by its numeric generation.
- A missing or rejected source calls the bounded unavailable procedure in one
  transaction. It clears the active pointer and records only the closed state,
  reason, and database transaction time. It accepts no path, digest, key,
  caller timestamp, or diagnostic text.
- A syntactically valid source whose descriptor, ancestry, overlap, capability,
  durability, writer-control, or identity proof fails first rolls back any
  candidate reconciliation, then calls the bounded attestation-invalid procedure
  while retaining the same process/root locks. That second transaction clears
  the pointer without misclassifying the source document. An unexpected database
  defect prevents media startup; the process refuses all root-bound work even if
  the fail-closed state mutation itself could not commit.
- Immediately before activation, bootstrap revalidates that every held
  descriptor still matches the proved identity. Activation validates exact row
  counts, one through five allowed kinds per
  slot, source/output dual-kind coherence, all capability/evidence pairs,
  duplicate and overlap absence, independent PostgreSQL digests, and the
  normalized proof fields. The state pointer changes only after every check, and
  all persisted reconciliation timestamps come from `transaction_timestamp()`.
- Application code cannot use a staged generation. No procedure returns staged
  rows to profile, association, job, readiness, or administration readers.

## Profile-Version Root Bindings

- ADR 521's profile parent keeps one public id and immutable profile key. The
  root contract requires exact nullable head columns
  `latest_media_profile_version_id bigint` and
  `active_media_profile_version_id bigint`. The latest head is the version used
  for `GET`, ETag, and the next `PUT`; the active head is the only version usable
  by a new association or job. They are equal after a successful active create
  or replacement. An unresolved YAML draft advances only the latest head. An
  archive version advances the latest head and clears the active head.
- ADR 521's immutable `media_profile_version` relation has
  `media_profile_version_id bigint generated always as identity`,
  `media_profile_id bigint not null`, positive `version integer not null`,
  `lifecycle_state text not null`, and the accepted non-root aggregate fields.
  Use `media_profile_version_pkey`, restrictive
  `media_profile_version_media_profile_fkey`,
  `media_profile_version_profile_version_key` on
  `(media_profile_id, version)`, and
  `media_profile_version_profile_id_version_id_key` on
  `(media_profile_id, media_profile_version_id)`. Lifecycle is exactly `draft`,
  `active`, or `archived` under
  `media_profile_version_lifecycle_known`. Other accepted profile fields and
  constraints retain their owning ADRs and are not redefined here.
- `media_profile_version_media_profile_fkey` maps `media_profile_id` to
  `media_profile(media_profile_id)` with delete restricted.
- Add restrictive composite foreign key `media_profile_latest_version_fkey` from
  `(media_profile_id, latest_media_profile_version_id)` and
  `media_profile_active_version_fkey` from
  `(media_profile_id, active_media_profile_version_id)` to
  `media_profile_version(media_profile_id, media_profile_version_id)`. A profile
  mutation procedure
  locks the parent, requires `version = prior latest version + 1`, inserts one
  immutable version and all children, then changes the heads atomically. A head
  never points to another profile's version.
- Create `media_profile_version_root_binding` with:
  - `media_profile_version_id bigint not null`;
  - `media_root_kind_id smallint not null`;
  - `logical_key text not null`;
  - `media_root_catalog_slot_attestation_id bigint null`;
  - `resolution_state text not null`; and
  - `created_at timestamptz not null default transaction_timestamp()`.
- Use primary key `media_profile_version_root_binding_pkey` on
  `(media_profile_version_id, media_root_kind_id)`, restrictive foreign keys
  named `media_profile_version_root_binding_profile_version_fkey`,
  `media_profile_version_root_binding_kind_fkey`, and
  `media_profile_version_root_binding_attestation_fkey`, plus
  `ix_media_profile_version_root_binding_attestation` on
  `(media_root_catalog_slot_attestation_id, media_profile_version_id)`.
- Those foreign keys respectively target
  `media_profile_version(media_profile_version_id)`,
  `media_root_kind(media_root_kind_id)`, and
  `media_root_catalog_slot_attestation(media_root_catalog_slot_attestation_id)`;
  every delete is restrictive.
- `media_profile_version_root_binding_kind` permits only output, workspace,
  backup, and quarantine ids. Source belongs only to a discovery association.
  `media_profile_version_root_binding_key_contract` calls
  `media_root_logical_key_valid_v1`.
  `media_profile_version_root_binding_resolution_known` permits `resolved`,
  `unmapped`, or `kind_forbidden`.
  `media_profile_version_root_binding_resolution_coherent` requires an
  attestation exactly for `resolved` and no attestation for the two unresolved
  states.
- Every active profile version has exactly one resolved output and workspace
  row. Backup exists exactly when its immutable policy enables backup;
  quarantine exists exactly when its immutable policy enables quarantine. Draft
  versions may preserve an unresolved row. Archived versions remain immutable.
- A logical key resolves only against the active catalog generation and an
  attestation containing the requested kind. The procedure independently checks
  that the stored slot's immutable logical key equals the caller's key.
- A new catalog generation does not mutate `resolution_state`. Freshness is a
  derived comparison between the binding's attestation generation and the
  singleton active generation. Repair always appends a new profile version under
  ADR 521's strong `If-Match` contract.
- Profile-version and binding rows are immutable. Head changes do not rewrite a
  prior version's lifecycle value; "active" means that version was admitted as
  operationally complete, while the parent's active head selects whether it is
  still the profile's current operational version.

## Discovery-Association Binding

- Replace profile-local source ownership with these normalized resources:
  - `media_discovery_association` owns
    `media_discovery_association_id bigint generated always as identity`,
    `media_discovery_association_public_id uuid not null default
    gen_random_uuid()`, `association_key text not null`, nullable
    `latest_media_discovery_association_version_id bigint`, nullable
    `active_media_discovery_association_version_id bigint`,
    `created_by_user_id bigint not null`, and
    `created_at timestamptz not null default transaction_timestamp()`;
  - `media_discovery_association_version` owns
    `media_discovery_association_version_id bigint generated always as identity`,
    `media_discovery_association_id bigint not null`, positive
    `version integer not null`, `lifecycle_state text not null`,
    `media_profile_version_id bigint not null`,
    `source_logical_key text not null`, nullable
    `media_root_catalog_slot_attestation_id bigint`,
    `resolution_state text not null`, `root_relative_path text not null`,
    `manual_enabled boolean not null`, `watcher_enabled boolean not null`,
    `schedule_enabled boolean not null`, `created_by_user_id bigint not null`, and
    `created_at timestamptz not null default transaction_timestamp()`.
- Use primary and unique constraints
  `media_discovery_association_pkey`,
  `media_discovery_association_public_id_key`,
  `media_discovery_association_association_key_key`, and
  `media_discovery_association_version_pkey`,
  `media_discovery_association_version_association_version_key` on
  `(media_discovery_association_id, version)`, and
  `media_discovery_association_version_parent_id_key` on
  `(media_discovery_association_id,
  media_discovery_association_version_id)`.
- Use restrictive foreign keys
  `media_discovery_association_created_by_user_id_fkey`,
  `media_discovery_association_version_association_fkey`,
  `media_discovery_association_version_profile_version_fkey`,
  `media_discovery_association_version_source_attestation_fkey`,
  `media_discovery_association_version_created_by_user_id_fkey`,
  `media_discovery_association_latest_version_fkey`, and
  `media_discovery_association_active_version_fkey`. The two head foreign keys
  are composite on association id plus version id, so neither can point at
  another association's version.
- The creator keys target `app_user(user_id)`; the version association key targets
  `media_discovery_association(media_discovery_association_id)`; the profile key
  targets `media_profile_version(media_profile_version_id)`; and the source-
  attestation key targets
  `media_root_catalog_slot_attestation(media_root_catalog_slot_attestation_id)`.
  The latest and active head keys respectively map
  `(media_discovery_association_id,
  latest_media_discovery_association_version_id)` and
  `(media_discovery_association_id,
  active_media_discovery_association_version_id)` to the version table's
  `(media_discovery_association_id,
  media_discovery_association_version_id)` unique key. Every delete is
  restrictive.
- Add
  `ix_media_discovery_association_version_profile` on
  `(media_profile_version_id, media_discovery_association_version_id)` and
  `ix_media_discovery_association_version_source_prefix` on
  `(media_root_catalog_slot_attestation_id, root_relative_path,
  media_discovery_association_version_id)`.
- `media_discovery_association_key_contract` and
  `media_discovery_association_version_source_key_contract` call
  `media_root_logical_key_valid_v1`.
  `media_discovery_association_version_positive` requires a positive version.
  `media_discovery_association_version_lifecycle_known` and
  `media_discovery_association_version_resolution_known` use the profile closed
  values. `media_discovery_association_version_resolution_coherent` requires an
  attestation exactly for `resolved`.
  `media_discovery_association_version_relative_path_contract` calls
  `media_root_relative_prefix_valid_v1`. An explicitly supplied empty string
  selects the whole source slot. A nonempty prefix is valid UTF-8 of one through
  4,096 bytes, non-absolute, NUL-free, with `/`-separated nonempty components,
  no `.` or `..` component, and no backslash. Omitted or null prefixes are
  invalid; no default silently broadens discovery to the whole root.
- An active association requires a resolved current-generation source
  attestation and a source kind row. `manual_enabled` may be true or false so an
  operator can retain a resolved but disabled association. Draft and archived
  versions cannot enable any trigger. `watcher_enabled` and
  `schedule_enabled` remain false under constraint
  `media_discovery_association_version_automatic_discovery_held` until ADR 516's
  held numeric values and activation proof receive separate implementation
  approval. This proposal does not select those values.
- An active association binds one exact profile version. Its source attestation
  and that profile version's output attestation must be the same attestation row,
  and the slot must contain both source and output kind rows. Multiple current
  associations may reference one profile version only when they use that same
  slot and their root-relative prefixes are pairwise non-overlapping.
- At most 128 current, non-archived associations may reference one profile
  version. "Current" here means selected by each parent's active head, not every
  historical row whose immutable lifecycle value is `active`. This is checked
  under the same advisory lock as association overlap and matches ADR 521's
  bounded configuration-resource model.
- The exact checks are
  `media_discovery_association_version_automatic_discovery_held` and
  `media_discovery_association_version_activation_coherent`. The latter requires
  an `active` version to be resolved with both automatic flags false and requires
  `draft` and `archived` versions to disable all three trigger booleans. Parent
  latest/active heads follow the same draft, active, and archive semantics as
  profile heads.
- Discovery fingerprint state is re-keyed at the later cutover by
  `media_discovery_association_version_id` and canonical source-relative path.
  It never uses mutable profile identity plus an absolute path as its logical
  key. Exact aggregate-fingerprint fields remain owned by ADRs 516 and 535.

## Overlap And Same-Slot Rules

- Catalog finalization rejects equality, ancestor, descendant, canonical
  device/inode alias, bind-mount alias, and equivalent mount identity across
  two slot attestations in the same generation. The Rust resolver owns
  descriptor and mount-table alias proof. The procedure independently rejects
  canonical-path equality/ancestry and repeated device/inode identity and
  verifies the supplied digests; either layer's rejection aborts reconciliation.
  A shared `mount_id` alone is not overlap: distinct non-overlapping directories
  on one otherwise accepted filesystem may be separate slots.
- One attestation may have both source and output join rows. That one row is the
  only source/output overlap. The database never stores duplicate source and
  output slot attestations for the same physical directory.
- Workspace, backup, and quarantine attestations must each be physically
  distinct from every other slot. The global generation overlap check enforces
  this before profiles can bind them.
- Current discovery-association prefixes on the same source attestation may not
  be equal, ancestor, or descendant. The version-creation procedure locks the
  current association heads in `(source attestation id, root-relative path,
  association id)` order and rejects overlap with
  `media_configuration_overlap`.
- The empty prefix contains every relative member and overlaps every other
  current prefix on that source attestation. Overlap checks use component
  ancestry, not raw string prefix matching, and include whole-root associations
  across profile versions. Whole-root selection grants no access outside the
  already-attested slot and does not loosen candidate traversal checks.
- API and YAML root-binding mutations use one lock order: fence and lock the
  catalog singleton, resolve sources against that generation, acquire all
  required source-attestation advisory locks in ascending attestation-id order,
  and then lock resource parents in the accepted kind/key order. No path locks
  resource parents before acquiring its required source locks.
- Before reading association heads, activation obtains the advisory lock
  computed by `hashtextextended('media_discovery_association_overlap_v1',
  media_root_catalog_slot_attestation_id)` through `pg_advisory_xact_lock` in
  that common order. The source-attestation lock is
  shared by every profile selecting that source, so two different profile
  versions cannot concurrently approve overlapping prefixes. The same critical
  section and locked profile parent enforce the 128-active-association bound.
  Draft creation need not take this lock until the draft is activated.
- A profile/association pair never derives a distinct output path. Output is the
  same descriptor-owned source member for ADR 523's first-release in-place
  replacement. Distinct final output, copy publishing, cross-slot moves, or
  multiple output artifacts remain unauthorized.

## Exact Five-Row Job Root Snapshot

- Rebuild `media_job_root_snapshot` at the pre-v1 init cutover with these
  columns in order:

  `media_job_id`, `media_root_kind_id`, and `binding_state` are `NOT NULL`.
  Every other column is nullable so `not_required` can carry no evidence.
  The coherence check requires every evidence column except
  `root_relative_prefix` for `bound`; that prefix follows the kind-specific rule
  below.

  | Column | Type | Meaning |
  | --- | --- | --- |
  | `media_job_id` | `bigint` | restrictive job FK |
  | `media_root_kind_id` | `smallint` | exact ordinal 1-5 |
  | `binding_state` | `text` | `bound` or `not_required` |
  | `media_root_catalog_generation_public_id` | `uuid` | bound generation public id |
  | `attestation_generation` | `bigint` | bound numeric generation fence |
  | `source_sha256` | `bytea` | bound source digest |
  | `generation_sha256` | `bytea` | bound generation identity |
  | `media_root_catalog_slot_public_id` | `uuid` | bound stable slot id |
  | `logical_key` | `text` | bound logical key snapshot |
  | `canonical_path` | `text` | bound authenticated path evidence |
  | `filesystem_device` | `bytea` | bound eight-byte identity |
  | `filesystem_inode` | `bytea` | bound eight-byte identity |
  | `mount_id` | `bigint` | bound mount identity |
  | `filesystem_type` | `text` | bound filesystem type |
  | `capability_mask` | `smallint` | bound seven-bit capability summary |
  | `durability_class` | `text` | bound class |
  | `durability_evidence` | `text` | bound evidence |
  | `sole_writer_class` | `text` | bound class |
  | `sole_writer_evidence` | `text` | bound evidence |
  | `root_relative_prefix` | `text` | source/output association prefix only |
  | `root_identity_sha256` | `bytea` | bound slot identity |

- Use `media_job_root_snapshot_pkey` on
  `(media_job_id, media_root_kind_id)`, restrictive foreign keys
  `media_job_root_snapshot_job_fkey` and
  `media_job_root_snapshot_kind_fkey`, nullable restrictive foreign key
  `media_job_root_snapshot_generation_fkey` from `attestation_generation` to
  `media_root_catalog_generation_id`, and
  `ix_media_job_root_snapshot_generation` on
  `(attestation_generation, media_job_id)`.
- `media_job_root_snapshot_binding_state_known` closes the state set.
  `media_job_root_snapshot_required_kinds_bound` requires source, output, and
  workspace to be `bound`. `media_job_root_snapshot_binding_coherent` requires
  every bound evidence column except `root_relative_prefix` and forbids every
  evidence column for `not_required`.
  `media_job_root_snapshot_relative_prefix` requires a valid prefix for bound
  source and output, requires null for workspace, backup, and quarantine, and
  calls `media_root_relative_prefix_valid_v1` for each nonnull value. An empty
  source/output prefix is bound whole-root evidence, distinct from null for the
  other kinds and for `not_required`. Add exact checks
  `media_job_root_snapshot_attestation_generation_positive`,
  `media_job_root_snapshot_logical_key_contract`,
  `media_job_root_snapshot_source_sha256_length`,
  `media_job_root_snapshot_generation_sha256_length`,
  `media_job_root_snapshot_filesystem_device_length`,
  `media_job_root_snapshot_filesystem_inode_length`,
  `media_job_root_snapshot_mount_id_nonnegative`,
  `media_job_root_snapshot_filesystem_type_bounds`,
  `media_job_root_snapshot_capability_mask_bounds`, and
  `media_job_root_snapshot_identity_sha256_length`.
- Every job receives exactly five rows in root-kind id order. Backup is bound
  exactly when the immutable job policy snapshot enables backup. Quarantine is
  bound exactly when that snapshot enables quarantine. Their `not_required`
  rows remain part of root-snapshot identity and carry no catalog or path value.
- Add ungranted helper
  `media_job_root_snapshot_complete_v1(media_job_id_input bigint) RETURNS void`.
  The only job-admission procedure calls it after inserting the five rows and
  before making the job visible. It requires ids 1 through 5 exactly once,
  backup/quarantine states to match the immutable policy snapshot, every bound
  row to use one generation/source/generation-digest tuple, source and output to
  be identical except kind, and every copied value to equal its immutable
  attestation. Failure raises a bounded root or snapshot code and aborts the
  complete job transaction. Runtime has no direct table or helper access, so no
  second insertion path can bypass this finalizer.
- Source and output rows must copy the same generation, slot, path, identity,
  evidence, and prefix. Their root-kind ids are the only differing semantic
  field. Workspace, backup, and quarantine copy their own profile-version
  bindings.
- Extend `media_job_configuration_snapshot` with
  `root_snapshot_contract_version smallint`,
  `root_snapshot_row_count smallint`,
  `root_snapshot_byte_count bigint`, and `root_snapshot_sha256 bytea`.
  Use checks `media_job_configuration_snapshot_root_contract_v1`,
  `media_job_configuration_snapshot_root_row_count`,
  `media_job_configuration_snapshot_root_byte_count_positive`, and
  `media_job_configuration_snapshot_root_sha256_length`. Version is exactly 1,
  row count is exactly 5, byte count is positive, and digest length is 32.
- Root-snapshot framing uses ASCII `revaer-media-job-root-snapshot`, zero byte,
  unsigned 32-bit version `1`, then exactly five rows. Each row begins with the
  root-kind id and binding state. `not_required` ends there. `bound` continues
  with every bound column above in table order using ADR 517 primitives. The
  database writer and Rust reader independently verify byte count and digest.
- The accepted ADR 517 procedure name
  `media_job_policy_snapshot_root_list_v1(job_public_id_input uuid,
  attempt_number_input int, claim_generation_input bigint)` returns exactly
  these five rows without pagination after repeating the claim fence.

## Stale Generation And Job Behavior

- Binding readiness is current only when source state is `ready`, every required
  binding points to the active generation, every kind and capability/writer rule
  still holds, and the association's profile-version id equals the profile's
  active head. Destructive readiness additionally requires persistence and the
  same-attestation source/output rule.
- A catalog generation change immediately makes prior profile and association
  versions not ready for new jobs. No trigger rewrites them, no background task
  creates a replacement version, and no logical-key lookup silently follows the
  new generation.
- Direct API editing creates the next complete version. YAML apply may create
  disabled drafts for unmapped or kind-forbidden keys. Activation always
  resolves the keys again against the then-current generation.
- Job admission resolves profile, association, policy, and all five root rows in
  one transaction. If the catalog pointer changes before commit, serialization
  or explicit generation checks reject the job with
  `media_root_attestation_stale`.
- Preview and an immutable dry-run-only job require binding readiness and perform
  no filesystem mutation. Any job whose policy can mutate, replace, quarantine,
  back up, or publish requires destructive readiness for every bound row before
  enqueue. A later runtime check repeats the same mode-specific requirement.
- A queued or recovering job never follows a new generation. Claim, workspace
  admission, and pre-mutation checks require its exact numeric generation and
  generation digest to remain active and its descriptor observations to match.
  Otherwise the job fails closed with `media_root_attestation_stale` or
  `media_root_identity_mismatch` and enters the accepted re-plan/recovery
  workflow without rewriting its snapshot.
- Generation loss during active filesystem work trips the next generation fence
  before another side effect. Existing descriptor and process cleanup contracts
  still settle already-started work; catalog change is not permission to abandon
  cleanup evidence.

## Stored-Procedure ABI And Grants

### Reconciliation procedures

- Define these exact security-definer procedures with fixed search path
  `pg_catalog, public`:

  ```sql
  media_root_catalog_reconcile_begin_v1(
      source_format_version_input smallint,
      source_sha256_input bytea,
      attestation_sha256_input bytea,
      generation_sha256_input bytea,
      slot_count_input smallint
  ) RETURNS TABLE (
      media_root_catalog_generation_public_id uuid,
      attestation_generation bigint,
      already_current boolean
  )
  ```

  ```sql
  media_root_catalog_reconcile_slot_v1(
      media_root_catalog_generation_public_id_input uuid,
      logical_key_input text,
      requested_path_input text,
      canonical_path_input text,
      filesystem_device_input bytea,
      filesystem_inode_input bytea,
      mount_id_input bigint,
      filesystem_type_input text,
      read_capable_input boolean,
      write_capable_input boolean,
      create_new_capable_input boolean,
      fsync_capable_input boolean,
      rename_capable_input boolean,
      delete_capable_input boolean,
      capacity_probe_capable_input boolean,
      durability_class_input text,
      durability_evidence_input text,
      sole_writer_class_input text,
      sole_writer_evidence_input text,
      owner_uid_input bigint,
      owner_gid_input bigint,
      mode_bits_input integer,
      root_identity_sha256_input bytea
  ) RETURNS uuid
  ```

  The returned UUID is the stable `media_root_catalog_slot_public_id`, not an
  attestation or generation id.

  ```sql
  media_root_catalog_reconcile_slot_kind_v1(
      media_root_catalog_generation_public_id_input uuid,
      media_root_catalog_slot_public_id_input uuid,
      root_kind_input text
  ) RETURNS void
  ```

  ```sql
  media_root_catalog_reconcile_activate_v1(
      media_root_catalog_generation_public_id_input uuid
  ) RETURNS TABLE (
      media_root_catalog_generation_public_id uuid,
      attestation_generation bigint,
      source_sha256 bytea,
      generation_sha256 bytea,
      slot_count smallint,
      activated_at timestamptz
  )
  ```

  ```sql
  media_root_catalog_mark_unavailable_v1(
      source_state_input text,
      source_reason_code_input text
  ) RETURNS void
  ```

  ```sql
  media_root_catalog_mark_attestation_invalid_v1(
      attestation_reason_code_input text
  ) RETURNS void
  ```

- The procedures accept no array, JSON, path list, extension bag, caller role,
  or free-form diagnostic. Begin and activate recompute generation identity;
  slot recomputes slot identity; finalization recomputes the aggregate.
- Source-unavailable mutation records a non-ready source plus unevaluated
  attestation. Attestation-invalid mutation records a ready source, invalid
  attestation, the closed supplied reason, and no active generation. Successful
  activation records both states ready and clears both reasons. Each mutation
  derives `reconciled_at` from its own transaction.

### Administration and readiness readers

- Define these exact read procedures:

  ```sql
  media_root_catalog_state_get_v1() RETURNS TABLE (
      source_state text,
      source_reason_code text,
      attestation_state text,
      attestation_reason_code text,
      media_root_catalog_generation_public_id uuid,
      attestation_generation bigint,
      source_format_version smallint,
      source_sha256 bytea,
      generation_sha256 bytea,
      slot_count smallint,
      activated_at timestamptz,
      reconciled_at timestamptz
  )
  ```

  ```sql
  media_root_catalog_slot_page_v1(
      limit_input smallint,
      cursor_logical_key_input text,
      cursor_slot_public_id_input uuid
  ) RETURNS TABLE (
      media_root_catalog_slot_public_id uuid,
      logical_key text,
      allowed_root_kind text,
      requested_path text,
      canonical_path text,
      filesystem_device bytea,
      filesystem_inode bytea,
      mount_id bigint,
      filesystem_type text,
      capability_mask smallint,
      durability_class text,
      durability_evidence text,
      sole_writer_class text,
      sole_writer_evidence text,
      owner_uid bigint,
      owner_gid bigint,
      mode_bits integer,
      validated_at timestamptz,
      root_identity_sha256 bytea,
      binding_ready boolean,
      binding_reason_code text,
      destructive_ready boolean,
      destructive_reason_code text,
      page_has_more boolean
  )
  ```

  ```sql
  media_root_catalog_readiness_get_v1() RETURNS TABLE (
      source_state text,
      source_reason_code text,
      attestation_state text,
      attestation_reason_code text,
      attestation_generation bigint,
      root_kind text,
      attested_slot_count integer,
      binding_ready_slot_count integer,
      destructive_ready_slot_count integer
  )
  ```

  ```sql
  media_profile_root_readiness_get_v1(
      media_profile_public_id_input uuid
  ) RETURNS TABLE (
      media_profile_public_id uuid,
      latest_profile_version integer,
      active_profile_version integer,
      binding_ready boolean,
      binding_reason_code text,
      destructive_ready boolean,
      destructive_reason_code text,
      stale_root_kind text,
      association_count integer
  )
  ```

  ```sql
  media_discovery_association_root_readiness_get_v1(
      media_discovery_association_public_id_input uuid
  ) RETURNS TABLE (
      media_discovery_association_public_id uuid,
      latest_association_version integer,
      active_association_version integer,
      media_profile_public_id uuid,
      profile_version integer,
      binding_ready boolean,
      binding_reason_code text,
      destructive_ready boolean,
      destructive_reason_code text
  )
  ```

- Slot paging orders by `(logical_key, media_root_catalog_slot_public_id)` and
  limits slots, not joined kind rows. `limit_input` is one through 200. Cursor
  fields are either both null or both nonnull and must identify an active slot
  exactly. The procedure internally probes one extra slot, emits at most
  `limit * 5` rows for the requested page, and repeats one `page_has_more` value
  on every emitted row. Each row represents one normalized allowed-kind pair;
  both readiness values and reasons are for that kind, not the slot as a whole.
  `binding_ready` means the attestation has the capabilities and writer evidence
  needed to bind that kind. `destructive_ready` additionally requires
  `restart_persistent`; it is a subset of binding readiness. The application
  groups those bounded rows into typed API arrays.
- Root readiness returns exactly five rows in root-kind ordinal order, including
  zero counts while the source is unavailable or a valid catalog is empty.
  `attested_slot_count` counts allowed-kind rows in the active generation;
  binding and destructive counts apply the definitions above. The
  generation and profile/association active-version columns are nullable when no
  active value exists; latest-version columns are nonnull for an existing
  resource.
- Readiness readers never return a path, device, inode, mount id, digest, or
  owner value. Missing resources raise their existing not-found code rather than
  returning an empty successful result.
- Profile and association binding readiness requires complete current-generation
  logical resolution and capability/writer proof. Destructive readiness also
  requires every bound row to be restart-persistent and the exact same-slot and
  profile/association-head rules. If several roots fail, `stale_root_kind` is the
  lowest root-kind ordinal. Within that root, reason precedence is unmapped,
  forbidden kind, incomplete binding, stale generation, invalid attestation,
  overlap, writer control, then durability. Source-state failure and then global
  attestation-state failure precede every row-level reason. A ready result has
  null reasons and stale kind.

### Privileges

- Every function above is owned by ADR 551's no-login schema owner, uses
  `SECURITY DEFINER`, and fixes its search path. Reconciliation and state
  mutation procedures are `VOLATILE`; administration and readiness readers are
  `STABLE`.
- Revoke all execution from `PUBLIC`. Grant ADR 551's recorded runtime role
  execute on exactly the six reconciliation/state writers and five readers
  named in this section, plus the separately accepted profile/association
  aggregate, job-admission, and ADR 517 root-reader procedures. The schema owner
  retains owner rights. An unrelated login receives none. Grant no table,
  sequence, trigger-function, validator, digest helper, overlap helper, or
  snapshot-finalizer privilege.
- The exact root grant inventory is
  `media_root_catalog_reconcile_begin_v1`,
  `media_root_catalog_reconcile_slot_v1`,
  `media_root_catalog_reconcile_slot_kind_v1`,
  `media_root_catalog_reconcile_activate_v1`,
  `media_root_catalog_mark_unavailable_v1`,
  `media_root_catalog_mark_attestation_invalid_v1`,
  `media_root_catalog_state_get_v1`,
  `media_root_catalog_slot_page_v1`,
  `media_root_catalog_readiness_get_v1`,
  `media_profile_root_readiness_get_v1`, and
  `media_discovery_association_root_readiness_get_v1`, each with the exact
  argument types above. PostgreSQL overloads with another argument list receive
  no grant by implication.
- Only injected bootstrap wiring calls reconciliation procedures. No HTTP, UI,
  YAML, discovery, job, or worker route exposes them. Possession of a database
  row or procedure result still cannot create a usable root because runtime
  filesystem operations require the in-memory source, held descriptor, root
  lock, exact generation, and exact identity digest.
- The authenticated administration API calls only the three root-catalog read
  procedures.
  General profile and association readiness calls only the path-free readers.

## HTTP Contract

### Root administration

- Add authenticated `GET /v1/media/root-catalog`. It accepts `limit` and
  `cursor`; default limit is 50 and maximum is 200. The cursor is URL-safe
  base64 without padding over unsigned 16-bit logical-key byte length, exact key
  bytes, and the 16 UUID bytes. Invalid, noncanonical, stale-shape, or oversized
  cursors return `400 media_configuration_invalid`.
- The response contains exactly:
  - `format_version: 1`;
  - `source_state`, optional `source_reason`, `attestation_state`, and optional
    `attestation_reason`;
  - optional `generation` containing generation public id, positive attestation
    generation as a decimal string, lowercase 64-character source and generation
    SHA-256 values, integer slot count, RFC 3339 UTC activation time, and RFC 3339
    UTC reconciliation time;
  - `slots`, each containing slot public id, logical key, requested path,
    canonical path, device and inode as exactly 16 lowercase hexadecimal
    characters, mount id as a decimal string, filesystem type, seven named
    capability booleans, durability class/evidence, sole-writer class/evidence,
    owner uid/gid as unsigned JSON integers, four-digit octal mode, RFC 3339 UTC
    validation time, lowercase identity digest, and ordered `allowed_kinds`;
  - each `allowed_kinds` item contains exactly `kind`, `binding_ready`, optional
    bounded `binding_reason`, `destructive_ready`, and optional bounded
    `destructive_reason`, ordered by root-kind ordinal; and
  - optional `next_cursor`.
- This is the only HTTP response authorized to contain complete root paths or
  raw filesystem identity. It uses the existing API-key middleware,
  `Cache-Control: no-store`, no SSE payload, no browser persistence, and no
  access-log query or body fields. A future separate root-admin scope requires a
  separately approved auth decision; this proposal uses the repository's
  current authenticated API boundary.
- Add authenticated `GET /v1/media/root-catalog/readiness`. It returns exactly
  `format_version: 1`, source and attestation states with their optional bounded
  reasons, optional current generation as a decimal string, and an ordered
  five-item `kinds` array. Each
  item contains exactly `kind`, `attested_slot_count`,
  `binding_ready_slot_count`, and `destructive_ready_slot_count` as nonnegative
  integers. It omits paths, keys, public ids, identities, and digests.
  Root administration has no `POST`, `PUT`, `PATCH`, or `DELETE` method. Those
  methods return `405`, RFC 9457 problem code `media_method_not_allowed`, and
  `Allow: GET, HEAD`.

### Profiles and discovery associations

- Every route in this section remains behind the existing API-key middleware.
  Preserve `GET /v1/media/profiles` and `POST /v1/media/profiles`, but replace
  path fields with exact logical fields:
  `output_root_key`, `workspace_root_key`, optional `backup_root_key`, and
  optional `quarantine_root_key`. `POST` requires `If-None-Match: *` and returns
  `201`, the complete version 1 representation, and strong ETag
  `"media-profile:<public-id>:v1"`.
- Preserve authenticated `GET /v1/media/profiles/{public-id}`. Replace `PATCH`
  with complete `PUT`, require the current strong `If-Match`, and return the
  complete new immutable version and ETag. `PATCH` returns `405`. Profile
  responses expose logical keys, resolution state, kind, version, lifecycle,
  and readiness reason, but no root path or filesystem identity.
- Add `POST /v1/media/profiles/{public-id}/archive` with the latest strong
  `If-Match` and an empty body. It appends an archived version, clears the active
  head, and returns `200`, the complete archived representation, and its ETag.
- Profile `GET`, `POST`, and `PUT` representations identify both
  `latest_version` and optional `active_version`. They contain the accepted
  non-root aggregate plus an exact ordinal root-binding array. Each binding
  contains `kind`, `logical_key`, `resolution_state`, `binding_ready`, optional
  bounded `binding_reason`, `destructive_ready`, and optional bounded
  `destructive_reason`; it contains no slot id, path, filesystem identity, or
  digest.
  ETags always fence the latest version. Preview, association creation, and job
  admission use only the explicitly named active profile version.
- Add authenticated collection and item routes:
  - `GET /v1/media/discovery-associations`;
  - `POST /v1/media/discovery-associations` with `If-None-Match: *`;
  - `GET /v1/media/discovery-associations/{public-id}`;
  - `PUT /v1/media/discovery-associations/{public-id}` with strong `If-Match`;
  - `POST /v1/media/discovery-associations/{public-id}/archive` with strong
    `If-Match` and an empty body; and
  - `GET /v1/media/discovery-associations/{public-id}/readiness`.
- Association create/replace requests contain exactly `association_key`,
  `media_profile_public_id`, positive `profile_version`, `source_root_key`,
  `root_relative_path`, `manual_enabled`, `watcher_enabled`, and
  `schedule_enabled`. Unknown fields fail. Until the held ADR 516 values are
  approved, either automatic flag set true returns
  `409 media_configuration_pending_contract`.
- `root_relative_path` is required even for whole-root selection, represented
  only by `""`. Candidate paths in preview/run are relative to the attested root,
  must be nonempty, and must fall inside the association prefix; the empty
  prefix admits any otherwise-valid candidate under that root. The application
  opens them through the held root descriptor and never joins an empty prefix
  against a working directory or a reconstructed host path.
- Association responses contain exactly the submitted logical fields, public id,
  `latest_version`, optional `active_version`, lifecycle, resolution state,
  binding readiness/reason, destructive readiness/reason, created time, and
  updated representation ETag. They contain no root path, slot id, filesystem
  identity, or digest. ETags fence the latest version while operational discovery
  uses only the active head.
- Profile and association collections use ADR 521 keyset pagination with default
  50 and maximum 200. Requests are capped at 1 MiB after decompression. A
  profile carries two required and at most two optional root-binding rows. An
  association carries exactly one source binding. No API request accepts an
  absolute root path.
- Change discovery preview and run requests to carry
  `media_discovery_association_public_id` plus one through 128 normalized
  root-relative candidate paths. Each path is at most 4,096 bytes and the
  complete request remains under 1 MiB. Profile id plus absolute source paths is
  retired. The worker-owned `POST /v1/media/jobs` route remains retired.
- Keep `GET /v1/media/profiles/{public-id}/readiness`, but return latest and
  optional active profile versions, binding and destructive readiness/reasons,
  root readiness by kind, active association count, and bounded reasons without
  paths or identities. Association readiness returns the same two readiness
  classes for its bound source/profile pair.

### HTTP errors

- Every failure uses the repository's RFC 9457 `ProblemDetails`. `type`, `title`,
  status, and bounded code are stable; `detail`, invalid field pointers, and
  bounded context never contain a path or rejected value.
- Use these exact status mappings:
  - `400` for malformed fields, cursor, key, relative path, kind, bound, or
    unknown field with `media_configuration_invalid` or
    `media_configuration_bound_exceeded`;
  - `401` for missing or invalid API authentication;
  - `404` for an unknown profile or association;
  - `405` with the exact `Allow` header for every unsupported collection, item,
    archive, readiness, or root-catalog method;
  - `409` for key conflicts, physical or association overlap, unmapped keys,
    forbidden kinds, incomplete bindings, pending automatic-discovery contract,
    or a catalog change during admission;
  - `412` for a stale strong ETag with
    `media_configuration_version_conflict`;
  - `428` for missing `If-None-Match` or `If-Match`; and
  - `413` with `media_configuration_bound_exceeded` when a decompressed JSON or
    YAML request exceeds its byte bound;
  - `503` for an operational preview, run, or job admission while source,
    generation, evidence, or profile readiness is unavailable; and
  - `500` with the generic internal problem and no SQL or root detail for an
    unexpected storage or serialization defect after bounded retries.
- Exact `Allow` values are `GET, HEAD` for root-catalog and readiness routes,
  `GET, HEAD, POST` for profile and association collections, `GET, HEAD, PUT` for
  profile and association items, and `POST` for archive commands. `HEAD` follows
  the framework's authenticated `GET` semantics and returns no body; `OPTIONS`
  follows the repository's existing authenticated routing policy.
- Preserve ADRs 521, 523, and 550 codes, including
  `media_configuration_root_unmapped`, `media_root_slot_unknown`,
  `media_root_kind_forbidden`, `media_root_attestation_stale`,
  `media_root_binding_incomplete`, `media_root_overlap`,
  `media_root_durability_unproven`, and
  `media_root_writer_control_unproven`. Problems outside root administration
  never include a path, logical key value, identity, digest, SQL detail, or
  source-document location.

## Portable YAML Contract

- Preserve accepted top-level `format_version: 1` and
  `kind: revaer.media.profile_bundle`. Profiles replace path fields with:

  ```yaml
  output_root_key: media-library
  workspace_root_key: media-workspace
  backup_root_key: media-backup       # optional
  quarantine_root_key: media-hold     # optional
  ```

- Add the accepted top-level `discovery_associations` array. Each row contains
  exactly `association_key`, `profile_key`, positive `profile_version`,
  `source_root_key`, `root_relative_path`, `manual_enabled`,
  `watcher_enabled`, and `schedule_enabled`.
- Whole-root export emits `root_relative_path: ""` explicitly. Import rejects
  missing or null values instead of interpreting them as whole-root selection.
- Portable YAML never contains `source_root`, `output_root`, another absolute
  path, slot public id, generation id, filesystem identity, evidence class,
  capability result, source digest, or root-catalog document location. Local
  path export remains a separate explicit operator action and is not accepted
  as import authority.
- Validate all logical references in memory through the same typed compiler as
  JSON. Apply uses one `SERIALIZABLE` transaction and the same ordering as API
  activation: lock the catalog singleton, resolve logical keys against its
  captured active generation, acquire all required source-attestation advisory
  locks in ascending attestation-id order, then lock resource parents in
  accepted kind/key order. It writes normalized version and binding rows and
  commits only if that generation is still active. A resource-parent lock must
  never precede a required source-attestation lock.
- An unknown or kind-forbidden key creates a disabled draft version with its
  exact logical key and null attestation reference. It never stores a path
  placeholder. Existing active versions remain current until an explicitly
  matched complete replacement succeeds. Any non-mapping error rolls back the
  complete bundle.
- Preserve ADR 521's 4 MiB bundle, 128 top-level resource, 4,096 child-row,
  expected-version, create-intent, duplicate, alias, custom-tag, and canonical
  export bounds. Root binding and association rows count as child rows.

## UI Workflow

- Root administration first loads the authenticated paged catalog and renders
  source state, generation, logical keys, kinds, evidence, and remediation.
  Full paths appear only in that administration view and are never copied into
  profile form state, browser storage, route parameters, toast text, or SSE.
- The profile editor uses catalog-backed selects for output, workspace, backup,
  and quarantine logical keys. It loads one complete version, sends `POST` or
  `PUT` with the required conditional header, and handles 412 by preserving the
  operator's unsaved typed values while reloading current state.
- Discovery association is a separate workflow. It selects an exact profile
  version and source logical key, explicitly selects the whole root or edits one
  nonempty relative prefix, and shows manual,
  watcher, and schedule state. Held automatic modes are disabled with their
  bounded remediation reason; they are not silently converted to manual mode.
- Preview and run actions select an association and accept only relative
  candidates. The UI never reconstructs an absolute path or submits one.
- Binding and destructive readiness are separately visible per root kind;
  profile-version and association readiness reflect destructive job admission.
  Stale generations provide an explicit create-new-version action. The UI never
  performs automatic rebinding or retries a 412/409 mutation without operator
  review.

## Legacy Cutover

- Make no migration. The frozen 167 migration files and generated candidate
  remain byte-identical transition evidence until ADRs 522, 541, and 551 perform
  the final pre-v1 `init.sql` cutover.
- In that final init only, replace or retire these incompatible structures:
  - profile-local `media_profile.source_root`, `output_root`, automation booleans,
    interval, and mutable `configuration_version` representation;
  - `media_profile_root` and its add, list, revalidate, identity, and overlap
    procedures;
  - path-bearing `media_profile_import_draft` rows and procedures;
  - every root-key constraint that currently relies on the wider
    `media_key_valid_v1` helper, replacing it with
    `media_root_logical_key_valid_v1` without changing unrelated key contracts;
  - profile-root-owned discovery schedule and watcher references;
  - absolute-path/profile-keyed discovery fingerprints;
  - the current arbitrary-row `media_job_root_snapshot`; and
  - profile, API, YAML, and UI models that accept or expose raw roots outside the
    authenticated administration reader.
- Preserve legacy behavior only through parity evidence, not compatibility
  tables, views, dual writes, fallback procedures, or a second bootstrap path.
  This v0 product has no released database to migrate or adopt.
- The final init must contain only the new normalized authority. Runtime,
  database tests, Playwright setup, local startup, CI, Sonar, Docker, and Helm
  all switch in the same cutover sequence accepted by ADR 541.

## Limits And Pagination

| Surface | Exact bound and ordering |
| --- | --- |
| Packaged source | 8,388,608 raw bytes; zero through 256 slots |
| Logical and association keys | one through 64 bytes under `media_root_logical_key_valid_v1` |
| Requested/canonical roots | absolute, non-root, NUL-free, one through 4,096 UTF-8 bytes |
| Association prefixes | explicit empty string for whole root, otherwise one through 4,096 bytes under `media_root_relative_prefix_valid_v1`; omitted/null forbidden |
| Candidate file paths | one through 4,096 bytes under `media_root_relative_path_valid_v1`; empty forbidden |
| Allowed kinds | one through five normalized rows in root-kind ordinal order |
| Profile roots | exactly output and workspace; zero or one backup; zero or one quarantine |
| Active associations | zero through 128 per active profile version |
| Discovery request | one through 128 candidates and at most 1 MiB after decompression |
| JSON aggregate mutation | at most 1 MiB after decompression |
| Portable YAML bundle | at most 4 MiB, 128 top-level resources, and 4,096 child rows |
| Catalog page | default 50 and maximum 200 slots; key/public-id keyset order |
| Profile/association page | default 50 and maximum 200 resources; key/internal-id keyset order |
| Job roots | exactly five unpaged rows in root-kind ordinal order |

- Collection limits are positive integers; zero, negative, non-integer, and
  over-limit values fail with `media_configuration_invalid` or
  `media_configuration_bound_exceeded` before a database call. Offset pagination
  is not accepted.
- Catalog cursors use the exact binary representation above. Profile and
  association cursors retain ADR 521's opaque, canonical URL-safe representation
  of normalized key plus stable id. Cursors are resource-specific and cannot be
  replayed across collections.

## Observability

- Metrics may label only closed source state, attestation state, readiness state,
  root kind, evidence class, lifecycle state, and bounded reason. Paths, logical keys,
  public ids, generation numbers, devices, inodes, mount ids, digests, profiles,
  associations, and jobs are forbidden labels.
- Startup and reconciliation logs contain result, slot count, source state, and
  bounded reason only. Full paths, JSON, identities, owner values, source
  location, and digests remain in the authenticated root administration response
  or bounded audit evidence, not general logs.
- Traces may record procedure name, outcome, duration, row count, and bounded
  reason. They do not record procedure arguments or result rows.
- Readiness exposes source, attestation, profile, and association state
  separately so a catalog-source failure is not misreported as a filesystem,
  capability, or policy failure.

## Consequences

- Deployment configuration remains the only path authority while the database
  gains exact, normalized, immutable evidence and reference integrity.
- Logical keys survive portable export, but a catalog change deliberately
  requires explicit new profile and association versions before new work.
- Whole-root discovery becomes expressible without a fabricated child directory.
  It monopolizes the non-overlapping discovery scope for that source attestation
  and does not authorize watchers, schedules, or destructive aggregate use.
- Exact five-row job snapshots become bounded and reproducible, including
  optional root absence, without reading live profile state.
- Source/output in-place replacement is straightforward because both rows point
  to one slot attestation. Distinct-output publishing remains unavailable.
- Multi-call reconciliation is more verbose than one JSON parameter, but it
  preserves normalization, bounded procedure arguments, atomicity, and direct
  row constraints.
- Historical generations and versions accumulate in v0. They are small,
  immutable, and required for evidence; no unapproved pruning heuristic is
  introduced.
- Root administration intentionally reveals paths to the existing authenticated
  API principal. A finer-grained administrator scope is not invented here.

## Rollback

- This proposal changes documentation only. Before approval, rollback is removal
  or supersession of this Proposed ADR and its generated documentation entries;
  no database or runtime state exists to reverse.
- After implementation, every failed begin/append/finalize sequence rolls back
  its complete `SERIALIZABLE` transaction and leaves the previously committed
  singleton state unchanged. A committed unavailable transition may be reversed
  only by a later complete, proved reconciliation; no operator SQL edits the
  pointer.
- Before the first stable release, an implementation rollback may use only an
  artifact whose embedded final-init digest and root contract match the database
  baseline. If they do not match, restore a compatible full database backup or
  recreate the disposable v0 database from the selected artifact. Never run a
  down migration, reinterpret an old profile-local path, reactivate a historical
  generation row, or add a dual-read/dual-write compatibility path.
- Historical catalog generations, profile/association versions, and retained job
  snapshots survive a compatible rollback. A runtime that cannot parse their
  contract version fails media readiness closed while preserving control-plane
  access and evidence.

## Authorization Boundary

### Already authorized

- ADR 523 already authorizes deployment-authoritative slots, maximum 256 slots,
  five root kinds, descriptor-relative proof, non-overlap, same-slot
  source/output, logical bindings, immutable attestation generations, exact
  five-kind jobs, rejection of stale new-job admission, identity revalidation,
  path-restricted administration, and its stable errors.
- ADR 550 already authorizes the version 1 startup JSON document, 8 MiB source
  bound, fixed packaged location, native Linux location override, semantic
  source digest, startup-only loading, closed evidence classes, fail-closed
  source states, and Linux `amd64`/`arm64` support boundary.
- ADRs 517, 521, 525, 541, and 551 already authorize generation-fenced snapshot
  reads, normalized operator resources and bounds, backup-root ownership,
  packaged init lifecycle, and the owner/runtime privilege model.

### Approved Scope

- Approval authorizes only the normalized relational contract and fields,
  normalized allowed-kind rows, attestation and generation framing, immutable
  lifecycle, atomic reconciliation ABI, logical version bindings, exact job
  root row shape, staleness behavior, procedure grants, HTTP/YAML/UI contracts,
  limits, whole-root prefix semantics, and later init-only legacy retirement
  specified here. Internal refinements follow only the accepted G1 boundary;
  no broader operator delegation is assumed.
- The newly requested choices include `public` schema placement; the exact
  root-key and relative-path validators; latest/active version heads and draft
  semantics; separate source and attestation state; repeated semantic
  digest versus monotonic occurrence identity; lock/commit ordering for competing
  reconciliations; fail-closed invalidation of queued work on any active-generation
  change; binding-versus-destructive readiness; the 128 active-association bound;
  the procedure ABI/grants; and use of the current API-key boundary for the only
  path-bearing HTTP response.
- Approval does not authorize an implementation outside these semantics and bounds,
  raw path mutation, automatic rebinding, catalog reload, database-authored
  slots, JSON/array persistence, distinct output, cross-slot replacement,
  automatic discovery activation, unsupported platforms, table grants, a new
  dependency, a migration, released-database adoption, or criteria relaxation.
- This documentation commit contains no SQL, Rust, API, OpenAPI, generated
  client, YAML parser, UI, Docker, Helm, workflow, or runtime implementation.
  Implementation remains blocked until the operator explicitly approves this
  exact ADR.

## Outside-In Validation Matrix

| Boundary | Required scenarios | Required result |
| --- | --- | --- |
| Authenticated root administration | unauthenticated, invalid key, valid key, page 1/2, invalid cursor, full and empty catalog | only valid auth sees full paths; pagination is stable and bounded |
| API path prohibition | every profile, association, YAML, preview, run, job, error, SSE, and readiness payload | no absolute root or filesystem identity outside root administration |
| UI root workflow | source missing/invalid, attestation invalid, empty, ready, stale, unmapped, forbidden-kind, one-over-page | exact state and remediation; no path copied into profile or association requests |
| Profile workflow | create, replace, missing/stale ETag, required/optional keys, unmapped draft, archive | immutable versions, exact root rows, no partial mutation |
| Association workflow | create, replace, profile-version mismatch, explicit whole root, omitted/null prefix, relative-path bounds, 128/129 associations | exact source binding and profile version; no implicit scope expansion; overflow fails closed |
| YAML | canonical export/import, unmapped keys, key remap, duplicate/unknown fields, aliases, 4 MiB and row bounds | one atomic transaction; drafts store keys only; no path placeholder |
| Version heads | new unresolved draft, resolved replacement, archive, concurrent PUT, cross-parent head id | latest and active heads diverge/converge exactly; no head references another resource |
| Key/path helpers | edge lengths, underscore, uppercase, Unicode, edge hyphen, slash, empty prefix versus empty candidate, empty component, dot/dot-dot, backslash, NUL | only association prefixes accept explicit empty; candidates and traversal retain the nonempty grammar |
| Source parser | every ADR 550 format, byte, field, key, kind, enum, and digest case | parser identity remains exact and no database call occurs on rejected content |
| Filesystem proof | symlink, hard link, path replacement, mount replacement, bind alias, unsafe ancestry, permissions, all capabilities | complete source rejected or one exact descriptor-owned attestation produced |
| State separation | every source failure, every attestation failure, valid empty source, successful proof, failed state write | source and attestation reasons remain distinct; no old generation authorizes work |
| Evidence matrix | every valid/invalid durability and writer pair on both Linux architectures | only accepted pairs persist; destructive readiness needs persistent/exclusive proof |
| Reconciliation | zero/one/256 slots, duplicate key, duplicate identity, nested path, kind omission, digest mismatch, failure after every append | prior generation remains current or one complete new generation activates |
| Reconciliation concurrency | identical concurrent starts, different catalogs, source failure racing success, old digest returning later | lock/commit order wins; each success creates or reuses only as specified; historical rows are never reactivated |
| Digest vectors | Unicode paths, all kind masks, all capability masks, max ids/mode, one-bit mutation, reordered slots/kinds | Rust/PostgreSQL slot, aggregate, and generation digests match exactly |
| Database constraints | identifier length/collision audit and direct invalid row attempts for every named constraint and immutable trigger | no identifier truncates; schema-owner tests fail at the exact constraint; runtime has no direct access |
| Privileges | PUBLIC, unrelated login, runtime login, owner login, `SET ROLE`, direct table and helper calls | only exact procedure grants work; no table/sequence/trigger access |
| Same-slot behavior | source/output same attestation, two duplicate attestations, distinct output, missing dual kind | only one dual-kind attestation is accepted |
| Physical overlap | equal, ancestor, descendant, device/inode alias, bind mount, equivalent mount | generation finalization fails with bounded overlap reason |
| Association overlap | whole-root versus any other prefix, equal, ancestor, descendant, siblings, stale historical versions, concurrent create across profiles | only non-overlapping current prefixes commit |
| API/YAML lock ordering | competing API activation and multi-source YAML apply, reversed input order, whole-root overlap | one catalog/source/parent lock order, deterministic bounded transaction failure or successful non-overlapping commit, no lock inversion |
| Job admission | backup/quarantine on/off truth table, stale catalog race, profile/association mismatch | exactly five rows and coherent optional states or no job |
| Snapshot reader | wrong job/attempt/generation, missing/extra/reordered row, byte/digest mismatch | ADR 517 reader returns five ordered rows or one stable failure |
| Runtime revalidation | catalog changes at enqueue, claim, admission, pre-mutation, and active work | no rebinding; side effects stop at the next fence and cleanup evidence settles |
| Package source | Docker read-only file, Helm ConfigMap modes, reserved environment, emptyDir/RWO/RWOP, restart | package evidence matches ADR 550 and unsupported states never become ready |
| Rollback | failed reconciliation, rejected implementation rollback, prior artifact with same/different final init digest | transaction restores prior pointer; incompatible baseline rollback fails closed |
| Observability | logs, traces, metrics, health, errors, SSE under every failure | only bounded low-cardinality state appears; sensitive identity stays absent |
| Full repository | focused SQL/data/API/YAML/UI/package tests, generated OpenAPI/client, media fixtures, cleanup | `just ci`, `just ui-e2e`, strict Sonar with positive coverage, and every required check pass |

## Operator Questions

1. R1: Do you accept the normalized immutable root/binding/job contract and one
   catalog-wide generation fence, including explicit rebinding after an unrelated
   slot changes or source continuity is lost, and stale queued-job rejection?
2. R2: Do you approve explicit whole-root discovery via an empty association
   prefix, while candidate file paths remain nonempty and descriptor-confined?
3. R3: Do you approve the described HTTP/YAML/UI contract with path-bearing
   read-only root administration using the existing API-key boundary rather
   than a new administrator permission?
4. R4: Do you approve 128 active discovery associations per profile version as
   a hard capacity limit, not a demonstrated throughput claim?

R1-R4 and the shared G1 boundary were accepted according to ADR 559's resolution.
The questions above are retained as the decision's review history.

## Unresolved Gaps

- ADR 516's exact watcher and schedule timing, concurrency, queue, retry, and
  traversal values remain held. This proposal persists both automatic flags as
  false and does not authorize activation.
- Distinct final output, cross-slot publishing, root-path migration, catalog live
  reload, a finer-grained root-administrator auth scope, non-Linux support, and
  generation pruning remain outside scope and require separate approval if
  desired.
- Accepted non-root profile, policy, target, source-aggregate, backup, and worker
  contracts retain their own implementation sequencing. This proposal closes
  the root persistence interface they consume but does not claim those features
  are implemented.

## Follow-Up

- Implement only within the accepted R1-R4/G1 resolution and retained holds.
- Implement outside in: authenticated API/OpenAPI contract tests;
  UI root/profile/association workflows; typed API and application services;
  stored-procedure ABI and normalized final-init relations; bootstrap source and
  descriptor proof; profile/association persistence; five-row job admission and
  reader; runtime revalidation; package evidence; then the coordinated ADR 522
  and 541 init cutover.
- Keep the frozen migrations and current runtime behavior unchanged until the
  final cutover slice can remove every legacy path atomically.

## Task Record

- Motivation:
  - Close the remaining implementation-significant persistence and outside-in
    choices between accepted ADRs 523 and 550 before database or API code makes
    an unapproved architectural decision.
  - The 2026-09-09 revision makes the operator-visible decisions explicit and
    corrects the August draft's inability to select a whole source root. The
    user approved document revision, not these architectural recommendations.
- Design notes:
  - The recommendation separates stable logical slot identity, immutable
    generation-specific attestation, versioned resource bindings, and immutable
    job evidence.
  - Allowed kinds use normalized join rows. Source and generation digests are
    evidence, while the injected source and open descriptors remain filesystem
    authority.
  - Legacy profile-local roots are documented as cutover inputs only, not a
    compatibility contract.
  - The revised prefix contract is a proposal. No SQL, caller, UI, migration,
    package behavior, or accepted predecessor decision is changed in this task.
- Test coverage summary:
  - This documentation-only proposal adds no SQL, Rust, API, YAML, UI, package,
    or runtime test.
  - Applicable validation is policy, instruction drift, documentation generation
    and build, link checking, changed-line bounds, whitespace, and clean-tree
    verification.
- Observability updates:
  - No runtime observability changes are made. Future implementation is bounded
    to the state, reason, visibility, and cardinality contract above.
- Status-doc validation:
  - Reviewed migration 0182, the generated frozen init candidate,
    `revaer-data` media profile/job/identity modules, API models and routes, UI
    profile workflow, and accepted ADRs 517, 521, 523, 525, 541, 550, and 551.
  - Product, API, and operator guides remain unchanged because the proposal is
    not implemented. The ADR index and documentation summary expose this pending
    decision only.
- Risk & rollback plan:
  - The proposal changes no runtime behavior. Rejection removes or supersedes
    this record.
  - After an accepted implementation changes final init bytes, rollback must use
    a compatible signed artifact and verified database baseline or recreate a
    disposable v0 database. It must never restore raw path authority or dual
    writes as a fallback.
- Dependency rationale:
  - No dependency is added or proposed. The contract uses existing PostgreSQL,
    SQLx transaction, UUID, SHA-256, JSON/YAML, and Linux descriptor facilities.
    A future dependency requires separate written rationale and review.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/revaer-ui.instructions.md`, and
    `.github/instructions/devops.instructions.md`, plus the Sonar strictness
    boundary in `.github/instructions/sonarqube_mcp.instructions.md`.
  - This review found proposal drift: one nonempty path validator had also been
    used for association scope, excluding whole-root discovery; approval text
    mixed operational decisions with private implementation naming. The draft
    now distinguishes prefix from candidate semantics and presents G1 as pending,
    not as an approved weakening of architecture oversight.
  - The overlap critical section is proposed on the shared source attestation,
    rather than on one profile version, to match the cross-profile non-overlap
    invariant. It remains part of the pending relational contract, not a claim
    that a runtime race was reproduced or fixed.
  - Peer review found contradictory YAML versus API parent/source lock ordering.
    Both proposed paths now require the same catalog/source/parent ordering and
    a concurrent acceptance test; no executed concurrency proof is claimed.
  - Legacy implementation drift remains in profile-local root authority and the too-broad
    `media_key_valid_v1`; both are recorded for later init-only retirement or
    replacement without treating current behavior as authority.
  - No migration, raw runtime SQL, JSONB, array persistence, source suppression,
    criteria relaxation, dependency exception, or implementation claim is
    introduced. Architectural implementation remains blocked on exact operator
    approval.
