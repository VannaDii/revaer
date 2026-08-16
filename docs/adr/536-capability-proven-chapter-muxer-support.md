# Capability-proven chapter muxer support

- Status: Accepted
- Date: 2026-08-16
- Operator approval: Explicitly approved by the operator on 2026-08-16:
  "I approve the ADRs as they are now and I'm resuming your goal."

## Problem

- ADR 508 defines exact `preserve`, `strip`, and `replace` chapter timelines,
  but it does not define how Revaer proves that a selected output muxer can
  materialize each policy exactly.
- Listing a muxer in FFmpeg or successfully opening it does not prove that the
  packaged toolchain writes chapter boundaries, time bases, titles, metadata,
  and complete replacement state with the semantics required by ADR 508.
- A live capability lookup can change after planning. A queued job must not be
  compiled from one muxer capability state and executed against another.
- Command construction must fail before native execution when chapter support
  is absent, unproven, stale, or bound to a different executable closure.

## Options

1. **Infer chapter support from the muxer name or FFmpeg capability listing.**
   This is inexpensive, but it proves only registration and cannot establish
   exact chapter round trips or policy-specific behavior.
2. **Attempt chapter output for every job and rely on final verification.**
   This eventually catches drift, but it spends worker resources on a plan that
   was never known to be executable and makes readiness and queue admission
   optimistic.
3. **Use a closed, probe-backed capability record bound immutably to the job.**
   Exercise the production command builder and verifier against the exact
   packaged toolchain, record a closed result for each ADR 508 policy, and bind
   the resulting evidence identity through planning and execution.

## Recommendation

- Adopt option 3.
- Represent chapter muxer capability as normalized, versioned state keyed by
  the immutable ADR 519 executable-closure identity, canonical muxer identity,
  output container family, probe-contract version, and command-materialization
  version.
- The closed capability surface has one result for each ADR 508 policy:
  `preserve`, `strip`, and `replace`. Each result is exactly
  `proven_supported` or `proven_unsupported`. Missing rows, unknown values,
  partial probe evidence, and evidence for another version are not support.
- Produce support evidence with deterministic fixtures that contain nonempty,
  distinguishable chapter timelines. The probe must use the same planner,
  command materializer, supervised native process boundary, inspector, and
  exact verifier used by production execution rather than a handcrafted probe
  command.
- Probe evidence must identify the input fixture digest, requested policy,
  normalized expected timeline digest, observed timeline digest, selected
  muxer, executable closure, capability snapshot, command-materialization
  version, verifier version, terminal result, and bounded reason code. Evidence
  is valid only when the expected and observed exact states agree.
- Bind the immutable evidence identity into the capability snapshot used by the
  desired-target compiler and then into the queued job snapshot. Planning,
  claiming, resuming, and final verification must resolve the same identity;
  they must not reread mutable current capabilities.
- Materialize chapter arguments only when the job-bound result for the selected
  policy is `proven_supported`. Unsupported, missing, stale, mismatched, or
  malformed evidence fails planning or readiness before FFmpeg is launched.
  There is no unrestricted or best-effort fallback.
- Re-probing creates a new immutable evidence identity. It does not mutate the
  meaning of a queued job. Jobs that need the new result follow ADR 520's
  explicit re-plan contract.
- This proposal introduces no independent command-line or argument-byte limit.
  ADR 507's accepted row, key, value, and aggregate bounds remain authoritative
  for metadata materialization. Any additional command-wide limit requires a
  separate Proposed ADR and explicit operator approval.

## Consequences

- Queue admission and command construction can distinguish packaged muxer
  presence from exact chapter-policy support.
- Capability drift cannot silently alter a queued job because probe evidence is
  immutable and version-bound.
- Every supported muxer and policy combination requires deterministic probe
  fixtures and retained evidence, increasing validation cost.
- A newly packaged muxer or toolchain version is unavailable for chapter work
  until its exact probe evidence exists. This is intentionally fail-closed.
- The closed representation prevents arbitrary capability strings or
  parser-specific output from becoming an implicit public contract.

## Implementation Boundary

- This ADR authorizes only the
  closed chapter capability representation, production-path probe evidence,
  immutable capability and job binding, fail-closed readiness and command
  materialization, and focused validation described here.
- Any persistence implementation updates `crates/revaer-data/init.sql`
  under ADR 522 and use normalized relational state plus stored procedures. It
  does not restore historical migrations or use JSONB.
- This ADR does not reopen ADR 508 chapter semantics, ADR 501 process
  limits, ADR 507 metadata bounds, ADR 519 execution identity, or ADR 520
  re-planning. It does not authorize new containers, inferred support, mutable
  capability reads, arbitrary FFmpeg arguments, or a command-wide size limit.

## Validation

- This docs-only proposal changes no production behavior and adds no runtime
  test.
- Add a complete muxer-by-policy matrix covering supported,
  unsupported, missing, malformed, stale, and version-mismatched evidence.
- Prove probes use production planning, materialization, process supervision,
  inspection, and verification boundaries and reject handcrafted or
  incompletely attested evidence.
- Mutate chapter boundaries, time bases, titles, metadata, ordering, muxer
  identity, executable closure, capability identity, and materializer version;
  each mismatch must fail closed before replacement.
- Prove queued and resumed jobs retain their original evidence identity across
  re-probing and require explicit ADR 520 re-planning to adopt a new identity.
- Any future implementation must pass focused database and real-media tests,
  `just ci`, `just ui-e2e`, strict Sonar analysis, and test-media cleanup.

## Follow-up

- Record the exact implementation and validation evidence in a
  separate task ADR without broadening the decision.

## Task Record

- Motivation:
  - Close the architectural gap between ADR 508's exact chapter state and the
    evidence required to claim that a concrete muxer can produce it.
- Design notes:
  - The recommendation separates muxer registration from policy-specific exact
    capability and makes immutable probe evidence part of job identity.
  - Implementation is limited to this accepted decision and requires separate
    task records with exact validation evidence.
- Test coverage summary:
  - This change is documentation-only; no runtime, schema, API, probe, or media
    tests are added.
  - Documentation index, policy, instruction-drift, link, and diff checks are
    the only validation appropriate to this acceptance-only change.
- Observability updates:
  - No telemetry changes are made.
  - A future accepted implementation should expose bounded capability-evidence
    and policy-specific readiness reason codes without paths, chapter text,
    command lines, or high-cardinality identities in metric labels.
- Status-doc validation:
  - Reviewed ADRs 449, 501, 507, 508, 519, 520, and 522. This proposal changes no
    implementation or product-completion claim.
  - `README.md`, `MEDIA_TRANSCODING.md`, roadmap/status documents, and operator
    guides are unchanged; the ADR index and documentation summary expose the
    accepted decision.
- Risk & rollback plan:
  - Acceptance alone changes no production behavior and needs no runtime
    rollback.
  - Once implemented, rollback must fail chapter readiness and
    admission rather than infer support or bypass immutable evidence.
- Dependency rationale:
  - No dependency is added. A future implementation must first evaluate the
    existing planner, process, inspection, hashing, and persistence primitives;
    any new dependency requires separate written rationale.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No repository-policy drift or contradiction was found. The missing
    capability-proof decision is documented here without relaxing policy.
