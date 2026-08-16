# Normalized media operator resource contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Accepted ADR 484 defines seven outside-in operator workflows but does not
  define the durable API/YAML resource boundaries, atomic mutation scope,
  optimistic versioning, or request bounds.
- Current models expose only part of the normalized configuration. Treating YAML
  as the advanced editor or adding loosely typed maps would bypass the relational
  and stored-procedure contracts.
- Jobs must continue to reference immutable profile, target, and policy versions
  while operators edit newer configuration.

## Options

1. **Expose individual table rows and PATCH operations.** This mirrors storage
   but permits partially valid aggregates and makes YAML application complex.
2. **Expose typed versioned aggregates backed by normalized rows.** Replace one
   complete aggregate atomically, require optimistic concurrency, and use the
   same semantic compiler for JSON and YAML.
3. **Use one untyped configuration document.** This is convenient for clients
   but violates normalized application state and weakens field ownership.

## Recommendation

- Adopt option 2.
- The operator contract contains exactly these top-level resources:

| Resource | Owned content |
| --- | --- |
| Instance root | one host-local ADR 523 root-catalog slot and attestation state; never portable YAML |
| Compatibility target | one versioned playback capability target |
| Desired target | one versioned container and 1-64 ordered typed stream rows |
| Media policy | one versioned complete ADR 518 policy aggregate and ordered child families |
| Media profile | display state and immutable references to one target version, one policy version, and required root binding keys |
| Discovery association | one non-overlapping source-root-relative path, exactly one profile version, explicit manual/watcher/schedule state |
| Job-retention policy | one versioned instance policy for completed and failed diagnostic history |

- Existing collection paths remain canonical where they already name these
  resources. Add `/v1/media/root-catalog` and
  `/v1/media/discovery-associations` rather than hiding those contracts inside
  profile booleans. Jobs, capabilities, planning preview, and commands remain
  operational resources, not configuration aggregates.
- JSON uses explicit tagged enums and typed child arrays. Unknown fields,
  duplicate keys, duplicate order, unknown enum values, noncanonical language or
  codec values, and unsupported contract versions are rejected. Generic maps,
  arbitrary native arguments, JSONB, and opaque extension bags are forbidden.

### Versioned Mutation Contract

- `POST` creates version 1 with `If-None-Match: *`. `PUT` replaces one complete
  aggregate and requires the strong ETag returned by its latest representation:
  `"<resource-kind>:<public-id>:v<positive-version>"`.
- A successful `PUT` appends immutable version `n + 1` and all normalized child
  rows in one `SERIALIZABLE` stored-procedure transaction. It never updates rows
  referenced by a queued job. The response is the complete new representation
  and ETag.
- Missing `If-Match` returns 428. A stale tag returns 412 with
  `media_configuration_version_conflict`; no row changes. Create-key conflict
  returns 409. Collection reads never provide a writable version token for a
  different resource.
- `PATCH` is not supported for versioned configuration. Archive is an explicit
  command that appends a disabled version; physical deletion is forbidden while
  any job, profile, bundle audit, or retained compact fact references a version.
- Each aggregate has one public UUID and one normalized immutable key of 1-64
  lowercase ASCII letters, digits, and hyphens. Display names are 1-128 UTF-8
  bytes and descriptions are at most 1,024 UTF-8 bytes. Normalization never
  silently truncates.

### Collection And Child Bounds

- Collection pagination is hard-capped keyset pagination: default 50, maximum
  200, ordered by normalized key and internal stable id. Offset pagination is
  not accepted for configuration collections.
- One aggregate request is at most 1 MiB after decompression and contains at most:
  - five root binding keys on a profile;
  - 128 file rules;
  - four subtitle-discovery rules;
  - 128 retention rules;
  - 32 compatibility targets;
  - exactly 13 operation-cost rows;
  - 128 classification rules;
  - 32 maintenance windows;
  - 64 desired streams.
- Child state from accepted ADRs 507-511 and the typed structure from ADR 515
  retain their smaller field and aggregate bounds and also count toward the 1 MiB
  request. Unresolved exact values create no field, default, or endpoint.
- A discovery association path is root-relative, valid UTF-8, 1-4,096 bytes,
  normalized without `.` or `..`, and contains no absolute prefix. Watcher and
  schedule fields remain disabled or rejected until ADR 516's exact timings and
  budgets receive separate approval and its validation gates pass.

### YAML Contract

- YAML uses `format_version: 1` and
  `kind: revaer.media.profile_bundle`. It contains only typed arrays named
  `compatibility_targets`, `targets`, `policies`, `profiles`,
  `discovery_associations`, and optionally `job_retention_policy`.
- Portable YAML never contains canonical host paths, filesystem identities,
  executable paths, credentials, capability runs, job evidence, or instance-root
  attestation. Profiles and associations reference logical root keys. An import
  with an unmapped key remains a disabled draft and cannot create a job.
- Validation parses to the same typed request models and invokes the same pure
  compiler as JSON without writes. Apply requires an explicit expected version
  for every existing resource and create intent for every new key.
- One bundle is at most 4 MiB, 128 top-level resources, and 4,096 total child
  rows. Apply locks resources in kind/key order and commits all normalized rows,
  references, versions, and one bounded import audit in one `SERIALIZABLE`
  transaction. Any error rolls back the entire bundle.
- YAML aliases, custom tags, duplicate mapping keys, multiple documents, unknown
  fields, non-string map keys, and recursive structures are rejected. Export
  emits one canonical key order and explicit normalized values; it does not
  preserve comments or caller formatting.

### Transactions And Errors

- API handlers call one application service per resource or bundle. The data
  adapter calls only versioned stored procedures; handlers and domain code do not
  issue raw SQL or create concrete dependencies.
- Stable problem details identify resource kind, bounded field pointer, and one
  code such as `media_configuration_invalid`,
  `media_configuration_bound_exceeded`, `media_configuration_reference_missing`,
  `media_configuration_version_conflict`, `media_configuration_overlap`,
  `media_configuration_pending_contract`, or `media_configuration_root_unmapped`.
- Validation errors expose no host path outside the root-catalog administration
  surface and no metadata value, credential, native command, or SQL detail.

## Consequences

- API, UI, YAML, persistence, and immutable job snapshots share one typed
  configuration model and atomic version boundary.
- Whole-aggregate replacement is less chatty than row PATCH and prevents partial
  validity, but clients must resend bounded child collections.
- Strong ETags and immutable versions make concurrent edits explicit and keep
  queued jobs reproducible.
- Host-local root authority stays outside portable exchange while profiles remain
  shareable through logical keys.

## Implementation Boundary

- This accepted ADR authorizes only the seven resources, endpoint ownership,
  complete replacement, ETag/version rules, transaction scopes, bounds, YAML
  structure, draft root mapping, and stable errors described above.
- Accepted ADRs 446-451, 484, 517, 518, 523, and 524 remain binding. Their rows and
  compiler must be exposed as one coordinated contract.
- This ADR does not activate watchers or schedules, invent unresolved exact values
  from ADRs 515 and 516, authorize JSONB or generic extension maps, place raw paths
  in portable YAML, permit arbitrary commands, or allow partial aggregate writes.
- Schema, API, OpenAPI, YAML, UI, generated-client, workflow, and runtime changes
  must remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only.
- During implementation, add contract tests for create, replace, archive, every bound,
  unknown/duplicate fields, canonical enums, 428, 412, 409, and concurrent PUTs.
- Add direct stored-procedure tests proving an application caller cannot bypass
  version checks, references, child bounds, overlap checks, or compiler validity.
- Add YAML parser abuse tests, canonical round trips, all-or-none bundle rollback,
  unmapped root drafts, and byte/row/resource limits.
- Add focused UI workflows for all seven ADR 484 workflows with stale-request
  fencing, accessible validation, loading, empty, and error states.
- An accepted implementation is not complete until OpenAPI generation, UI E2E,
  `just ci`, and `just ui-e2e` pass.

## Follow-up

- Inventory existing routes against the seven resources and preserve compatible
  route names without preserving incomplete semantics.
- Implement accepted ADRs 517, 518, 523, and 524 together before freezing OpenAPI
  or YAML version 1.

## Task Record

- Motivation:
  - Complete the API/YAML and transaction decision deferred by accepted ADR 484.
- Design notes:
  - Public aggregates are typed and versioned while durable child state remains
    normalized relational rows.
- Test coverage summary:
  - The ADR-only change added no API, YAML, database, UI, or E2E tests.
- Observability updates:
  - Future metrics may use bounded resource kind and outcome only. Keys, public
    ids, paths, ETags, and validation values must not be labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 446-451, 484, 500, and 501,
    and accepted ADRs 507-516. Unresolved exact-value fields remain unavailable.
- Risk & rollback plan:
  - Any reversal requires a superseding ADR. A later rollback must retain
    immutable versions referenced by jobs and reject unknown versions.
- Dependency rationale:
  - No new dependency is required. Existing HTTP, YAML, typed model, transaction,
    and stored-procedure facilities are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/revaer-ui.instructions.md`, and
    `.github/instructions/rust.instructions.md`; no drift or relaxation was
    found.
