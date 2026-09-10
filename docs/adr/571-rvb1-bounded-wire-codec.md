# RVB1 bounded wire codec

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - ADR 558's accepted resolution approves B1-B3 and shared G1. Annex A fixes
    the RVB1 representation. This task implements its pure data codec without
    selecting held environment values or changing any accepted semantics.
  - The parent integrates the independent ADR 573 vectors and ADR 572 package
    observations. Coordination of these subtasks is not operator approval.
- Decision:
  - Implement checked encoding, fixed-header validation and borrowed decoding
    under `revaer-media-runtime::process::broker`, with no process operations.
    Use the existing standard library, `thiserror`, and test dependencies.
  - Preserve explicit numeric mappings, big-endian integers, exact field and
    aggregate bounds, raw Unix argument bytes, canonical UTF-8 secondary
    evidence, status consistency, and caller-supplied frame expectations.
- Consequences:
  - Independent literals catch a shared encoder/decoder layout mistake.
    Invalid bytes remain closed error data, never native execution authority.
  - This is a prerequisite, not broker lifecycle or media-feature completion.
- Follow-up:
  - Implement the accepted identity authorities, lifecycle, preemptible I/O,
    deadline rounding/enforcement, cancellation, cleanup and containment with
    their own real failure-path proof. E1 and ADR 569 remain separately held.
  - Retain full CI/UI, strict published Sonar coverage, package, security-diff,
    GitHub review and exact-restacked-revision verification as open acceptance
    work. No production request or release is authorized by a codec result.

## Scope And Invariants

`FrameHeader::parse` accepts fixed 12-byte storage plus an `ExpectedFrame`.
It rejects magic, reserved bytes, kinds, invalid caller expectations and the
kind-specific length before returning a bounded payload size. The caller must
use this boundary before allocating or reading a payload. `decode_frame` then
requires exactly one complete frame, including no trailing bytes. Borrowed byte
and string fields refer to the caller's frame storage. Only bounded arrays of
argument/evidence references allocate, after count checks against both protocol
bounds and the remaining payload. Evidence additionally receives an allocation-free
full item-length, content and canonicality preflight. Encoding validates every field and exact
payload length before one fallible reservation.

The wire models are public data, not sealed execution capabilities. Their
constructors deliberately do not certify a file descriptor, execution closure,
root-owned immutable package, category, clock, nonce source, PID/PGID operating
system identity, queue state or broker lifetime. No production caller uses the
codec in this change. A future caller MUST obtain the accepted sealed
`VerifiedNativeExecutable` authority before constructing a native request and
MUST independently enforce the lifecycle. An expected response supplied twice
will decode twice; duplicate-terminal rejection belongs to the owning state
machine, not these stateless functions. Neither lane is opened here.

Per-field limits remain as in Annex A: request payload 69..1,048,576 bytes,
program/argument strings at most 4,096 bytes, at most 4,096 arguments, combined
string bytes at most 1,000,000; each stream/evidence field at most 16,777,216;
response payload at most 50,335,744. Stream limits may be zero. Handshakes have
exact lengths 20 and 68. Evidence count is bounded by the bytes available for
nonempty length-prefixed items, not an invented smaller count. UTF-8 evidence
is strictly byte-sorted and unique. It is neither normalized nor silently
truncated by this layer. Native stdout/stderr remain arbitrary bytes, including
NUL and non-UTF-8, while program and arguments reject NUL.

The codec only validates the transmitted positive whole-millisecond duration
against the existing control/job ceilings. It does not perform rounding or
grant execution time. ID equality is checked against the caller's nonzero
expected ID, without incrementing, wrapping, retrying or resetting identifiers.
Hello acknowledgement compares all supplied identity fields and requires a
positive Linux PID equal to the transmitted process group ID. It does not
replace the parent's required OS group lookup or executable verification.

## Task Record

- Motivation:
  - Finish an independently approved broker prerequisite while held database
    parity and package values cannot be activated or invented.
- Design notes:
  - Split wire models, validation, decoding, encoding and tests by responsibility.
    No new serialization framework, unsafe code, fallback spawn, environment
    access, runtime SQL or source-level suppression. The focused recipe is
    `just test-media-broker-codec`; full workspace gates also exercise the tests.
- Test coverage summary:
  - Focused tests include 25 independent literal frames for
    both lanes and all statuses, every short prefix/trailer of representative
    frames, exact header/program/argument/stream/evidence limits, one-over
    rejection, reserved/unknown values, nonce/PID/digest/ID mismatch, malformed
    counts/lengths/UTF-8/order, raw binary output and checked size-sum overflow.
    Evidence preflight rejects legal maximum counts paired with over-bound,
    truncated, empty or malformed items before reference storage reservation.
    Response decoding also checks smaller and zero original-request limits.
    Deterministic arbitrary bytes and mutations behind valid headers may only
    decode if re-encoding reproduces their complete canonical bytes.
  - Allocation exhaustion is handled by `try_reserve_exact` but not injected in
    these tests. Unrepresentable u32-to-usize conversions are not reachable on
    supported 64-bit targets. Header acceptance of the response ceiling is not
    a claim that every permitted status can populate all three fields at once.
  - Full integrated gate results are recorded separately below; fixture and
    package-worker results do not establish an integrated pass.
- Observability updates:
  - No runtime telemetry changes. `Frame` debug output contains only the kind;
    errors contain static closed messages with no raw payload, path, arguments,
    digest, nonce, output or evidence. Other payload models do not derive Debug.
    The future owning lifecycle must classify/log an error once and retain
    bounded secondary evidence without exposing sensitive wire content.
- Status-doc validation:
  - Reviewed README development/status guidance, the specification's full
    first-release scope and validation gates, ADR 564's evidence ledger and
    ADRs 554/558/559. This task does not change product scope or claim an
    available operator flow. ADR 573 corrects coordination wording so a parent
    assignment is not represented as an operator's decision-specific approval.
- Risk & rollback plan:
  - Parsing bugs can reject valid frames or admit invalid data to a future
    caller; keep independent literals and adversarial bounds in required tests.
    Revert this codec, its recipe/instruction and task record together if
    defective. No deployed state, database authority or process lifecycle is
    changed. The unapproved final-init prototype remains local and inert.
- Dependency rationale:
  - None added. Standard-library byte parsing and fallible vectors suffice.
    Existing `thiserror` supplies static errors; existing `anyhow` and
    `serde_json` are used only to read and assert test fixtures.
- Stale-policy check:
  - Reviewed AGENTS.md, Rust/devops/Sonar scoped instructions, the ADR template,
    canonical media/quality recipes and accepted broker/approval contracts.
    The devops instruction now names the focused codec gate and its evidence
    limits. No accepted constant, workflow, required check, lint, analyzer,
    coverage scope or other criterion was relaxed. Historical Annex A draft
    language is subordinate to ADR 558's actual approval resolution.

## Integrated Validation

Pending the final executable revision's `just ci` and `just ui-e2e` runs.
No current GitHub, full Sonar, package or overall completion result is claimed.
