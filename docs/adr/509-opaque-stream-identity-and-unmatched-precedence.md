# Opaque stream identity and unmatched precedence

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- The PR 122, 127, 128, 133, and 138 replay introduced attachment and opaque
  data passthrough, container attachment policy, per-kind unmatched actions, and
  retained-attachment verification, but the historical ADRs do not contain
  decision-specific operator approval.
- The replay API accepts authored attachment and data target rows as exact
  passthrough selectors while the core desired-target compiler rejects those
  kinds. A durable contract cannot be valid at ingress and unsupported during
  authoritative compilation.
- Stream indexes are muxer-local and can change when streams are selected,
  removed, or reordered. Some replay execution and verification paths compare
  a retained source stream with an output stream by numeric stream id even when
  the desired graph has an explicit source-to-output binding.
- Attachment removal has two explicit controls: desired-container
  `container_attachment_policy` and policy-profile
  `unmatched_attachment_action`. The specification describes the former as an
  override but also contains a contradictory summary that says only `strip` can
  remove attachments.
- The five stream families need one total precedence order that also accounts
  for subtitle sidecars and keeps chapters outside ordinary stream handling.

## Options

1. **Author attachment and data target rows.** Support explicit passthrough
   selectors now and later extend the same surface to replacement or transform.
   This requires consistent selection, artifact, codec, materialization, and
   verification semantics that do not yet exist.
2. **Retain opaque streams through policy and source bindings.** Keep authored
   target rows to video, audio, and subtitle; make attachments and data
   unmatched source streams governed by explicit policy and verified through
   source-to-output bindings.
3. **Use only one generic unmatched action or only container strip.** Remove the
   per-kind matrix or make the container field the sole attachment decision.
   This is smaller but cannot express the documented mixed defaults and erases
   the distinction between target intent and operational fallback.

## Recommendation

- Adopt option 2.
- Authored desired-target stream rows support only video, audio, and subtitle.
  API, YAML, database procedures, snapshot reconstruction, and core compilation
  reject authored attachment, chapter, and data rows consistently. Attachment
  and data replacement or transformation requires a separate approved ADR.
- Apply unmatched handling in this exact order:
  1. Deterministically match authored video, audio, and subtitle target rows and
     mark their selected source streams as consumed.
  2. If `container_attachment_policy` is `strip`, remove every remaining source
     attachment without consulting `unmatched_attachment_action`.
  3. Apply the corresponding per-kind action to every remaining embedded source
     stream in source order.
  4. Apply `unmatched_subtitle_action` to every remaining discovered sidecar and
     its owned companion as one aggregate.
  5. Handle container chapters only through the independently proposed chapter
     policy; chapters never use an unmatched-stream action.
- Public and persisted unmatched actions are exactly `remove`, `preserve`, and
  `fail`. Defaults are video `fail`, audio `preserve`, subtitle `preserve`,
  attachment `preserve`, and data `remove`. New desired jobs snapshot all five
  explicit actions; a legacy generic value is compatibility input only and is
  never authoritative for newly created jobs.
- Attachment precedence is explicit: `strip` plus any attachment action removes;
  `preserve` plus unmatched `remove` removes; `preserve` plus unmatched
  `preserve` retains; and `preserve` plus unmatched `fail` stops planning. Any
  accepted implementation must correct documentation that claims `strip` is
  the only possible explicit attachment-removal path.
- `preserve` emits a source-to-output `DesiredStreamBinding`. The source stream
  id identifies the inspected source only; the output stream id identifies the
  desired position only. Equality never requires those numeric ids to match.
- Retained attachments and data use codec `copy`. Their kind, canonical codec,
  normalized language, title, dispositions, channel state, technical inspection
  state, metadata, and side data must equal the bound source state at candidate
  and final verification. No retained field may be authored or rewritten.
- Retained attachments additionally require exact extracted payload byte count
  and SHA-256 equality within the approved process and workspace limits. Data
  preservation proves source binding, copy command, codec identity, and exact
  normalized inspection state; it does not claim a cryptographic payload proof.

## Consequences

- All public and internal boundaries agree that attachment and data target rows
  are unsupported instead of accepting configurations that fail later.
- Output stream reindexing no longer creates a false passthrough failure, while
  an incorrect source binding, codec change, state change, or attachment payload
  change still fails closed.
- Both attachment-removal controls remain meaningful and explicit. Operators
  must understand that target `preserve` means attachment retention is eligible
  for the snapshotted unmatched policy, not unconditional retention.
- The mixed per-kind defaults prevent silent video loss while retaining ordinary
  audio, subtitles, and attachments and removing opaque data by default.
- Preserved data has a narrower verification claim than attachments. If byte
  identity becomes a product requirement, data `preserve` must fail until a
  separately bounded payload verifier is approved and implemented.
- API/OpenAPI, YAML, normalized policy and job snapshots, compilation, planning,
  command construction, verification, documentation, and UI labels must change
  together after approval.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only the supported authored stream kinds, exact
  precedence order, five action values and defaults, explicit snapshots,
  binding-based identity, copy-only opaque retention, exact retained-state
  checks, and attachment payload verification described above.
- Before v1, accepted persistence changes must modify
  `crates/revaer-data/init.sql`; historical migrations 0175, 0176, and 0190 are
  provenance only and must not be restored.
- The pure compiler approved by ADR 451 owns action normalization and precedence.
  The planner consumes the compiled desired graph, the executor consumes its
  bindings, and candidate and final verifiers re-inspect the bound streams.
- ADR 450 continues to own sidecar filename discovery and ownership. This ADR
  governs only the action applied after approved discovery leaves a sidecar
  unmatched.
- Acceptance would not authorize attachment or data replacement, attachment or
  data target selectors, data transcoding, a new sidecar grammar, fuzzy codec
  matching, output-index identity, caller-authored bindings, live policy reads,
  or relaxation of process, workspace, cancellation, or verification bounds.
- No schema, runtime, API, YAML, generated-contract, or UI behavior may change
  until this ADR receives explicit decision-specific operator approval.

## Validation

- Proposal validation is documentation-only; this record changes no stream,
  attachment, sidecar, planner, or verifier behavior.
- After approval, add an exhaustive table for every stream kind and every
  `remove | preserve | fail` action, both attachment policies, matched and
  unmatched streams, multiple source streams, and subtitle sidecars with owned
  companions.
- Prove API, OpenAPI, YAML, stored procedures, snapshot reconstruction, and core
  compilation all reject authored attachment, chapter, and data target rows.
- Add reordered and compacted output-index tests proving every execution and
  verification path resolves `DesiredStreamBinding`, plus negative tests for
  missing, duplicate, mismatched, and caller-authored bindings.
- Add exact codec and retained-state mutation tests and real FFmpeg fixtures for
  attachment and data copy. Attachment fixtures must cover payload equality,
  payload drift, count and byte ceilings, symlink roots, timeouts, cancellation,
  cleanup, candidate failure, final failure, quarantine, and rollback.
- Prove the legacy generic action is never written for a new job and cannot
  override the five immutable snapshot values.
- An accepted implementation is not complete until `just ci` and
  `just ui-e2e` pass in addition to focused tests.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- Relevant replay commits are PR 122 `1d1fd262`, PR 127 `19e2d057`, PR 128
  `f4dafad0`, PR 133 `470429c5`, and PR 138 `db8a9650`.
- Historical ADRs 368, 369, 370, 375, 376, and the retained-attachment portion
  of ADR 421, plus historical migrations 0175, 0176, and 0190, describe the
  source-stack behavior. Their `Accepted` labels are not operator approval.
- PR 139 `910ed071` was inspected because matched-audio materialization must not
  alter unmatched-audio preservation. It adds no migration and does not define
  unmatched precedence.
- No local `replay/pr-125` ref or attributable PR 125 commit was present, so no
  decision or implementation claim is derived from PR 125.
- Approved ADRs 442, 446, 449, 450, and 451 continue to govern planning,
  immutable snapshots, capability binding, sidecar discovery, and policy
  compilation. This proposal does not reopen them.

## Follow-up

- Obtain explicit decision-specific operator approval before implementation or
  before changing this ADR to `Accepted`.
- If approved, correct the API/core target-kind divergence and numeric-id
  identity defects before expanding any opaque-stream surface.
- Reconcile the contradictory attachment-removal language in
  `MEDIA_TRANSCODING.md`, then update OpenAPI, YAML examples, generated clients,
  policy UI labels, and verification documentation in the implementation change.

## Task Record

- Motivation:
  - Resolve unapproved and internally inconsistent attachment, data, identity,
    and per-kind unmatched behavior before replaying it into the active stack.
- Design notes:
  - The recommendation keeps opaque streams source-owned and policy-retained.
    `DesiredStreamBinding` separates durable source provenance from mutable
    output order.
  - Attachment `strip` is the highest-precedence target rule; otherwise the
    immutable per-kind policy decides the remaining unmatched stream.
- Test coverage summary:
  - This proposal adds no runtime, API, schema, planner, verifier, or UI tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this proposal-only change.
  - The matrix and full validation listed above remains mandatory after any
    approval and implementation.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation should persist bounded per-kind planning and
    retained-state verification reason codes. It must not log attachment
    payloads, metadata values, source paths, fingerprints, or identifiers as
    metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, historical ADRs 368-370, 375, 376, and 421,
    and approved ADRs 442, 446, 449-451. This proposal makes no claim that the
    recommended precedence or identity contract is approved or implemented.
  - `README.md`, roadmap/status documents, and operator guides are unchanged;
    only the ADR index and documentation summary expose this pending proposal.
- Risk & rollback plan:
  - This proposal changes no production behavior and can be rolled back by
    reverting this ADR and its two index entries.
  - After implementation, rollback must preserve immutable queued-job actions
    and source bindings. Jobs carrying incompatible snapshots must drain or fail
    explicitly rather than reading mutable profile state.
- Dependency rationale:
  - No dependency is proposed. Existing bindings, normalization, hashing,
    process, and workspace primitives are sufficient for the recommended scope.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`, and the approved ADRs
    that constrain prospective implementation.
  - No repository-policy drift was found. The source-stack API/core divergence
    and contradictory attachment-removal statements are product-contract drift
    recorded for resolution only after approval.
