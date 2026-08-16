# Canonical exact HDR10 color volume

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- The PR 137 replay introduced twelve authored HDR10 color-volume values and
  described them as exact, but historical ADR 401 contains no
  decision-specific operator approval.
- Replay ingress and physical validation accept decimal or rational text through
  binary floating-point parsing. Command construction later requires exact
  integer units for x265, and verification compares FFprobe values with an
  epsilon. A value can therefore pass durable admission but fail later, while
  an "exact" verification result can still mean approximate equality.
- HDR10 mastering-display and content-light metadata already have fixed wire
  units. Canonicalizing those units before persistence can give API, YAML,
  stored procedures, compilation, encoder arguments, and verification one exact
  representation.
- The color-volume object must remain all-or-none, normalized into scalar
  relational columns and immutable job snapshots, and valid only for an HDR10
  target with complete signaling and payload verification.

## Options

1. **Keep free-form numeric text and floating tolerance.** Preserve the replay
   representation and accept late x265 representability checks. This is the
   smallest change but does not provide an exact end-to-end contract.
2. **Compile to canonical fixed-point HDR10 units.** Accept bounded decimal or
   rational text at ingress, convert it with exact integer arithmetic, persist
   the twelve integer scalars, and compare the same units after inspection.
3. **Persist arbitrary exact rationals and support encoder-specific
   materializers.** This preserves values outside the current wire grid but
   expands persistence, capability, command, and verification semantics beyond
   the first supported encoder.

## Recommendation

- Adopt option 2.
- `hdr10_color_volume` is valid only when `hdr_format` is exactly `hdr10` after
  canonical normalization. All twelve fields must be present together; no
  partial object or defaulted field is permitted.
- Compile the values into these nonnegative integer units without binary
  floating-point arithmetic:
  - Red, green, blue, and white-point x and y use units of `1/50000`.
  - Mastering minimum and maximum luminance use units of `1/10000` candela per
    square metre.
  - MaxCLL and MaxFALL use integer candela-per-square-metre units.
- API and YAML input may use bounded decimal or rational strings, but the exact
  parser rejects any value that is malformed, nonfinite, negative where not
  allowed, overflows the persisted integer type, or is not exactly representable
  in its field's unit. API responses and YAML export emit one canonical form:
  fixed-denominator rational strings for chromaticity and luminance and decimal
  integer strings for MaxCLL and MaxFALL.
- Persist the twelve canonical integers as scalar desired-target columns and
  immutable job-snapshot columns. JSON, floating columns, and command-form text
  are not application state.
- Physical validation is exact and fail-closed: each chromaticity coordinate is
  positive, each x/y pair is inside the chromaticity domain, the primary
  triangle is nondegenerate, the white point lies inside that triangle,
  mastering minimum luminance is nonnegative, mastering maximum is greater than
  minimum, MaxCLL and MaxFALL are positive, and MaxFALL does not exceed MaxCLL.
- An HDR10 target requires BT.2020 primaries, SMPTE ST 2084 transfer,
  BT.2020 nonconstant-luminance matrix signaling, a supported 10-bit output
  pixel format, mastering-display side data, and content-light side data.
- Exact color-volume materialization is supported only through `libx265` for
  the first release. The command renders deterministic `master-display` and
  `max-cll` values directly from the canonical integers. A hardware-only or
  otherwise incapable snapshot fails during capability-derived planning.
- Candidate and final FFprobe side-data values are parsed exactly into the same
  integer units and compared for equality. Epsilon comparison and rounded
  acceptance are not part of the contract.

## Consequences

- A value accepted by API, YAML, or a stored procedure is guaranteed to be
  representable by the first supported materializer; late unit-conversion
  failures are eliminated.
- Exactness has one testable meaning across persistence, command construction,
  and verification.
- Physically meaningful values outside the fixed HDR10 unit grid are rejected.
  Supporting them later requires a separately approved representation and
  materializer rather than implicit rounding.
- Persisted field types and public canonical output differ from the replay's
  free-form text. Because the product has not shipped, the contract can be
  corrected before compatibility obligations exist.
- Jobs that request exact color volume cannot fall back to a hardware encoder
  that lacks identical mastering-display and content-light controls.

## Implementation Boundary

- This accepted ADR authorizes only the twelve-field all-or-none object, exact
  fixed-point units, canonical ingress and export forms, physical invariants,
  normalized scalar persistence and snapshots, required HDR10 signaling,
  libx265 materialization, capability failure, and exact candidate and final
  verification described above.
- Before v1, accepted persistence changes must modify
  `crates/revaer-data/init.sql`; historical migrations 0180 and 0189 are
  provenance only and must not be restored.
- API models, OpenAPI, YAML import/export, exact domain parsing, stored
  procedures, desired-target compilation, capability projection, command
  construction, inspection normalization, and verification may change only as
  required for this contract.
- Approved ADR 442 remains authoritative for capability-derived planning. This
  ADR supplies exact HDR10 requirements to that boundary and does not
  change capability snapshot binding or worker snapshot ownership.
- This ADR does not authorize HDR10+, Dolby Vision, HLG, tone mapping,
  arbitrary mastering metadata, approximate verification, silent rounding,
  another encoder, hardware fallback, or general video constraints covered by
  ADR 511.
- Schema, runtime, API, YAML, generated-contract, and UI changes must remain
  limited to the accepted contract above.

## Validation

- Decision validation to date is documentation-only; this record changes no HDR target,
  encoder, planner, or verification behavior.
- During implementation, add exact parser tests for decimal and rational canonical
  equivalents, every field unit, lower and upper boundaries, overflow,
  nonrepresentable fractions, negative and nonfinite input, and canonical export
  round-trips.
- Add all-or-none, non-HDR10, chromaticity-domain, triangle, white-point,
  luminance, MaxCLL, and MaxFALL tests through API, YAML, core compilation, and
  direct stored-procedure calls.
- Prove desired-target and immutable job snapshots preserve every integer
  exactly and reject partial or inconsistent rows.
- Add capability and command tests for deterministic libx265 arguments,
  unavailable libx265, hardware-only capability sets, malformed snapshots, and
  no fallback encoder.
- Add real HDR10 fixtures whose candidate and final FFprobe payloads match,
  differ by one canonical unit, omit either payload, use incorrect color
  signaling or bit depth, fail verification, quarantine, and roll back.
- An accepted implementation is not complete until `just ci` and
  `just ui-e2e` pass in addition to focused tests.

## Provenance

- Evidence was inspected in `/private/tmp/revaer-descendant-replay` at
  `replay/pr-194` without modifying that worktree.
- The relevant replay commit is PR 137 `4580a5eb`.
- Historical ADR 401 and historical migrations 0180 and 0189 describe the
  replay implementation. Its `Accepted` label is not operator approval.
- Replay command construction scales chromaticities by 50,000, luminances by
  10,000, and content-light values by one before constructing x265 arguments,
  but only after earlier floating validation. Replay verification compares
  parsed values with an epsilon of `1e-6`; that mismatch motivates this ADR.
- Approved ADRs 442, 446, 449, and 451 continue to govern capability planning,
  immutable snapshots, capability binding, and policy compilation. This
  ADR does not reopen them.

## Follow-up

- Introduce the exact domain representation first and make every
  boundary consume it; do not retain a floating or free-form compatibility path.
- Reconcile `MEDIA_TRANSCODING.md`, OpenAPI, YAML examples, generated clients,
  UI fields, and verification documentation with the accepted representation.

## Task Record

- Motivation:
  - Replace the unapproved and internally approximate PR 137 representation with
    an explicit operator choice about what exact HDR10 color volume means.
- Design notes:
  - The recommendation uses the HDR10 wire grid as canonical domain state rather
    than persisting encoder command syntax or binary floating values.
  - Exact parsing occurs before durable admission, and the same integer values
    drive materialization and verification.
- Test coverage summary:
  - The ADR-only change added no runtime, API, schema, planner, command, or HDR fixture
    tests.
  - `git diff --check`, `just instruction-drift`, and
    `just docs-link-check` passed for this ADR-only change.
  - The exactness and full validation listed above remains mandatory after any
    implementation.
- Observability updates:
  - No telemetry changes are made by this ADR.
  - A future implementation should persist bounded field and phase reason codes
    without logging authored color-volume values, command lines, source paths,
    or job and capability identifiers as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, historical ADR 401 and PR 137 evidence, and
    approved planning and snapshot ADRs. This ADR is accepted but does not claim
    the fixed representation is implemented.
  - `README.md`, roadmap/status documents, and operator guides are unchanged;
    the ADR index and documentation summary expose this accepted but unimplemented
    decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must not reinterpret integer snapshots as
    free-form text. Incompatible queued jobs must drain or fail explicitly, and
    exact verification must remain fail-closed.
- Dependency rationale:
  - No new dependency is required. A bounded exact decimal/rational parser and
    checked integer conversion can use existing Rust primitives.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as prospective
    implementation constraints.
  - No policy drift was found. Historical migration references remain
    provenance-only under the pre-v1 `init.sql` rule.
