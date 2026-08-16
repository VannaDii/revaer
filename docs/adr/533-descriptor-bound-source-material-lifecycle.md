# Descriptor-bound source material lifecycle

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Accepted ADR 526 binds native inspection to exact retained source, sidecar,
  and companion handles. It does not decide what bytes later planning,
  transcoding, or verification may use.
- A worker can currently validate a fingerprint, release that point-in-time
  evidence, and later reopen a host pathname for execution. A concurrent rename,
  exchange, replacement, or in-place write can therefore make the native
  transcoder consume bytes other than those inspected and admitted.
- Retaining an open descriptor prevents pathname substitution but does not by
  itself prevent in-place mutation of the opened object. Pre- and post-operation
  metadata checks also cannot prove that bytes were not changed and restored
  while a native process was reading them.
- Planning must not pair inspection evidence from one source aggregate with an
  execution input from another aggregate. Verification must likewise evaluate
  the candidate produced from the same admitted material rather than reopening
  ambient source paths.
- The required invariant is that one exact aggregate of admitted bytes supplies
  inspection evidence, plan identity, transcoder input, and any source reads
  needed by output verification.

## Options

1. **Continue pathname reopen plus fingerprint checks.** This is inexpensive
   and portable, but retains swap-and-restore and in-place mutation windows.
2. **Keep source descriptors open for the complete attempt.** This prevents
   pathname re-resolution, but a writer can still modify the same objects in
   place and open handles cannot survive process restart as durable evidence.
3. **Require a cooperative source lease and retain source descriptors.** This
   composes with ADR 500 for cooperating writers, but the admitted aggregate is
   still not private, durable attempt material and a faulty writer can violate
   the lease.
4. **Create a private, content-verified source-material package and use only its
   retained read-only handles.** Copy or proven copy-on-write snapshot the exact
   ADR 526 aggregate into an attempt-owned namespace, seal it from further
   writes, bind planning to its canonical identity, and pass only retained
   package handles to inspection, transcoding, and verification.

## Recommendation

- Adopt option 4 while retaining ADR 500's cooperative source lease as the
  admission boundary.
- Introduce an injected, typed `MediaSourceMaterial` lifecycle. Production
  bootstrap owns the platform implementation; domain and planner code receive
  only the typed material identity, bounded inspection evidence, and approved
  read-only inputs.
- Enroll exactly the source, sidecar, and companion set accepted by ADR 526. This
  proposal extends the lifetime of that set; it does not add another member,
  format, protocol, discovery rule, or pathname fallback.
- Under the managed-root handle and the current aggregate lease, open each
  enrolled member once with ADR 526's beneath-root, no-follow rules. Materialize
  it into a create-new, attempt-scoped private package while computing its full
  content digest and bounded descriptor observations.
- A platform copy-on-write snapshot may replace a byte copy only when the
  implementation proves snapshot isolation from later writes, same-filesystem
  ownership, crash behavior, and equivalent content verification. A hard link,
  ordinary reflink assumption, bind mount, or read-only mode bit alone is not an
  acceptable proof.
- Compare the complete package manifest and digest with the immutable job source
  aggregate before admitting planning or execution. A mismatch fails closed and
  publishes no plan or candidate.
- Fsync every material member, its canonical manifest, and the package namespace.
  Close every writable handle before publishing the package as ready. Reopen the
  ready package descriptor-relatively, verify its manifest and member digests,
  and retain only read-only, close-on-exec handles.
- Derive a domain-separated source-material identity from the aggregate contract
  version, ordered member kinds and names, byte lengths, content digests, and
  manifest version. Planning evidence and the immutable plan snapshot carry this
  identity. Execution rejects any plan whose identity differs from the retained
  material.
- Inspection uses ADR 526 aliases backed by these handles. Transcoding receives
  the same closed input set through the typed supervisor request. Verification
  receives the candidate through a separate attempt-owned output handle and may
  read source material only through the retained package handles.
- Native arguments, logs, telemetry, and durable public records never contain the
  private package host pathname. The child sees only deterministic sandbox-local
  aliases approved by ADRs 526 and 527.
- After every native phase, verify member descriptor observations and package
  digests before accepting output. Any mutation, missing member, extra member,
  manifest mismatch, or identity mismatch is terminal for the attempt and blocks
  source replacement.
- A restart may resume only from an attempt package whose ownership tuple,
  canonical manifest, fsync evidence, complete member set, and digests all
  verify. Otherwise recovery quarantines the package and starts no native tool.
- Cleanup follows accepted ADR 513's attempt-scoped retention transaction and
  must not remove material needed by unresolved execution, verification,
  replacement, rollback, or recovery evidence.

## Consequences

- Inspection, planning, transcoding, and verification refer to one exact source
  aggregate instead of independently resolving a mutable host pathname.
- In-place mutation and swap-and-restore races against the host source cannot
  alter what a native child reads after the private package becomes ready.
- Large media incurs copy, hash, fsync, and temporary-capacity cost unless a
  platform can prove an equivalent isolated snapshot. Capacity admission and
  cleanup therefore become production readiness requirements.
- Attempt recovery gains durable, content-verifiable input evidence, but the
  source-material manifest and lifecycle add schema, filesystem, and supervisor
  complexity.
- The accepted ADR 526 input surface and ADR 527 confinement remain unchanged.
  This proposal changes temporal binding, not product capability.

## Implementation Boundary

- While this ADR remains Proposed, it authorizes no schema, runtime, filesystem,
  API, UI, workflow, or deployment implementation.
- If explicitly accepted, it would authorize only the typed source-material
  lifecycle, exact ADR 526 aggregate staging, canonical material identity,
  retained read-only handles across planning/transcoding/verification, fail-
  closed restart validation, bounded reason codes, and tests described here.
- It would not authorize a broader ADR 526 input set, remote inputs, new media or
  sidecar formats, arbitrary descriptor inheritance, ambient host paths,
  hard-link staging, unproven reflink semantics, source mutation, weaker ADR 527
  confinement, or destructive work without ADR 500's cooperative lease.
- Persistence, if accepted, belongs only in the approved pre-v1 `init.sql`
  transition under ADR 522. No historical migration is authorized.

## Validation

- Use deterministic race tests that exchange, replace, delete, rewrite, truncate,
  extend, and change-and-restore every source member before staging, during
  staging, after package publication, during inspection, during transcoding, and
  during verification. Work must either consume the verified package bytes or
  fail before replacement.
- Prove inspection evidence, plan snapshots, transcoder requests, verifier
  requests, checkpoints, and recovery records all carry one source-material
  identity and reject cross-attempt or cross-generation substitution.
- Test complete source and sidecar aggregates, including paired VobSub members,
  duplicate identities, non-Unicode names, maximum approved sizes, insufficient
  capacity, cancellation, partial copy, failed fsync, malformed manifests,
  unexpected entries, restart at every publication boundary, and cleanup races.
- Validate copy-on-write implementations with mutation-after-snapshot tests and
  disable readiness when isolation cannot be proven. Hard links and ordinary
  mutable reflinks must fail the capability test.
- Prove no writable package handle or unlisted descriptor reaches a child and no
  private host pathname reaches arguments, logs, metrics, API responses, or
  durable audit labels.
- After an approved implementation, run focused filesystem and process fault
  tests, the complete media fixture matrix, `just ci`, `just ui-e2e`, security
  scanning, release-image verification, and the strict Sonar gate.

## Follow-up

- Obtain explicit decision-specific operator approval before implementation.
- If approved, reconcile the implementation with ADRs 500, 501, 513, 516, 517,
  519, 522, 523, 526, and 527 in one outside-in contract and fault matrix.
- Measure representative copy, hash, fsync, storage, restart, and cleanup costs
  before proposing production capacity and retention values. Those exact values
  require their own operator approval when not already fixed by an accepted ADR.

## Task Record

- Motivation:
  - Record the source-byte substitution gap found by independent review before
    inspection-only descriptor binding is treated as end-to-end source safety.
- Design notes:
  - The recommendation makes private verified material, rather than a mutable
    host pathname, the input authority for every phase after admission.
  - It extends ADR 526 without changing its accepted member or format surface.
- Test coverage summary:
  - This documentation-only proposal adds no runtime test and runs no media
    conversion.
  - Proposal validation is limited to generated documentation indexes, policy,
    instruction drift, link checks, and diff hygiene.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation must expose bounded material-stage, identity-
    mismatch, mutation, capacity, resume, and cleanup reason codes without paths,
    names, digests, jobs, attempts, or principals as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 500, 501, 513, 516, 517, 519,
    522, 523, 526, and 527. No current capability or production-readiness claim
    is changed by this proposal.
- Risk & rollback plan:
  - The proposal changes no production behavior. If a later accepted
    implementation regresses, disable media readiness and preserve every
    unverifiable package for recovery; never fall back to pathname execution.
- Dependency rationale:
  - No dependency is added by this proposal. A future implementation must
    justify any snapshot, safe filesystem, or platform wrapper against standard
    APIs, auditability, portability, and transitive supply-chain cost.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift, contradiction, or criteria relaxation was found.
