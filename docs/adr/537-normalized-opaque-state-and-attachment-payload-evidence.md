# Normalized opaque state and attachment payload evidence

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- ADR 509 requires source-bound retained attachments and data streams to match
  exact normalized metadata and side data. It also requires exact attachment
  payload byte count and SHA-256 evidence.
- The accepted contract does not define a normalized metadata and side-data
  schema, how unknown side-data kinds fail, or which component extracts an
  attachment payload without exceeding approved process and workspace limits.
- Raw probe JSON, field-order-sensitive text, output stream indexes, and opaque
  blobs are not stable normalized state. They also conflict with the repository
  prohibition on JSONB application state.
- An unbounded extractor could consume arbitrary time, disk, memory, output, or
  worker capacity. Inventing local numeric limits while implementing it would
  create unapproved product and operational policy.

## Options

1. **Compare raw inspector output.** Persist or hash the relevant probe JSON and
   compare it after copy. This is easy to begin, but parser ordering, omitted
   defaults, version drift, and unrelated fields make the result unstable and
   non-normalized.
2. **Verify only codec and stream mapping.** Treat metadata, side data, and
   attachment bytes as outside the retained-state claim. This is smaller but
   contradicts ADR 509's exact retention contract.
3. **Normalize retained opaque state and inject a policy-bounded payload
   extractor.** Persist closed metadata and side-data values relationally,
   resolve them through immutable source bindings, and derive attachment byte
   count and SHA-256 through an injected extractor that consumes only approved
   limits.

## Recommendation

- Adopt option 3.
- Define a versioned normalized opaque-stream snapshot keyed by the immutable
  source aggregate, source stream binding, stream kind, inspection contract,
  and capability/executable identities required by ADRs 449 and 519.
- Store stream metadata as normalized child rows with canonical key and value,
  deterministic ordinal, and explicit normalization version. Canonicalization
  must be shared by source, candidate, and final inspection; duplicate canonical
  keys, malformed values, partial rows, and values outside approved policy fail
  rather than being silently discarded.
- Store side data through a closed kind enum and normalized kind-specific child
  rows. Each supported kind has an explicit field schema, units,
  canonicalization, and exact equality rule. Unknown kinds, unknown fields,
  missing required fields, duplicates, and parser-version ambiguity mean the
  stream cannot satisfy `preserve`; raw JSON, JSONB, and arbitrary key/value
  escape hatches are not permitted.
- Resolve source-to-output comparison only through the immutable
  `DesiredStreamBinding` accepted by ADR 509. Output stream indexes are
  observations, not identity. Candidate and final verification must compare
  complete normalized row sets in both directions so omissions and additions
  both fail.
- Add an injected `AttachmentPayloadExtractor` boundary. Production bootstrap
  selects its concrete implementation; planner, domain, and verifier logic
  receive the collaborator and cannot construct a native tool or read the
  environment directly.
- The extractor accepts a descriptor-bound aggregate input, immutable source
  stream binding, attested execution identity, attempt workspace, cancellation
  signal, and one immutable approved-limit snapshot. It returns only bounded
  evidence: payload byte count, SHA-256, extractor contract version, source
  binding, execution identity, and a bounded terminal reason.
- Extraction must use the ADR 501 supervised process envelope and the accepted
  descriptor/confinement boundaries when native tooling is required. It must
  enforce every approved count, byte, workspace, process-output, deadline, and
  cancellation limit before and during extraction, clean partial artifacts, and
  fail closed on missing or exceeded limits.
- This ADR intentionally chooses no numeric extractor limit. Every value must
  come from an explicitly approved policy contract and be snapshotted
  immutably. If any required limit is absent, held, or not approved, attachment
  `preserve` is unavailable; an implementer may not supply a convenient default.
- Source evidence is captured once for the admitted aggregate and bound into
  the immutable job snapshot. Candidate and final payloads are independently
  extracted through the same versioned contract and must match source byte
  count and SHA-256 exactly before replacement can proceed.
- Data-stream payload hashing remains outside this proposal. ADR 509's narrower
  data preservation claim remains unchanged.
- This proposal introduces no independent command-line or argument-byte limit.
  ADR 507's accepted row, key, value, and aggregate bounds remain authoritative
  for metadata materialization. Opaque retention generates only typed mapping
  and copy operations; any additional command-wide limit requires a separate
  Proposed ADR and explicit operator approval.

## Consequences

- Retained opaque state becomes stable across parser ordering and output stream
  reindexing while remaining relational and exactly comparable.
- Unsupported or newly observed side data fails closed instead of disappearing
  from verification or entering an untyped persistence escape hatch.
- Attachment payload identity becomes independently reproducible at source,
  candidate, and final boundaries under one injected, bounded contract.
- Production attachment preservation remains unavailable until all required
  numeric limits have explicit approval. This preserves architectural control
  but may defer that capability.
- Schema, extractor, fixtures, and comparison coverage become more substantial,
  and every new side-data kind requires an explicit contract extension.

## Implementation Boundary

- This ADR authorizes only the
  normalized metadata and side-data representation, immutable binding-based
  comparison, injected attachment payload extractor, approved-limit snapshot,
  exact digest evidence, cleanup, and focused validation described here.
- Any persistence implementation updates `crates/revaer-data/init.sql`
  under ADR 522 and use normalized relational tables plus stored procedures. It
  does not restore historical migrations or store raw probe documents.
- This ADR does not authorize attachment or data target selectors,
  replacement or transcoding, data payload hashing, arbitrary side-data fields,
  caller-authored bindings, invented numeric limits, weaker cancellation or
  confinement, raw FFmpeg arguments, or an independent command-size ceiling.
- ADR 507's accepted bounds and ADR 509's stream policy, precedence, and
  attachment-only payload guarantee remain authoritative and unchanged.
- Attachment preservation activation remains stopped until the operator
  separately approves every numeric policy still held.

## Validation

- This docs-only proposal changes no production behavior and adds no runtime
  test.
- Add complete source/candidate/final round trips for every
  supported metadata and side-data kind, including empty sets, ordering,
  duplicate keys, unknown kinds and fields, malformed units, omitted rows,
  added rows, parser-version drift, and output-index reordering.
- Prove direct stored-procedure calls cannot create partial, duplicate,
  unversioned, JSON, or binding-inconsistent snapshots.
- Use deterministic attachment fixtures with identical and changed payloads,
  equal-size replacements, multiple attachments, cancellation, timeout,
  process-output exhaustion, workspace exhaustion, partial-output cleanup,
  quarantine, rollback, and recovery.
- Prove every extractor limit is loaded from the exact immutable approved-policy
  snapshot. Missing, held, stale, mismatched, and exceeded values must fail
  without default substitution.
- Prove extractor doubles and production implementations consume descriptor-
  bound inputs and cannot reopen caller-supplied host paths or bypass the
  supervised process boundary.
- Any future implementation must pass focused database and real-media tests,
  `just ci`, `just ui-e2e`, strict Sonar analysis, and test-media cleanup.

## Follow-up

- Present every still-held numeric extraction and workspace policy in a separate
  Proposed ADR, or an explicit amendment, before enabling attachment preserve.
- Record the exact implementation and validation evidence in a
  separate task ADR without broadening this decision.

## Task Record

- Motivation:
  - Close the schema and bounded-extraction gaps in ADR 509 before retained
    opaque streams are implemented or claimed complete.
- Design notes:
  - The recommendation uses normalized relational state, immutable source
    bindings, and an injected extractor whose limits are policy inputs rather
    than implementation constants.
  - Architectural implementation is authorized, but no numeric limit or
    attachment-preservation activation is authorized by this decision.
- Test coverage summary:
  - This change is documentation-only; no runtime, schema, API, extractor, or
    media tests are added.
  - Documentation index, policy, instruction-drift, link, and diff checks are
    the only validation appropriate to this acceptance-only change.
- Observability updates:
  - No telemetry changes are made.
  - A future accepted implementation should expose bounded normalization,
    extraction, limit, cancellation, and digest-mismatch reason codes without
    payload bytes, metadata values, paths, hashes, or identities in metric
    labels.
- Status-doc validation:
  - Reviewed ADRs 449, 501, 507, 509, 519, 522, 526, and 527. This proposal
    changes no implementation or product-completion claim.
  - `README.md`, `MEDIA_TRANSCODING.md`, roadmap/status documents, and operator
    guides are unchanged; the ADR index and documentation summary expose the
    accepted decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior and needs no runtime
    rollback.
  - Once implemented, rollback must disable attachment
    preservation when exact normalized or payload evidence is unavailable; it
    must not silently narrow the retained-state comparison.
- Dependency rationale:
  - No dependency is added. A future implementation must first evaluate the
    existing process, descriptor, hashing, workspace, and persistence
    primitives; any new dependency requires separate written rationale.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No repository-policy drift or contradiction was found. The missing
    normalized-state and bounded-extractor decisions are documented without
    relaxing policy or inventing criteria.
