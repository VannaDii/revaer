# Profile readiness and source-bound planning

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Context

- The source-stack ADR 355 (`355-media-compatibility-target-admission.md`) made
  a source-independent profile capability check a hard gate for non-dry-run
  direct and discovery admission. Source-stack ADR 356
  (`356-media-profile-readiness-api.md`) exposed that check as a boolean
  profile-readiness API and shared it with background queueing.
- [ADR 442](442-capability-derived-media-planning.md) subsequently established
  that authoritative planning requires the exact persisted capability snapshot,
  inspected source graph, compiled desired graph, and effective video policy.
  Demux, decode, copy, stream binding, and some subtitle decisions cannot be
  proven from a profile alone.
- A source-independent evaluator can still identify fixed conditions that no
  possible source can cure, such as a missing referenced target or unavailable
  muxer or encoder that a mandatory output requires. It cannot certify that one
  concrete source is executable.
- Four decisions must therefore remain distinct:
  - **Advisory readiness:** a pre-source operator assessment of profile and
    capability facts.
  - **Universal impossibility:** a source-independent proof that every source
    would fail a fixed requirement.
  - **Queue admission:** whether a valid, authorized command may create durable
    work for later inspection and planning.
  - **Authoritative preflight:** the source-bound, snapshot-bound decision that
    may authorize command construction and execution.

## Options

1. **Keep the profile boolean authoritative.** Reject queue admission whenever
   profile readiness is false and allow execution whenever it is true. This is
   simple but either rejects source-contingent work prematurely or overstates
   what a source-independent check proves.
2. **Remove source-independent readiness.** Queue every structurally valid
   request and defer every capability decision to worker preflight. This avoids
   false certainty but hides fixed impossibilities from operators and spends
   queue capacity on work known to be impossible.
3. **Use a layered contract.** Preserve source-independent readiness as an
   advisory assessment, let proven universal impossibility affect admission,
   and reserve execution authority for ADR 442 preflight with a concrete source.

## Recommendation

- Adopt option 3.
- The profile-readiness API reports an `assessment` with exactly two semantic
  outcomes:
  - `advisory_ready`: no source-independent blocker is known; this is not an
    execution guarantee.
  - `universally_impossible`: one or more stable reason codes prove that the
    fixed profile, target, and selected capability snapshot cannot produce the
    required output for any source.
- The response also states `source_preflight_required: true`. Source-dependent
  `copy` viability, input demux and decode support, stream binding, source
  metadata, sidecar ownership, and final candidate selection are never converted
  into profile-only readiness claims.
- Universal-impossibility reasons are limited to fixed facts. Examples include
  a missing referenced catalog version or absent output muxer or encoder for a
  mandatory transcode. An unresolved source-dependent question is advisory
  uncertainty, not universal impossibility.
- Queue admission remains a separate command decision:
  - Invalid, unauthorized, unmanaged-root, or malformed requests are rejected
    under their existing contracts.
  - A non-dry-run command with a current `universally_impossible` assessment is
    rejected before queueing with the same stable reason codes.
  - Source-dependent uncertainty does not reject an otherwise valid command; it
    is resolved by authoritative preflight.
  - A valid dry-run command may be admitted despite universal impossibility so
    it can persist deterministic planning evidence or a preflight-failure audit
    without permitting mutation.
- The advisory endpoint and queue gate share one source-independent evaluator.
  That evaluator projects only fixed capability and catalog facts; it does not
  duplicate or approximate the full planner.
- ADR 442 preflight remains authoritative. The worker inspects the concrete
  source and uses the exact capability snapshot, compiled desired graph, and
  effective policy bound by the accepted execution architecture. Only successful
  preflight can reach command construction, whose independent fail-closed checks
  remain intact.

## Consequences

- Operators receive useful early diagnostics without being told that a profile
  alone proves execution readiness.
- Known impossible non-dry-run work is rejected before consuming queue and
  worker capacity, while valid source-contingent work can reach the only stage
  capable of deciding it correctly.
- Dry-run remains an evidence-producing path and cannot mutate source media.
- Existing clients using a boolean `ready` field must move to the explicit
  assessment contract before release.
- Reason-code taxonomy, API documentation, admission tests, and worker preflight
  tests must agree on which facts are fixed and which require a source.

## Implementation Boundary

- This proposal authorizes no implementation while its status is `Proposed`.
- Acceptance would authorize only the advisory assessment vocabulary, universal
  impossibility rule, queue-admission behavior, and authoritative-preflight
  boundary described above.
- It would not relax or supersede ADR 442, capability snapshot binding, managed
  root checks, fingerprint admission, dry-run isolation, command-builder
  validation, or worker ownership.
- It would not authorize speculative capabilities, a fallback
  `all_supported` planner, source-independent approval of `copy`, caller-selected
  capability evidence, or execution based on the readiness endpoint alone.
- It would not authorize a schema, runtime, API, or generated-contract change
  until this ADR receives decision-specific operator approval.

## Follow-up

- Obtain decision-specific operator approval before changing the readiness DTO,
  queue admission, worker preflight, schema, OpenAPI, or UI.
- After approval, prove every fixed blocker and source-contingent category with
  table-driven evaluator tests, then prove direct, discovery, watcher, schedule,
  dry-run, and non-dry-run admission all use the same classification.
- Add integration tests proving a source-dependent advisory result can be
  admitted but still fails closed in authoritative preflight when the concrete
  source lacks required demux or decode support.

## Task Record

- Motivation:
  - Resolve the PR 108-120 replay conflict between ADRs 355/356 and approved ADR
    442 before restoring profile readiness and target admission behavior.
- Design notes:
  - The contract names epistemic limits directly: advisory assessment reports
    fixed facts, universal impossibility is a proof, admission controls durable
    work, and preflight controls execution.
  - Stable reason codes are shared, but the source-independent evaluator and
    source-bound planner remain separate responsibilities.
- Test coverage summary:
  - This change is proposal-only and adds no runtime, API, or schema behavior.
  - `just policy`, `just instruction-drift`, `just docs-build`,
    `just docs-link-check`, and `git diff --check` passed.
  - The evaluator and integration matrix in Follow-up is required before any
    accepted implementation is complete.
- Observability updates:
  - No telemetry changes are made by this proposal. A future implementation
    should preserve bounded readiness and admission outcome metrics using stable
    outcome codes, without profile, source, or snapshot identifiers as labels.
- Status-doc validation:
  - Reviewed ADRs 355, 356, 442, 449, and `MEDIA_TRANSCODING.md`. This proposal
    does not change the product requirement for capability-aware, source-bound
    preflight and does not claim the layered contract is implemented.
  - `README.md`, roadmap/status documents, and operator guides were not changed;
    none currently claims this proposed contract is implemented.
- Risk & rollback plan:
  - Proposal-only publication changes no production behavior and can be rolled
    back by reverting this ADR and its index entries.
  - After implementation, rollback must keep ADR 442's authoritative preflight;
    the advisory API can be removed without restoring profile-only execution
    authority.
- Dependency rationale:
  - No dependency is proposed. Existing normalized catalog, capability, planner,
    and API primitives are sufficient.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and
    `.github/instructions/revaer-data.instructions.md` as the prospective
    implementation constraints.
  - No instruction drift was found. ADRs 355 and 356 contain an architectural
    assumption superseded in scope by ADR 442; this proposal makes the boundary
    explicit without changing repository policy.
