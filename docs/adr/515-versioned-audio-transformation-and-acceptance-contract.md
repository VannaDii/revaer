# Versioned audio transformation and acceptance contract

> Current approval: The operator accepted ADR 588's exact applicable choices
> on 2026-09-11 at reviewed commit `9575c077`. The [resolution](588-first-release-decision-package.md#approval-resolution)
> releases only those named design holds. Earlier pending/candidate wording
> below retains its historical context; implementation and qualification are
> not certified, and all other conditions remain binding.

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- `MEDIA_TRANSCODING.md` requires loudness normalization, dynamic-range
  compression, speech-focused audio policy, bitrate decisions, quality guards,
  and verification before replacement. It does not select a loudness standard,
  target values, compression algorithm, compression parameters, filter order,
  acceptance tolerances, or audio bitrate meaning.
- Replayed runtime behavior gives `dialog-normalized` and `speech` exact FFmpeg
  recipes and accepts fixed measurement windows. Audio bitrate is emitted as a
  nominal total bitrate and accepted within a fixed symmetric tolerance.
- The current argument and mocked-verification tests prove that those literals
  are enforced. They do not prove that the values are correct for consumer
  dialog, broadcast, audiobook, music, or another operator intent, and no real
  conversion fixture measures the resulting loudness or compression behavior.
- Execution and verification own separate constants. When both current policies
  are selected, compression follows normalization, so later processing can alter
  the output that the normalization step targeted.
- Historical ADRs 326, 328, and 329 describe this replayed behavior but contain
  no decision-specific operator approval. Their `Accepted` labels are provenance,
  not authorization under ADR 445.
- The exact transformation and acceptance contract crosses normalized
  persistence, API and YAML semantics, policy compilation, process execution,
  verification, audit evidence, and operator-visible behavior. It therefore
  requires an explicit architectural decision before production activation.

## Baseline And Held Existing Choices

- The baseline requires the ability to express loudness normalization and
  dynamic-range behavior. It does not require preserving the replayed hidden
  recipe.
- The current `dialog-normalized` recipe, its target and acceptance values, the
  current `speech` compressor and parameters, their order when combined, and the
  symmetric audio bitrate tolerance are unapproved existing choices.
- Until exact preset values and evidence receive separate decision-specific
  approval after measurement against real fixtures, `dialog-normalized`, `speech`,
  and any audio bitrate constraint whose nominal, maximum, or per-channel meaning
  is unresolved are held from production execution.
- Reconstruction must fail closed before native command construction when a held
  value is encountered. It must not silently preserve, reinterpret, ignore, or
  substitute another audio policy.
- This ADR changes no runtime behavior by itself. Its acceptance authorizes the
  fail-closed hold and versioned contract structure, while production behavior
  remains held pending separate exact-value approval.
- Preserve-only audio behavior, codec and channel selection, and exact sample-rate
  verification remain outside this hold when they do not depend on a held
  loudness, dynamic-range, or bitrate interpretation.

## Options

1. **Ratify the replayed recipe unchanged.** Keep the current semantic names,
   filter literals, processing order, measurement windows, and bitrate tolerance.
   This minimizes implementation work but adopts values without representative
   conversion or listening evidence and leaves execution and verification prone
   to drift.
2. **Compile versioned semantic presets into one transformation and acceptance
   contract.** Store a stable preset identity and version, compile it into typed
   process and verification settings, and derive every command and acceptance
   check from that one immutable contract. Exact preset values remain pending
   operator selection after measured validation.
3. **Expose fully parameterized numeric audio policy.** Persist loudness targets,
   peak ceilings, range targets, compressor controls, and acceptance tolerances as
   operator-authored normalized fields. This maximizes flexibility but greatly
   expands schema, UI, semantic validation, incompatible-combination handling,
   and long-term support obligations.
4. **Ship preserve-only audio behavior.** Reject all loudness, compression, and
   unresolved bitrate requests for v1. This is the safest interim behavior but
   does not deliver the complete first-release audio policy required by the
   baseline specification.

## Recommendation

- Adopt option 2 as the accepted architecture. Production activation follows only
  after the operator selects or revises the unresolved choices.
- Represent each approved preset with a stable semantic key, an explicit contract
  version, and a typed immutable compiled value. Do not persist or accept raw
  FFmpeg filter strings as operator policy.
- Compile target rows and immutable job snapshots through the approved
  `EffectiveMediaPolicy` boundary. The same compiled audio contract must supply
  transformation arguments, required pre-analysis, final measurements,
  acceptance comparisons, and bounded audit descriptions.
- For a preset that combines dynamic-range processing and loudness normalization,
  apply the selected dynamic-range transform before the final normalization step
  so no later transform silently invalidates the normalization target.
- Use measured two-pass normalization for presets that promise measured output.
  Persist bounded first-pass measurements and the preset version as attempt
  evidence so retries and resumable checkpoints cannot change the recipe.
- Reject missing, non-finite, malformed, or tool-incompatible measurements. A
  required measurement or native-process failure is a verification failure and
  must prevent replacement.
- Use the approved ADR 501 process supervisor for analysis and transformation.
  Native-process deadlines, output bounds, cancellation, process-group cleanup,
  and reaping are not redefined here.
- Keep bitrate semantics in the same compiled contract, but do not select total,
  maximum, average, per-channel, container-reported, or packet-derived behavior
  without operator approval. Execution and verification must use the same
  approved meaning.

## Unresolved Operator Choices

- Whether the first preset targets consumer dialog, broadcast, audiobook, music,
  or multiple separately versioned intents.
- The loudness measurement standard and exact integrated loudness target.
- The true-peak target and the distinct final acceptance ceiling, if any.
- The loudness-range target or ceiling and its acceptance tolerance.
- The dynamic-range algorithm, parameters, makeup behavior, and whether it may be
  combined with each loudness preset.
- The allowed difference between requested and measured output, including any
  codec- or container-specific tolerance.
- Whether audio bitrate means a nominal encoder target, inclusive maximum,
  average, per-channel value, or another explicitly measured quantity.
- The evidence threshold for approving a preset: objective fixture measurements,
  operator listening review, or both.
- No replayed numeric value, tolerance, coefficient, or filter literal is adopted
  by this ADR merely because it already exists or currently passes tests.

## Consequences

- Audio transformation and verification become one versioned product contract
  instead of unrelated command and acceptance literals.
- Jobs, retries, and audits can identify exactly which preset and measurements
  governed the output.
- Two-pass processing adds analysis time and checkpoint evidence, but removes the
  ambiguity of claiming measured normalization from an unmeasured one-pass recipe.
- Existing targets that carry held policy values cannot become production-active
  until they are mapped to an approved preset version or revised to preserve-only
  behavior.
- A preset version is immutable. Any semantic, algorithm, parameter, filter-order,
  or acceptance change creates a new version and explicit re-planning evidence.
- Operators cannot supply arbitrary native filter expressions. New behavior must
  enter through reviewed typed policy and a new or revised ADR.

## Implementation Boundary

- This accepted ADR authorizes only a versioned typed audio preset contract,
  shared execution and verification compilation, required measured analysis,
  fail-closed parsing, immutable snapshot and attempt evidence, and fail-closed
  treatment of any unselected bitrate meaning.
- This acceptance does not authorize production activation. The operator must
  approve the exact values and evidence as part of this ADR or an explicitly
  linked follow-up decision before held behavior is enabled.
- Any accepted persistence work belongs in the v0 `init.sql`; historical
  migrations 0157 and 0159 remain provenance and must not be replayed as the
  implementation mechanism.
- The affected implementation surface is limited to normalized audio target and
  snapshot contracts, API/YAML/UI representations, pure policy compilation,
  media-runtime command construction, the injected ADR 501 supervisor,
  verification, audit evidence, and their focused tests.
- This ADR does not authorize arbitrary filter strings, new codecs, a new
  process runner, weaker source or replacement verification, source-level lint or
  Sonar suppression, or unrelated target and stream-policy changes.
- Approved ADRs 445, 446, 448, 451, 483, 484, and 501 remain binding.
- Structural schema, runtime, API, generated-contract, UI, workflow, and
  specification changes may implement only the accepted typed and fail-closed
  boundary. Held exact preset and bitrate behavior may not become production-
  active until separate decision-specific approval is recorded.

## Validation

- Decision validation to date is documentation-only. This ADR adds no
  implementation or behavioral test.

| Scenario | Required result after exact-value approval and implementation |
| --- | --- |
| Preset compilation | One immutable typed contract supplies both process arguments and acceptance checks; no duplicated semantic constants remain. |
| Dialog, music, audiobook, silence, clipped, low-range, and high-range fixtures | Real packaged-tool conversion records finite pre- and post-transform measurements and meets only the operator-approved preset. |
| Combined dynamic-range and loudness policy | The approved order is deterministic, final normalization is not invalidated by a later transform, and the final output is measured. |
| Missing, malformed, non-finite, truncated, or incompatible analyzer output | Verification fails closed and source replacement does not begin. |
| AAC, Opus, AC3, multichannel, and stereo outputs | The approved contract is either satisfied with codec-appropriate evidence or rejected with a stable reason; no permissive fallback occurs. |
| Bitrate boundary cases | Execution and packet- or stream-derived verification implement the one approved bitrate meaning, including exact boundary behavior. |
| Retry or resume after analysis or transformation | The preset version and completed valid measurement checkpoint are reused only when all approved input identities still match. |
| Candidate or final mismatch | Durable verification evidence records bounded expected and actual values and replacement remains blocked. |

- Before production activation, add pure compiler and boundary tests, real
  conversion fixtures, analyzer parser tests including all non-finite values,
  process cancellation and timeout tests, API/YAML/UI validation, database
  procedure tests, and replacement prevention tests.
- An accepted implementation is not complete until the full fixture acceptance
  matrix, strict Sonar coverage and result guardrails, `just ci`, and
  `just ui-e2e` pass.

## Provenance

- Evidence was inspected read-only in
  `/private/tmp/revaer-approved-decisions-stack` on branch
  `work/media3-approved-decisions-stack`.
- The baseline requirements are in `MEDIA_TRANSCODING.md`, including policy-owned
  verification strictness, audio loudness normalization, dynamic-range
  compression, audiobook intent, quality guards, and verification-before-
  replacement gates.
- Replayed behavior is in
  `crates/revaer-media-runtime/src/execute/mod.rs` and
  `crates/revaer-app/src/media_job_runtime/verification/audio.rs`, with process
  measurement in `audio_measurement.rs` and tests in `media_job_runtime/tests.rs`.
- Relevant replay commits include `18dcf49f`, `5e135259`, `87dd911e`, and
  `75eb2b9f`. Their implementation and signed authorship are not
  decision-specific approval.
- Historical ADRs 326, 328, 329, and 339 describe the replayed audio constraint,
  filter, measurement, and process behavior. They lack current operator approval
  evidence and do not authorize the exact semantics.
- Approved ADRs 445, 446, 448, 451, 483, 484, and 501 constrain governance,
  snapshots, resumability, policy compilation, crate ownership, operator
  surfaces, and native process execution preserved by this ADR.

## Follow-up

- Obtain separate explicit operator selection or revision of the unresolved
  choices before production activation.
- Build a representative measurement and listening packet for each proposed
  preset without enabling production behavior or pushing an unapproved prototype.
- Define the typed contract and normalized persistence/API surface in fail-closed
  form first. After exact values are approved, implement command generation,
  measured processing, verification, operator controls, and audit evidence
  against that contract.
- Reconcile `MEDIA_TRANSCODING.md`, generated API contracts, user documentation,
  and status claims only in the implementation change that activates approved
  exact values.

## Task Record

- Motivation:
  - Convert the exact audio transformation and acceptance values exposed by the
    architecture audit into an explicit operator decision instead of silently
    inheriting replayed constants.
- Design notes:
  - This ADR adopts semantic versioning and one compiled contract while
    deliberately preserving every unresolved product and measurement choice.
  - The hold distinguishes required baseline capabilities from unapproved exact
    behavior and prevents passing tests from being treated as architecture
    approval.
- Test coverage summary:
  - No runtime, schema, API, UI, process, or media-conversion test was added or
    run by the ADR-only change.
  - ADR checks are `git diff --check`, `just instruction-drift`, and
    `just docs-link-check`.
  - The validation matrix and full repository gates remain mandatory after any
    exact-value approval and implementation.
- Observability updates:
  - No telemetry changes are made by this ADR.
  - A future implementation must expose bounded preset-version, analysis-stage,
    and acceptance-result dimensions without paths, job ids, stream ids, or raw
    measurements as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md`, historical ADRs 326, 328, 329, and 339, and
    approved ADRs 445, 446, 448, 451, 483, 484, and 501.
  - `README.md`, runtime status, API contracts, operator guides, workflows, and
    the specification remain unchanged. Only the ADR index and documentation
    summary expose this accepted architecture and unresolved exact-value hold.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior. Any reversal requires a
    superseding ADR.
  - After implementation, rollback must preserve readable preset and measurement
    evidence and must fail closed for jobs whose contract an older runtime cannot
    interpret exactly.
- Dependency rationale:
  - No new dependency is required. Existing typed policy, FFmpeg-compatible
    tooling, process supervision, normalized persistence, and verification
    boundaries are sufficient for the recommended design.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/revaer-ui.instructions.md`, and
    `.github/instructions/devops.instructions.md` as prospective implementation
    constraints.
  - No policy drift was found. Exact behavior remains held, no criteria are
    relaxed, and production activation of held behavior remains blocked pending
    separate decision-specific approval and measured preset selection.
