# Fingerprint Helper Registration Deadline

- Status: Superseded
- Superseded by: [ADR 593](593-simple-checkpoint-recovery.md). The custom owner
  registration protocol is no longer required. Evidence below is historical,
  not a prerequisite for the simplified recovery workflow.
- Date: 2026-09-12
- Operator approval: 2026-09-15, "ADRs 590 and 589 are approved under your recommendations."
- Supersedes: None. Clarifies only ADR 588's `REGISTER_HASH` deadline field.
- Implementation status: Typed supervisor registration and exact wire/ACK codec
  implemented. Independent owner transport and enforcement remain incomplete;
  approval hold released.

## Approval Resolution

The operator approved the recommendation on 2026-09-15. The implementation hold
is released. Require exact wire/zero-rejection tests and real runtime deadline
enforcement evidence; correct encoding alone does not qualify enforcement.

## Requested Decision

**Approve carrying the fingerprint helper's existing, nonzero effective deadline
in its registration message to the supervisor?** I recommend yes.

The [approved wire contract](support/588-lifecycle-control.md#exact-bytes) defines
the broker registration deadline, but gives the fingerprint registration only
the same identity rules. It does not say whether its deadline field is used.
Choosing zero versus a real cutoff changes what the supervisor can enforce.

## Proposed Rule

`REGISTER_HASH` offset 24 carries the already-running effective fingerprint
cutoff as absolute Linux `CLOCK_MONOTONIC` nanoseconds; zero is invalid. The
caller supplies the existing bound with all earlier applicable limits already
composed. Registration and ACK cannot start, extend or reset a clock. The codec
does not choose durations or compose deadlines. All other record fields and the
[approved fingerprint limits](support/588-discovery-values.md) stay unchanged.

The alternative, zero/unused, omits that cutoff from the independent owner.
Reusing the broker startup/recovery clock would apply the wrong operation's
budget. Neither alternative is recommended. This clarification does not reopen
S2/LIFE-1, add a timeout, qualify containment or authorize a weaker gate.

## Task Record

- Motivation and design notes: remove one wire-field ambiguity before implementation; keep deadline authority with the existing admission boundary.
- Test coverage summary: exact opcode-3 known-answer bytes, zero-field rejection,
  ACK nonce/reference/sequence and unused-byte rejection pass on host and Linux.
  Six real-child supervisor cases pass on both, including a delayed registration
  that cannot restart the expired deadline. Independent owner enforcement,
  HELLO state and process-identity validation are not yet qualified.
- Observability updates: registration encoding/ACK errors retain existing typed
  I/O rejection and child-settlement handling, without logging wire contents.
- Status-doc validation: accepted status is unchanged; implementation is partial,
  not another pending architectural approval.
- Risk and rollback: a wrong cutoff would misstate authority. Remove the codec
  and typed callback change together to roll back this implementation; automatic
  discovery must remain closed until the full owner boundary is qualified.
- Dependency rationale: none added.
- Stale-policy check: root, Rust, DevOps, FFI and ADR 588 reviewed. No criterion is relaxed and no approval is inferred.
