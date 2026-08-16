# Exact chapter timeline state

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- The PR 122, 123, and 126 replay introduced exact chapter preservation,
  stripping, and authored replacement, but the historical ADRs do not contain
  decision-specific operator approval.
- Chapters are container state rather than ordinary stream rows. Their contract
  must agree across API and YAML input, normalized relational persistence,
  immutable job snapshots, desired-graph compilation, temporary FFmetadata
  materialization, and candidate and final inspection.
- A chapter merge or edit model needs identity for existing chapters and rules
  for insertion, deletion, reordering, metadata conflicts, and time-base
  conversion. None of those rules is currently an approved product contract.
- The timeline and its authored metadata must be bounded before durable work is
  admitted, and temporary chapter artifacts must remain inside the managed
  workspace and be removed on every terminal path.

## Options

1. **Preserve or strip only.** Retain the complete source timeline or remove it,
   while deferring authored chapters.
2. **Preserve, strip, or exact full replacement.** Compile one complete expected
   timeline and verify both output stages against it.
3. **Merge or edit the source timeline.** Identify source chapters and apply
   inserts, updates, deletes, and reordering. This is expressive but requires a
   new identity and conflict model.

## Recommendation

- Adopt option 2.
- `container_chapter_policy` has exactly three normalized values:
  `preserve`, `strip`, and `replace`. Missing API or YAML input defaults to
  `preserve`; persisted desired-target and job-snapshot state is explicit.
- `preserve` forbids authored chapter rows and compiles the normalized complete
  source timeline as expected state, including an empty timeline.
- `strip` forbids authored chapter rows and compiles an empty expected timeline.
- `replace` requires at least one authored chapter and treats the authored rows
  as the complete timeline. It never merges with source chapters.
- Each chapter uses integer `start_millis` and `end_millis`. Start is
  nonnegative, end is strictly greater than start, canonical ordering is
  `(start_millis, end_millis)`, overlap is forbidden, and gaps are allowed.
- Replacement is bounded to 1,024 chapters. Each chapter may contain at most 64
  normalized metadata rows. Metadata keys and values use the 128-byte key and
  4,096-byte value bounds, and all chapter metadata together is bounded to
  65,536 bytes. Blank or repeated canonical keys within one chapter are invalid.
- Preserve maps the source chapter timeline, strip disables chapter mapping,
  and replace writes a managed FFmetadata artifact beginning with
  `;FFMETADATA1` and emitting each chapter with `TIMEBASE=1/1000` before mapping
  that dedicated input into the output.
- Replacement fails before execution when the selected muxer cannot materialize
  the compiled timeline. The temporary artifact is created without replacement
  inside the admitted workspace and deleted on success, failure, cancellation,
  quarantine, or recovery cleanup.
- Candidate and final verification compare the complete normalized timeline,
  including ranges, order, and per-chapter metadata. Added, removed, shifted,
  overlapping, or metadata-mutated chapters are mismatches.

## Consequences

- Operators receive deterministic preservation, erasure, and complete authored
  replacement without needing source-chapter identity or merge conflict rules.
- Millisecond canonicalization aligns persistence, FFmetadata materialization,
  and verification, but cannot represent authored sub-millisecond boundaries.
- A muxer that rewrites chapter boundaries or metadata outside the canonical
  representation fails verification rather than receiving an implicit
  tolerance.
- Replacement requires a bounded managed text artifact and one additional
  FFmpeg input. Workspace admission, cleanup, and cancellation must account for
  that artifact.
- Clients that need partial chapter edits must construct and submit the complete
  replacement timeline or wait for a separately approved merge contract.

## Implementation Boundary

- This accepted ADR authorizes only the policy values, defaults, complete-state
  semantics, timeline and metadata bounds, normalized desired-target and
  immutable snapshot rows, managed FFmetadata materialization, muxer rejection,
  cleanup, and exact two-stage verification described above.
- API requests, OpenAPI models, YAML import/export, pure compilation, stored
  procedures, workspace planning, execution steps, and verification may change
  only as needed to carry this exact contract.
- Before v1, accepted persistence changes must modify
  `crates/revaer-data/init.sql`; historical migrations 0172 and 0174 are
  provenance only and must not be restored.
- This ADR does not authorize chapter merge or patch operations, stable
  source-chapter identifiers, sub-millisecond authoring, chapters as ordinary
  stream targets, arbitrary external artifact paths, or relaxed verification.
- It does not alter the approved capability, immutable snapshot, sidecar
  discovery, or effective-policy compilation decisions.
- Schema, runtime, API, YAML, generated-contract, and UI changes must remain
  limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only; this record changes no chapter or
  workspace behavior.
- During implementation, add policy and shape tests covering empty preservation, strip,
  nonempty replacement, normalization, sorting, allowed gaps, overlap, invalid
  ranges, duplicate metadata keys, and every chapter and metadata resource bound.
- Exercise the same failures through API, YAML, core compilation, immutable
  snapshot reconstruction, and direct stored-procedure calls.
- Add command and workspace tests for exact input numbering, FFmetadata escaping,
  line-break rejection, unsupported muxers, create-new behavior, cancellation,
  and artifact cleanup on every terminal path.
- Add real-media candidate and final fixtures for empty and nonempty source
  timelines, strip, authored replacement, muxer boundary changes, metadata
  changes, verification failure, quarantine, and rollback.
- An accepted implementation is not complete until `just ci` and
  `just ui-e2e` pass in addition to focused tests.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- Relevant replay commits are PR 122 `1d1fd262`, PR 123 `a0c940d2`, and PR 126
  `5026f59f`.
- Historical ADRs 367, 372, and 374 and historical migrations 0172 and 0174
  describe the implemented source-stack behavior. Their `Accepted` labels are
  not treated as operator approval.
- Approved ADRs 442, 446, and 451 continue to govern planning, worker snapshot
  access, and pure compilation. This ADR does not reopen them.

## Follow-up

- Implement the contract outside-in and keep chapter state,
  artifact ownership, cleanup, and verification in one reviewed change.
- Reconcile `MEDIA_TRANSCODING.md`, OpenAPI, YAML examples, generated clients,
  and operator UI labels with the accepted semantics.

## Task Record

- Motivation:
  - Convert the unapproved chapter behavior from PRs 122, 123, and 126 into an
    explicit operator choice before replay or correction.
- Design notes:
  - Chapters are modeled as one complete declarative timeline. Replacement is
    intentionally nonempty and distinct from stripping.
  - Canonical millisecond ranges and normalized metadata are compiled once for
    materialization and verification.
- Test coverage summary:
  - The ADR-only change added no runtime, API, schema, workspace, or UI tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this ADR-only change.
  - The focused and full validation listed above remains mandatory after any
    implementation.
- Observability updates:
  - No telemetry changes are made by this ADR.
  - A future implementation should persist stable bounded mismatch and cleanup
    outcomes without logging chapter titles, metadata values, source paths, or
    job identifiers as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, the historical replay ADRs and commits, and
    approved planning and snapshot records. This ADR is accepted but does not
    claim exact chapter replacement is implemented.
  - `README.md`, roadmap/status documents, and operator guides are unchanged;
    the ADR index and documentation summary expose this accepted but unimplemented
    decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must preserve immutable queued-job meaning
    and clean every managed FFmetadata artifact; queued replacement jobs must
    drain or fail explicitly rather than be reinterpreted.
- Dependency rationale:
  - No new dependency is required. Existing deterministic text rendering, path,
    workspace, and normalization primitives are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as prospective
    implementation constraints.
  - No policy drift was found. Historical migration references remain
    provenance-only under the current pre-v1 `init.sql` rule.
