# Exact container metadata state

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- The PR 122-124 replay introduced container metadata policy, exact
  preservation, stripping, and authored values, but its historical ADRs record
  implementation state rather than decision-specific operator approval.
- Container metadata crosses API and YAML input, normalized persistence,
  immutable job snapshots, desired-graph compilation, FFmpeg command
  materialization, and candidate and final verification. Those boundaries need
  one definition of the expected final state.
- `preserve`, `strip`, full replacement, and merge have materially different
  meanings. A merge contract leaves inherited keys, duplicate keys, and
  muxer-authored values ambiguous and prevents exact verification.
- Runtime configuration must remain normalized relational state. YAML is an
  exchange format only, runtime database behavior must use stored procedures,
  and pre-v1 schema work must update `crates/revaer-data/init.sql` rather than
  restore historical migration files.

## Options

1. **Preserve or strip only.** Permit exact source preservation or complete
   removal, but defer all authored metadata. This is small but cannot express a
   complete operator-authored container state.
2. **Preserve, strip, or exact full replacement.** Compile one complete expected
   metadata set and compare candidate and final inspection against it.
3. **Merge or patch source metadata.** Apply authored additions and removals to
   inspected source values. This is flexible but makes inheritance, duplicate
   handling, command order, and verification more complex.

## Recommendation

- Adopt option 2.
- `container_metadata_policy` has exactly three normalized values:
  `preserve`, `strip`, and `replace`. Missing API or YAML input defaults to
  `preserve`; persisted desired-target and job-snapshot state is always
  explicit.
- `preserve` forbids authored metadata rows and compiles the normalized source
  metadata as the complete expected set, including an empty set.
- `strip` forbids authored metadata rows and compiles an empty expected set.
- `replace` requires at least one authored row and treats those rows as the
  complete expected set. It never inherits or merges a source value.
- Inspection normalization trims keys and values, lowercases keys, removes
  blank entries, sorts deterministically, and removes exact duplicate pairs.
  Authored replacement additionally rejects a repeated canonical key.
- Authored replacement is bounded to 64 rows, 128 UTF-8 bytes per key, 4,096
  UTF-8 bytes per value, and 65,536 aggregate key-and-value bytes. Blank keys or
  values and values exceeding any bound fail before a job can carry them.
- Materialization is deterministic: preserve maps source container metadata,
  strip disables metadata mapping, and replace first disables inherited
  metadata and then emits every compiled key/value exactly once in canonical
  order.
- Candidate and final verification compare normalized inspection state with the
  complete compiled set. Missing values, changed values, inherited values, and
  muxer-authored additions are mismatches and prevent replacement.

## Consequences

- Operators can distinguish exact source retention, intentional erasure, and a
  complete authored state without hidden inheritance.
- The compiler, command builder, and verifier share one bounded value model, so
  a value accepted at ingress cannot acquire a different meaning later.
- Some muxers add container tags automatically. Under this contract those tags
  are verification failures unless the compiled expected set contains their
  normalized values; implementation must not silently filter them to obtain a
  pass.
- Existing clients that treat replacement as a merge must change before
  release. Replacement with zero rows remains invalid; explicit erasure uses
  `strip`.
- Desired-target values and immutable job values require normalized parent and
  child rows plus stored-procedure validation. JSON or caller-authored FFmpeg
  arguments are not permitted.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only the three policy values, defaults,
  normalization, resource bounds, normalized desired-target and immutable
  job-snapshot rows, deterministic materialization, and exact candidate and
  final verification described above.
- API requests, OpenAPI models, YAML import/export, the pure effective-policy
  and desired-target compilers, stored procedures, command construction, and
  verification may change only as needed to carry this exact contract.
- Before v1, any accepted persistence change must modify
  `crates/revaer-data/init.sql`; historical migrations 0170, 0171, and 0173 are
  provenance only and must not be restored.
- Acceptance would not authorize metadata merge or patch operations, per-stream
  metadata editing, chapter metadata semantics, arbitrary FFmpeg arguments,
  verifier exclusions, live-profile reads, or relaxation of the approved
  capability and immutable-snapshot boundaries.
- No schema, runtime, API, YAML, generated-contract, or UI behavior may change
  until this ADR receives explicit decision-specific operator approval.

## Validation

- Proposal validation is limited to documentation and repository-policy checks;
  no runtime, API, or schema behavior is changed by this record.
- After approval, add table-driven tests for all policy and row-shape
  combinations, normalization, duplicate canonical keys, every resource bound,
  defaulting, and direct stored-procedure calls that try to bypass application
  validation.
- Add API, OpenAPI, and YAML round-trip tests proving all surfaces emit the same
  explicit canonical contract.
- Add planner and command tests for no-op preservation, strip, replacement, and
  muxer capability failure, including exact argument ordering and argument
  budget enforcement.
- Add real-media candidate and final fixtures covering empty and nonempty
  preservation, complete stripping, complete replacement, muxer-added tags,
  verification failure, quarantine, and rollback.
- An accepted implementation is not complete until `just ci` and
  `just ui-e2e` pass in addition to the focused tests.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- Relevant replay commits are PR 122 `1d1fd262` and PR 124 `5a340587`.
- Historical ADRs 365, 366, 371, and 373 and historical migrations 0170, 0171,
  and 0173 describe the implemented source-stack behavior. Their `Accepted`
  labels are not treated as operator approval.
- Approved ADRs 442, 446, and 451 already govern capability-derived planning,
  immutable worker snapshot access, and pure effective-policy compilation.
  This proposal does not reopen those decisions.

## Follow-up

- Obtain explicit decision-specific operator approval before changing any
  implementation or changing this ADR to `Accepted`.
- If approved, implement the contract outside-in from API/YAML and stored
  procedures through compilation, planning, command materialization, and both
  verification stages.
- Reconcile `MEDIA_TRANSCODING.md`, OpenAPI, operator UI labels, and generated
  clients with the accepted semantics in the same implementation change.

## Task Record

- Motivation:
  - Convert the unapproved PR 122-124 metadata behavior into one explicit
    operator decision before any replay or correction is implemented.
- Design notes:
  - The recommendation models metadata as a complete declarative final state.
    It keeps `strip` distinct from nonempty `replace` and deliberately rejects
    merge semantics.
  - Expected state is compiled once and consumed by planning, materialization,
    and verification rather than reinterpreted independently.
- Test coverage summary:
  - This change is proposal-only and adds no runtime, API, schema, or UI tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this proposal-only change.
  - The focused and full validation listed above remains mandatory after any
    approval and implementation.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation should persist bounded verification outcomes and
    stable reason codes without logging metadata values or using target, job,
    source, or metadata identifiers as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, the source replay ADRs and commits, and the
    approved planning and snapshot ADRs. This proposal makes no claim that the
    recommended contract is approved or implemented.
  - `README.md`, roadmap/status documents, and operator guides are unchanged;
    the ADR index and documentation summary are updated only to expose this
    pending proposal.
- Risk & rollback plan:
  - This proposal changes no production behavior and can be rolled back by
    reverting this ADR and its two index entries.
  - After implementation, rollback must preserve immutable queued-job semantics;
    incompatible queued snapshots must drain or be rejected explicitly rather
    than being reinterpreted.
- Dependency rationale:
  - No dependency is proposed. Exact normalization and resource accounting can
    use existing Rust and database primitives.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as prospective
    implementation constraints.
  - No policy drift was found. The historical migration references are
    intentionally provenance-only under the current pre-v1 `init.sql` rule.
