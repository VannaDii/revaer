# Atomic source-identity-checked replacement

- Status: Proposed
- Date: 2026-08-16
- Operator approval: Pending

## Problem

- Accepted ADR 500 requires an immediate source identity check before the first
  replacement mutation and an exclusive cooperative aggregate lease through
  finalize or rollback.
- A separate check followed by an overwriting rename still leaves a final
  check-to-rename interval. A destination entry can change in that interval, and
  an ordinary rename can destroy or displace bytes that were never included in
  the checked source identity or ready backup.
- Pinning the parent directory prevents directory redirection but does not make a
  namespace entry compare-and-swap. Pinning the prior file descriptor likewise
  does not require the name being replaced to refer to that descriptor at the
  instant of mutation.
- Replacement must preserve both candidate and displaced bytes until the exact
  expected destination identity has been proven at the final commit boundary.
  Aggregate recovery must remain deterministic across crashes and retries.

## Options

1. **Keep immediate recheck followed by overwriting rename.** This is portable
   and composes with cooperative ownership, but it retains the destructive
   interval identified above.
2. **Require stronger cooperative locks only.** This narrows the race among
   correct writers, but a faulty writer or violated lease can still make an
   unchecked entry the rename target.
3. **Atomically exchange candidate and destination, then validate the preserved
   displaced object before commit.** The namespace operation never discards
   either object. A mismatch reverses the exchange under the same lease or
   quarantines both objects without acknowledging commit.
4. **Publish immutable versioned names and atomically update a pointer.** This
   offers clean compare-and-swap semantics but changes the source pathname and
   reader contract, which broadens the first-release product surface.
5. **Require a filesystem transaction or snapshot service.** This can provide a
   native conditional commit on selected storage but creates a platform and
   operational dependency not established for all supported deployments.

## Recommendation

- Adopt option 3 as the v1 replacement primitive. Keep option 5 available only
  as a later proven equivalent and do not adopt option 4 in this decision.
- Add an injected `IdentityCheckedReplacement` capability whose production
  implementation can atomically exchange two regular-file entries relative to
  one attested parent handle without following links or overwriting a third
  object. Media destructive readiness is unavailable when the platform or
  filesystem cannot prove this capability.
- Require the candidate and destination to be on the same mounted filesystem and
  under the accepted managed-root and attempt-workspace ownership model. Cross-
  device copy at commit time, unlink-then-rename, and check-then-overwrite
  fallbacks are forbidden.
- Before exchange, pin the current destination and candidate, validate regular-
  file type, link and mount constraints, candidate output identity, expected
  source aggregate, ready ADR 525 backup, current attempt and claim generation,
  and ADR 500 lease. Record this exact intent in the attempt-scoped replacement
  manifest and fsync it.
- Perform one atomic exchange. The former destination must then exist at the
  private candidate entry and remain open through a retained descriptor; no
  original bytes are unlinked by the operation.
- Validate the displaced object against the expected final source member using
  platform file identity, type, byte length, required times, and the exact
  content digest from the immutable source aggregate and ready backup manifest.
  Validate that the live destination names the exact candidate identity.
- Only after both identities verify and the parent namespace is fsynced may the
  replacement transaction cross its durable commit boundary and acknowledge the
  member in a generation-fenced stored procedure.
- On any mismatch, cancellation, capability loss, or fsync failure before that
  acknowledgement, atomically reverse the exchange while the lease remains held
  and verify both restored identities. If reversal cannot be proven, preserve
  both entries, mark the transaction uncertain, quarantine the aggregate from
  further destructive work, and require recovery. Never unlink either object to
  manufacture a clean state.
- For a multi-member source aggregate, exchange members in canonical manifest
  order under one aggregate lease. The replacement manifest records each
  prepared, exchanged, verified, reversed, and acknowledged member. Failure
  reverses previously exchanged members in reverse order; crash recovery derives
  action only from verified manifest, namespace, backup, and database evidence.
- The final database completion event occurs only after every aggregate member is
  identity-checked, the namespace and manifest are durable, and the displaced
  originals remain in the recovery-owned transaction. Cleanup occurs later under
  ADRs 513 and 525.
- ADR 500's sole-writer or cooperative-lease requirement remains binding. This
  proposal does not claim correctness on a root where arbitrary writers may
  mutate entries during the exchange and validation transaction.

## Consequences

- An unexpected destination is preserved instead of being destroyed by an
  unconditional rename, and no replacement is acknowledged against unchecked
  bytes.
- The live pathname can hold the candidate during post-exchange validation. The
  accepted aggregate lease is therefore a required visibility barrier, and
  readiness must reject deployments whose readers or writers cannot honor it.
- Atomic exchange is a stricter platform capability than ordinary rename. Some
  filesystems and deployment targets may remain dry-run-only.
- Aggregate replacement and recovery gain additional manifest states, fsyncs,
  reverse-order rollback, and fault cases.
- Backup, rollback, attempt fencing, and recovery contracts remain authoritative;
  this proposal narrows only the final filesystem mutation boundary.

## Implementation Boundary

- While this ADR remains Proposed, it authorizes no filesystem, FFI, schema,
  runtime, API, UI, workflow, or deployment implementation.
- If explicitly accepted, it would authorize only the typed atomic-exchange
  capability, expected/displaced/candidate identity proof, replacement-manifest
  states, reverse exchange, uncertain quarantine, readiness gating, bounded
  reason codes, and validation described here.
- It would not authorize an unconditional rename fallback, unlinking unexpected
  bytes, cross-device commit, pathname-only checks, a weaker ADR 500 lease,
  versioned public source names, pointer indirection, filesystem-specific
  activation without capability proof, or unrelated replacement policy.
- Persistence, if accepted, belongs only in the approved pre-v1 `init.sql`
  transition under ADR 522. Any unsafe or FFI implementation needs its own
  repository-compliant boundary and rationale.

## Validation

- Inject a committer that replaces, exchanges, deletes, recreates, truncates, or
  rewrites the destination immediately before the delegated atomic operation.
  The unexpected bytes must survive, commit must not be acknowledged, and the
  expected source or ready backup must remain recoverable.
- Test mutation before pinning, after pinning, during intent fsync, immediately
  before exchange, after exchange, during displaced identity hashing, during
  namespace fsync, before database acknowledgement, and during reversal.
- Crash at every manifest, exchange, validation, fsync, acknowledgement, rollback,
  and cleanup boundary. Recovery must classify the exact state without guessing,
  unlinking unknown entries, or publishing duplicate completion.
- Cover absent destinations, symlinks, hard links, directories, devices, FIFOs,
  mount changes, cross-device candidates, unsupported filesystems, stale claim
  generations, lease loss, cancellation, candidate substitution, backup mismatch,
  and all aggregate member orderings.
- Prove a platform without atomic exchange reports destructive media readiness
  unavailable and cannot reach an ordinary rename fallback through configuration,
  retry, recovery, or test-only wiring.
- After an approved implementation, run focused replacement and recovery fault
  tests, complete source-plus-sidecar fixtures, `just ci`, `just ui-e2e`, security
  scanning, release-image verification, and the strict Sonar gate.

## Follow-up

- Obtain explicit decision-specific operator approval before implementation.
- If approved, reconcile the transaction with ADRs 447, 448, 500, 512, 513, 517,
  522, 523, 525, and 533 without changing their ownership or retention rules.
- Prove the exact atomic primitive, filesystem support matrix, crash durability,
  and recovery behavior in the implementation task record before enabling
  destructive readiness on any platform.

## Task Record

- Motivation:
  - Record the destructive check-to-rename race found by independent review
    before the current replacement boundary is considered source-identity safe.
- Design notes:
  - Atomic exchange preserves candidate and displaced objects, allowing exact
    post-operation identity proof without first destroying unexpected bytes.
  - The recommendation composes with the accepted cooperative ownership model;
    it does not claim a portable kernel compare-and-swap that does not exist.
- Test coverage summary:
  - This documentation-only proposal adds no filesystem or media test and
    performs no replacement.
  - Proposal validation is limited to generated documentation indexes, policy,
    instruction drift, link checks, and diff hygiene.
- Observability updates:
  - No telemetry changes are made by this proposal.
  - A future implementation must expose bounded capability-unavailable,
    expected-identity-mismatch, candidate-mismatch, exchange, reversal, fsync,
    and uncertain-recovery reasons without paths, identities, digests, jobs, or
    attempts as metric labels.
- Status-doc validation:
  - Reviewed `MEDIA_TRANSCODING.md` and ADRs 447, 448, 500, 512, 513, 517,
    522, 523, 525, and 533. No product-status or readiness claim is changed by
    this proposal.
- Risk & rollback plan:
  - The proposal changes no production behavior. If a later accepted
    implementation regresses, disable destructive readiness and preserve both
    sides of every uncertain exchange plus its backup and manifest; never
    downgrade to ordinary rename.
- Dependency rationale:
  - No dependency is added by this proposal. A future platform or safe FFI
    wrapper must be justified against standard APIs, auditability, portability,
    maintenance, and transitive supply-chain cost.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/revaer-data.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`.
  - No policy drift, contradiction, or criteria relaxation was found.
