# Exact video technical constraints

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- The PR 138 replay introduced authored video dimensions, pixel format, bit
  depth, average frame rate, color range, and safety limits, but historical ADR
  421 contains no decision-specific operator approval.
- Video fields do not all have the same semantics. Resolution, pixel format,
  bit depth, frame rate, profile, level, and color signaling describe exact
  output state, while bitrate is naturally an upper bound. Treating every field
  as equality or every field as advisory would misstate operator intent.
- Declaring a constraint must not force a lossy encode when source inspection
  already satisfies the final contract. Conversely, missing or mismatched
  inspection evidence must not be treated as compliance.
- Resource bounds for dimensions, frame rate, upscaling, bitrate, workspace
  admission, and process duration must be enforced consistently before command
  execution and again during candidate and final verification.

## Options

1. **Advisory preferences.** Use the values to rank plans but allow a final
   stream that does not match. This avoids failures but cannot support exact
   target compliance.
2. **Always encode when any field is declared.** Materialize every declared
   constraint unconditionally. This is simple but performs avoidable lossy work
   and can reduce quality without changing the final state.
3. **Exact final state with mismatch-only encoding.** Define equality fields and
   ceiling fields precisely, compare them with source inspection, and encode
   only when codec or declared technical state differs.

## Recommendation

- Adopt option 3.
- Desired width and height are an all-or-none pair and must equal final inspected
  width and height. Pixel format, bit depth, codec-aware profile and level,
  color range, color primaries, color transfer, and color space are exact after
  canonical normalization.
- Average frame rate is an exact positive rational. Ingress accepts an integer
  or `numerator/denominator`, rejects zero or negative terms, bounds each term to
  1,000,000 and the value to 240 FPS, reduces by greatest common divisor before
  persistence, and compares reduced rationals exactly without floating-point
  tolerance.
- `video_bitrate_bps` is an inclusive maximum, not an equality target. Final
  average bitrate must be present and no greater than the configured maximum;
  final peak bitrate, when inspection reports it, must also be no greater than
  that maximum. A missing average measurement is a mismatch.
- A source video stays on the copy or remux path when its codec and every
  declared constraint match. A codec mismatch or any declared-constraint
  mismatch requires an encode candidate. Missing evidence is a mismatch, not a
  reason to assume compliance.
- Capability-derived planning under ADR 442 must exclude an encode candidate
  unless the exact immutable snapshot proves the required decoder, encoder,
  codec support, muxer, and demuxer. Command construction retains an independent
  fail-closed capability check.
- Ingress and compilation enforce these resource limits:
  - Width and height are each at most 16,384 pixels.
  - Frame area is at most 134,217,728 pixels.
  - Maximum bitrate is at most 1,000,000,000 bits per second.
  - Upscaling is at most four times either source axis and sixteen times source
    frame area.
  - Candidate bytes cannot exceed the amount admitted by workspace preflight.
  - Media execution retains ADR 501's approved duration-derived transcode
    deadline and 24-hour maximum; this proposal does not define another process
    deadline.
- Persist declared constraints as normalized scalar desired-target rows and
  immutable job-snapshot rows. API responses and YAML export emit canonical
  values, including reduced frame-rate rationals.
- Candidate and final verification apply the same exact equality, inclusive
  ceiling, and missing-evidence rules. A mismatch prevents source replacement.

## Consequences

- Matching source streams avoid unnecessary lossy encoding even when a target
  explicitly documents their technical state.
- Each field has one operator-visible meaning: exact equality except for the
  explicitly named maximum bitrate.
- Files whose inspector cannot provide a declared value fail or require encode;
  they are never certified by absence of evidence.
- A peak bitrate above the ceiling fails even when average bitrate is below it.
  If no peak measurement exists, the present average is the available bounded
  verification evidence rather than an invented peak.
- Strict limits reject extreme but syntactically valid targets and upscales
  before they consume unbounded CPU, memory, disk, or process time.
- API/OpenAPI, YAML, normalized persistence and snapshots, compilation,
  planning, command construction, verification, docs, and UI labels must agree
  on equality versus ceiling semantics after approval.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only the exact video fields, inclusive maximum
  bitrate, canonical frame-rate representation, mismatch-only encode rule,
  listed resource limits, normalized desired-target and immutable snapshot
  state, capability projection, and exact candidate and final verification.
- Before v1, accepted persistence changes must modify
  `crates/revaer-data/init.sql`; historical migration 0190 is provenance only
  and must not be restored.
- API models, OpenAPI, YAML import/export, pure target compilation, stored
  procedures, source-bound planning, command materialization, workspace
  admission, inspection, and verification may change only as required to carry
  this contract.
- ADR 510 independently owns exact HDR10 color-volume representation. This ADR
  may consume an approved HDR constraint as one exact video mismatch but does
  not authorize or redefine its values.
- ADR 501 remains authoritative for native-process deadlines, output bounds,
  cancellation, and process-group cleanup.
- Acceptance would not authorize audio constraints, encoder quality or preset
  heuristics, automatic target relaxation, arbitrary scaling strategies, frame
  interpolation, tone mapping, approximate frame-rate comparison, capability
  fallback, or increased process and workspace bounds.
- No schema, runtime, API, YAML, generated-contract, or UI behavior may change
  until this ADR receives explicit decision-specific operator approval.

## Validation

- Proposal validation is documentation-only; this record changes no video,
  planner, encoder, workspace, or verification behavior.
- After approval, add boundary tests for paired dimensions, per-axis and area
  limits, bitrate limits, frame-rate parsing and reduction, term and value
  limits, normalized profile/level and color values, and direct stored-procedure
  attempts to bypass every invariant.
- Add a table proving each exact match remains copy/remux and each codec,
  equality-field, maximum-bitrate, missing-evidence, and peak-bitrate mismatch
  requires encode or fails closed when capabilities are absent.
- Add source-bound upscale tests at and beyond each axis and area limit,
  candidate-size admission tests, process deadline and cancellation tests, and
  independent command-builder capability failures.
- Prove API, OpenAPI, YAML, desired-target persistence, and immutable snapshots
  round-trip canonical reduced frame rates and all scalar constraints exactly.
- Add real candidate and final fixtures for exact match, each mismatch family,
  inclusive bitrate edges, absent bitrate measurements, peak excess, output
  drift, verification failure, quarantine, and rollback.
- An accepted implementation is not complete until `just ci` and
  `just ui-e2e` pass in addition to focused tests.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- The relevant replay commit is PR 138 `db8a9650`.
- Historical ADR 421 and historical migration 0190 describe the replay
  implementation. Its `Accepted` label is not operator approval.
- PR 137 `4580a5eb` supplies the separate HDR10 color-volume evidence addressed
  by ADR 510. PR 139 `910ed071` materializes audio rate constraints, has no
  migration, and is outside this video decision.
- Approved ADRs 442, 446, 449, and 451 continue to govern capability planning,
  immutable snapshots, capability binding, and effective-policy compilation.
  This proposal does not reopen them.

## Follow-up

- Obtain explicit decision-specific operator approval before implementation or
  before changing this ADR to `Accepted`.
- If approved, implement one typed constraint contract and make ingress,
  persistence, planning, command construction, and both verification stages
  consume it without field-specific reinterpretation.
- Reconcile `MEDIA_TRANSCODING.md`, OpenAPI, YAML examples, generated clients,
  UI labels, and operator documentation with exact and ceiling semantics.

## Task Record

- Motivation:
  - Convert the unapproved PR 138 video behavior into an explicit operator
    choice about exact output state, bitrate ceilings, and encode necessity.
- Design notes:
  - The recommendation distinguishes equality constraints from the one maximum
    constraint and uses exact rational frame-rate state.
  - Source-bound inspection determines whether encoding is necessary; the
    declaration alone does not authorize lossy work.
- Test coverage summary:
  - This proposal adds no runtime, API, schema, planner, command, workspace, or
    media-fixture tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this proposal-only change.
  - The focused and full validation listed above remains mandatory after any
    approval and implementation.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation should persist bounded mismatch, capability, and
    resource-rejection reason codes without placing source, target, job,
    capability, resolution, or bitrate values in metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, historical ADR 421 and PR 138 evidence, and
    approved planning and snapshot ADRs. This proposal does not claim the
    recommended constraints are approved or implemented.
  - `README.md`, roadmap/status documents, and operator guides are unchanged;
    only the ADR index and documentation summary expose this pending proposal.
- Risk & rollback plan:
  - This proposal changes no production behavior and can be rolled back by
    reverting this ADR and its two index entries.
  - After implementation, rollback must not reinterpret immutable constraints
    or silently relax queued jobs. Incompatible jobs must drain or fail
    explicitly, and safety limits must remain fail-closed.
- Dependency rationale:
  - No dependency is proposed. Checked integer arithmetic, greatest-common-
    divisor reduction, normalization, and existing process/workspace primitives
    are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as prospective
    implementation constraints.
  - No policy drift was found. Historical migration 0190 remains
    provenance-only under the current pre-v1 `init.sql` rule.
