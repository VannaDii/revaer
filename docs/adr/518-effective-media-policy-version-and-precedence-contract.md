# Effective media policy version and precedence contract

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- Accepted ADR 451 assigns effective-policy compilation to a pure Rust domain
  compiler, but it leaves the exact family inventory, precedence, identity hash,
  and compiler-version behavior undecided.
- Distributed defaults already exist in SQL, API conversion, the planner, and
  runtime constants. Reproducing those defaults inside a new compiler would
  preserve ambiguity rather than establish one immutable contract.
- Accepted ADRs 507-511, 515, and 516 describe additional target and discovery
  semantics. The unresolved exact values in ADRs 515 and 516 cannot be treated as
  compiler inputs or permissive defaults before separate approval.

## Options

1. **Compile only fields used by the current runtime.** This is smaller but
   continues to ignore persisted policy families and cannot satisfy ADR 451.
2. **Compile one exhaustive, versioned domain value.** Require every accepted
   family, apply a closed precedence table, and derive one canonical identity
   consumed by planning, execution, verification, and audit.
3. **Let each subsystem compile its own view.** This avoids a broad type but
   permits discovery, planning, execution, and verification to disagree.

## Recommendation

- Adopt option 2.
- The pure entry point is conceptually
  `compile_effective_media_policy(snapshot, contract_version) -> Result<EffectiveMediaPolicy, PolicyCompileError>`.
  It receives only the bounded persistence DTO from ADR 517. It performs no I/O,
  environment reads, clock reads, capability discovery, path resolution,
  filesystem inspection, logging, or adapter construction.
- `EffectiveMediaPolicyV1` contains these exhaustive families:

| Family | Required compiled content |
| --- | --- |
| Identity | profile configuration version, policy key/version, target key/version, snapshot contract version |
| File selection | ordered include/exclude rules and one explicit size/duration/sample/trailer/trash/quarantine filter |
| Sidecar discovery | explicit disabled state or one compiled ADR 524 rule set; no built-in patterns |
| Stream classification | ordered typed stream-kind, role, matcher, and confidence rules |
| Desired target | one container contract and 1-64 ordered video/audio/subtitle target streams |
| Stream disposition | ordered retention rules and explicit per-kind unmatched actions |
| Compatibility | ordered target identities, unsupported-format action, and require-all behavior |
| Planning cost | a complete map for every v1 operation kind and whether the operation is permitted |
| Runtime control | concurrency, retry, absolute runtime, I/O, free-space, battery, and thermal constraints |
| Maintenance | ordered weekly windows with one explicit inside/outside interpretation |
| Output | dry-run, replacement mode, quarantine, permission, and ownership behavior |
| Workspace and backup | retention, diagnostics, cleanup, maximum bytes, backup enablement, backup retention, and reserves |
| Verification | strictness plus duration, mux, decode, keyframe, and playback requirements |
| Root requirements | five stable binding ids and required root-kind semantics, but no host path or filesystem handle |

- Source inspection, resolved host paths, capability evidence, command lines,
  attempts, checkpoints, clocks, and mutable job state are not policy families.
  They remain separate inputs to later planning or execution boundaries.
- The v1 operation-cost map contains exactly these thirteen normalized keys:
  `no_op`, `remux`, `metadata_rewrite`, `disposition_rewrite`, `label_rewrite`,
  `stream_reorder`, `embed_subtitle`, `extract_subtitle`,
  `copy_sidecar_subtitle`, `remove_sidecar_subtitle`, `subtitle_transcode`,
  `audio_transcode`, and `video_transcode`. A missing or unknown key is an error.
  `enabled = false` forbids that operation; it is not an infinite or zero cost.

### Closed Precedence Table

1. Repository safety invariants and accepted architecture boundaries are
   absolute. No profile value can permit root escape, unbounded work, unsupported
   native execution, mutation during dry-run, or replacement before verification.
2. The immutable desired-target graph owns matched output state. A retention or
   unmatched rule cannot rewrite a stream already bound to a target row.
3. For source streams and sidecars left unmatched by the target, accepted ADR 509
   owns per-kind precedence. Compilation must implement its exact action values
   without a fallback or alternate precedence.
4. Ordered rule families use first matching **enabled** row. Disabled rows remain
   in snapshot identity but never match. Equal precedence or duplicate order is
   invalid; there is no database-order tie break.
5. Explicit scalar behavior rows are complete values, not optional overrides.
   Defaults are materialized when a new version is authored and snapshotted;
   the worker compiler never consults a current default.
6. Runtime, maintenance, capacity, and compatibility constraints may remove
   candidate plans or delay work. They may not relax target, verification,
   source-identity, process, or replacement requirements.
7. Verification strictness may add checks. It may not disable exact checks
   required by an accepted target contract, including ADRs 507-511 and the
   separately approved exact values carried by ADR 515.
8. `dry_run = true` dominates every mutation setting. Backup, quarantine, and
   replacement fields remain compiled for explanation but cannot authorize a
   filesystem mutation.
9. Backup enablement requires one accepted backup root binding and complete
   backup policy. Quarantine enablement similarly requires its root binding.
   Missing bindings are compile errors, not reasons to disable behavior.
10. No live profile, target, capability run, environment value, or application
    constant participates as a final fallback.

### Identity And Versioning

- Persist `policy_contract_version` and `policy_compiler_version` with every job
  snapshot. The first supported pair is `(1, 1)`.
- The effective identity is the tuple `(policy_contract_version,
  policy_compiler_version, sha256)`. The digest uses the same framed primitives
  as ADR 517 but a distinct domain prefix:
  `revaer-effective-media-policy`, zero byte, contract version, compiler version.
- The digest covers every compiled semantic value in the family order above,
  normalized enum ordinals, ordered collections, operation permission and cost,
  stable root binding ids, and accepted nested target-contract versions. It
  excludes capture timestamps, database row ids, host paths, capability runs,
  source facts, command text, and diagnostics.
- Snapshot creation compiles and stores the expected effective digest before a
  job is admitted. The worker recompiles the loaded DTO and requires exact
  identity equality before planning.
- Any change to defaults, normalization, precedence, cross-field validation, or
  hash framing increments `policy_compiler_version`. Any change to the accepted
  family or field contract increments `policy_contract_version`. Existing jobs
  retain their recorded pair and are never reinterpreted by a newer compiler.
- A binary may support multiple explicit compiler versions. An unsupported pair
  yields `media_policy_compiler_version_unsupported` and requires the explicit
  ADR 520 re-plan path or an older compatible runtime; it does not auto-upgrade.

### Stable Compile Errors

- Use a closed error enum including `family_missing`, `family_incomplete`,
  `unknown_value`, `duplicate_rule_order`, `rule_conflict`,
  `operation_cost_incomplete`, `root_binding_incomplete`,
  `cross_field_invariant`, `pending_contract_unsupported`,
  `compiler_version_unsupported`, and `effective_identity_mismatch`.
- Errors carry bounded family and field enums and an optional row ordinal. They
  do not contain paths, metadata values, YAML fragments, or command arguments.

## Consequences

- Every runtime phase consumes one immutable semantic value instead of selecting
  its own defaults.
- Versioning prevents a deploy from silently changing queued-job meaning, at the
  cost of retaining explicit compiler implementations while supported jobs
  exist.
- Unimplemented accepted target contracts and unresolved exact audio values cannot
  leak into production through existing rows. They require a deliberate contract-
  version change, and exact audio values require separate approval.
- The compiler type is broad, but it remains pure and decomposable into private
  family validators and values rather than coupling domain logic to persistence.

## Implementation Boundary

- This accepted ADR authorizes only the v1 family inventory, thirteen operation
  keys, precedence table, domain separation, identity tuple, version rules, and
  stable compile-error shape described above.
- This ADR incorporates fields authorized by accepted ADRs 507-514. ADRs 515 and
  516 contribute only their accepted typed structures until exact values receive
  separate approval; any later exact fields enter through a new or explicitly
  revised contract version and retain their own implementation boundaries.
- ADR 517 owns persistence transport; ADR 519 owns executable capability
  identity; ADR 523 owns host root resolution. This compiler may validate their
  stable references but may not perform their I/O.
- SQL, Rust, API, YAML, UI, workflow, generated-contract, and runtime changes must
  remain limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only.
- During implementation, add table-driven tests that remove, duplicate, disable, reorder,
  or alter every family and prove one stable result or error.
- Add precedence matrices for matched and unmatched streams, conflicting ordered
  rules, dry-run versus every mutation flag, compatibility versus target
  exactness, and verification strictness versus mandatory checks.
- Publish known-answer hashes for minimum and maximum policies and prove database,
  API, YAML, and worker round trips preserve them.
- Add compatibility tests proving compiler version 1 remains stable after a
  version 2 implementation is introduced and unsupported versions fail closed.
- An accepted implementation is not complete until focused tests, `just ci`,
  and `just ui-e2e` pass.

## Follow-up

- Implement accepted ADRs 517, 521, 523, and 524 together so the first DTO, operator surface,
  root references, and compiler inventory agree.
- Implement accepted ADRs 507-514 without hidden fields. Preserve the unresolved
  exact-value holds in ADRs 515 and 516 until separate approval.

## Task Record

### Operation-Cost Family, 2026-09-21 (Incomplete Aggregate)

- Added the pure typed operation-cost family in `revaer-media-core::policy`
  for the configuration-save dependency. It requires exactly the thirteen
  approved operations, rejects duplicate/missing rows, retains explicit enabled
  flags, validates the existing persisted weight bound, and canonicalizes order.
  No default, I/O, database change or new dependency was introduced.
- Four focused validation tests passed; the complete media-core suite passed
  (104 unit tests and one integration test). Strict Clippy initially identified
  a test field-order issue, which was corrected. This is domain-model evidence,
  not real service, persistence, complete compiler or release qualification.
- Remaining: other complete policy families, aggregate validation and immutable
  save integration. Runtime integration evidence is recorded below; this type
  alone is not a complete effective policy or operation authority.
- Added core `prune_with_policy`: explicit weights drive ranking and recorded
  totals, disabled operations are rejected, and the injected safety validator
  remains mandatory. Selected costs are retained instead of recomputed from
  defaults. Checked addition rejects overflow. Full core tests passed (107 unit
  tests plus one integration test), including reversed cost ranking, disabled
  zero-cost operation rejection and independent safety rejection. Strict core
  Clippy passed. Service/pipeline injection and populated-snapshot tests remain
  incomplete; no real job or complete policy-save qualification is claimed.
- Service connection: the injected MediaStore reads job-captured cost rows
  before inspection. The app rejects missing/extra kinds, unknown kinds,
  duplicate/negative ordering and invalid weights without a live-policy fallback.
  Embedded-output planning calls the explicit-policy pipeline. Every ready
  preflight, including the separate sidecar path, rejects disabled operations.
  Sidecar alternative ranking is not qualified by this change.
- Three scoped app tests passed for row validation, disabled zero-cost values
  and sidecar permission rejection. The core suite passed with a pipeline test
  proving authored costs reach selection and explanation totals. Strict scoped
  app/runtime/core Clippy passed after completing an exhaustive overflow-error
  match and extracting the permission guard to satisfy the function-length rule.
  Populated database-to-job evidence remains pending; existing incomplete policy
  fixtures now need explicit rows, not bypasses or inferred defaults. No complete
  service workflow, full CI/UI, Sonar quality or package qualification claimed.
- Reviewed AGENTS.md, Rust guidance and approved ADRs 518/521. No criteria or
  architectural scope changed. No observability changes. Rollback is removal
  of the unshipped module/export; no stored state is affected.

### Snapshot Reader, 2026-09-21 (Integration Incomplete)

- Added a stored-procedure reader and typed Rust adapter for the existing
  immutable job operation-cost snapshot. It reads no live policy, retains
  disabled rows and returns at most fourteen rows so overflow beyond thirteen
  remains detectable. The single init's runtime grant list includes only this
  read procedure; no direct table grant or migration was added.
- The repository sealed-init fixture passed on an owned disposable PostgreSQL
  16.14 container using the pinned image and generated, unpublished credentials.
  It proves restricted-runtime execution and the absent-job result, not populated
  snapshot correctness or planner use. The first attempt failed for a missing
  test database URL; the provisioned rerun passed. Container cleanup completed.
- Strict data Clippy passed. Existing scanner authorization excludes the external
  temporary harness; its upload was rejected and no contents were read or used.
  Repository-owned fixture support was used instead. Secrets checks passed on
  changed repository files. Full release gates remain outstanding.
- Runtime fixture validation: replaced this fixture's migration bootstrap with
  atomic application of the committed single init to an owned test database.
  It now authors all thirteen costs explicitly through the existing append
  procedure's typed adapter and fails on missing database configuration instead
  of returning an optional fixture and silently skipping tests.
- Both targeted database-backed cases passed: captured rows survive into a
  completed dry-run, while absent costs fail with the bounded policy error.
  Both preserve the filesystem snapshot and execute no commands. The complete
  runtime test module then passed: 66 passed, zero failed/ignored, sequentially
  on disposable PostgreSQL 16.14. This includes existing cancellation,
  verification and rollback tests. Inspectors/runners are injected; this is not
  real-media, UI, restricted-runtime-role or packaged-service qualification.
- Strict scoped app/data/test-support Clippy, formatting and secrets checks
  passed. An initial compile failure required the fixture SQL input to have a
  static lifetime; fixed before the successful runs. Owned containers were
  removed. No dependency or production migration path was introduced.
- Next: complete immutable configuration-save integration; explicit-cost
  snapshot overflow/isolation coverage and full release qualification remain.
  Root/data/Rust
  guidance reviewed; no new architecture, dependency or observability behavior.
  Rollback removes the unshipped reader, grant and adapter, leaving snapshots
  intact. This does not complete immutable configuration save.

- Motivation:
  - Complete the semantic and versioning decision deferred by accepted ADR 451.
- Design notes:
  - The effective value is exhaustive but excludes I/O identities and mutable
    execution state.
- Test coverage summary:
  - The ADR-only change added no compiler or behavioral tests.
- Observability updates:
  - Future telemetry may use bounded compiler version, family, and error enums.
    Digests, policy keys, paths, and row values must not become metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, accepted ADRs 446-451, 484, 500, and 501,
    and accepted ADRs 507-516. The unresolved exact values in ADRs 515 and 516
    remain unavailable.
- Risk & rollback plan:
  - Any reversal requires a superseding ADR. A later rollback must keep old
    compiler versions available or hold incompatible jobs explicitly.
- Dependency rationale:
  - No new dependency is required; existing typed domain and SHA-256 facilities are
    sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md`; no drift or relaxation
    was found.
