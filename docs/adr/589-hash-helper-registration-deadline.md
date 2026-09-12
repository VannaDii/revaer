# Fingerprint Helper Registration Deadline

- Status: Proposed
- Date: 2026-09-12
- Operator approval: Pending
- Supersedes: None. Clarifies only ADR 588's `REGISTER_HASH` deadline field.
- Implementation status: Not started; affected codec validation remains paused.

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
- Test coverage summary: source comparison only; after approval, add exact opcode-3 known-answer bytes and reject zero without claiming runtime deadline enforcement.
- Observability updates: none; existing failure classifications remain unchanged.
- Status-doc validation: approval register and navigation identify this one pending clarification; all previously accepted choices stay accepted.
- Risk and rollback: accepting a wrong cutoff would misstate authority. No runtime change exists to roll back; reject this proposal before implementation if the field should be unused.
- Dependency rationale: none added.
- Stale-policy check: root, Rust, DevOps, FFI and ADR 588 reviewed. No criterion is relaxed and no approval is inferred.
