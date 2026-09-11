# S2 Lifecycle Control Proposal

> Current approval: The operator accepted ADR 588's exact applicable choices
> on 2026-09-11 at reviewed commit `9575c077`. The [resolution](../588-first-release-decision-package.md#approval-resolution)
> releases only those named design holds. Earlier pending/candidate wording
> below retains its historical context; implementation and qualification are
> not certified, and all other conditions remain binding.

Status: NEW ARCHITECTURAL DELTA, UNAPPROVED AND UNIMPLEMENTED. This is an exact
candidate for ADR 588, not an extension of RVB1 and not a third broker lane.
No new daemon, socket, upload, dependency, or production setting was introduced.

## Ownership And Bounds

- One same-binary non-Tokio PID1 owner creates exactly one application child.
  Branch to owner mode before constructing the application's Tokio runtime.
  The owner does no media-root, database, network, stdout, tracing or formatter
  I/O. Before drain it may validate private-namespace identity at registration.
  During the deadline path it uses only bounded pipe/control, clock, signal and
  wait operations. No proc/cgroup/counter/file reads or formatting enter that
  path. Do not move application work into this owner.
- Exactly two inherited anonymous unidirectional nonblocking pipes, one in each
  direction. Only the owner and application inherit the relevant ends; all
  native/helper execs close them. No environment-selected endpoint or pathname.
- Request Linux pipe capacity 4,096 bytes before child launch; verify actual
  capacity is >=4,096 and <=65,536 bytes per pipe (larger Linux page sizes can
  round this up). Reject a larger/unverifiable capacity before admitting media.
  Kernel pipe payload storage is therefore <=131,072 bytes for both pipes.
- Records are exactly 64 bytes; assert `_PC_PIPE_BUF >= 64` before use. One
  write per record; short write is a terminal protocol error. Reads assemble
  into one 64-byte partial buffer per direction. EOF with a partial record fails.
- At most 64 outstanding unacknowledged records per direction, 4,096 payload
  bytes each direction, including the active write. No hidden spill or dynamic
  retry queue. ACK records are not themselves ACKed. The record whose ACK is
  pending remains charged. Stop/fatal is a separate one-way monotonic latch.
- A 1,024-slot fixed owner registry, 128 bytes per slot, 131,072 bytes total,
  includes the application identity and retained descendant identity/wait state.
  Slot reuse requires settlement of the former identity, not a recycled PID.
  Completed records are folded into fixed counters once no reference remains;
  they are not an unbounded process-lifetime event list.
- Process count is not thread count: every Linux task remains charged to P1024.
  Registry entries do not grant an additional 1,024 tasks. Admission must also
  reserve all thread/tool growth of a request, not just fork one leader.
- Process at most 16 records or wait results in one batch; read current time
  before each action, checking due enforcement before lower-priority work.
  Prevalidate and retain identity while admission is open. After drain, refuse
  new registrations and use retained identities/ownership records. No proc,
  cgroup, counter, log formatting or file I/O in the deadline loop. Full census
  is deferred until affirmative wait settlement, and still counts against the
  final observation allowance. The corrective Perl case tests this minimal
  pattern only, not the Rust allocator, live codec or normal-phase identity I/O.

## Exact Bytes

All integers are unsigned little-endian. UUID/nonce bytes are opaque, not text.
No trailing bytes, variable-length payloads, strings, paths, logs, or native
output are accepted. Reserved bytes must be zero.

| Offset | Width | Meaning |
| --- | ---: | --- |
| 0 | 1 | Version, exactly 1 |
| 1 | 1 | Closed opcode below |
| 2 | 2 | Closed reason; zero unless FATAL/DRAIN |
| 4 | 4 | Sender sequence, starts at 1; strictly increases; wrap is terminal |
| 8 | 16 | Owner-generated fresh incarnation nonce |
| 24 | 8 | Absolute Linux CLOCK_MONOTONIC deadline in nanoseconds; zero only when inactive/unused |
| 32 | 4 | Namespace PID; zero when unused |
| 36 | 4 | Namespace PGID; zero when unused |
| 40 | 8 | `/proc/PID/stat` start ticks; zero when unused |
| 48 | 4 | Referenced sender sequence for ACK, SETTLED_HINT or request update; zero otherwise |
| 52 | 8 | Existing RVB1 u64 request identifier (prospective until its first frame byte); zero when unused |
| 60 | 4 | Closed opcode-specific detail below; zero when unused |

| Code | Opcode / direction | Exact effect |
| --- | --- | --- |
| 1 | HELLO, owner to app | First owner record, sequence 1, new 128-bit nonce; all payload fields zero. App must ACK before media admission. Entropy/setup failure admits no mutator. |
| 2 | REGISTER_NATIVE, app to owner | Register the BROKER lifetime identity, PID=PGID, start ticks; detail=1 control or2 job, request ID zero. No native work until ACK. Deadline is the already-running startup/recovery deadline. It does not create a separate native-tool group. |
| 3 | REGISTER_HASH, app to owner | Same identity rules for an ADR535 read-only helper; not a broker or native-tool bypass. |
| 4 | ACK, either direction | Reference one valid outstanding peer sequence; all other payload fields zero. No ACK loop. ACK means receipt/registration only, never task settlement. |
| 5 | SETTLED_HINT, app to owner | Reference the registration/current BEGIN_REQUEST sequence and identity/request ID; zero deadline/detail. Advisory only; neither ACK nor this hint releases occupancy. |
| 6 | DRAIN, either direction | First drain fixes origin; deadline carries an already-earlier applicable cutoff, or zero for no additional cutoff. Identity/reference fields zero. ACK cannot postpone drain. |
| 7 | FATAL, app to owner | Latch unclean plus first reason, initiate drain/containment; identity/reference/deadline zero. |
| 8 | APP_QUIESCENT, app to owner | After all application producers, workers, pools/listeners and logger have settled; payload zero. This is a claim to verify, not durable death evidence. App subsequently exits; owner waits it and observes the remaining unit. |
| 9 | BEGIN_REQUEST, app to owner | Exact registered broker identity, reference its registration sequence, prospective RVB1 request ID, nonzero absolute request deadline. Only after opcode11 verified the preceding request. ACK before first RVB1 frame byte; recheck original deadline before that byte. No new lane or RVB1 ID-assignment rule. |
| 10 | REVOKE_REQUEST, app to owner | Exact current request identity/reference, nonzero cancellation/deadline cutoff. May only shorten authority, never renew a request. Owner applies its role-specific grace clipped by all earlier deadlines. |
| 11 | VERIFIED_SETTLEMENT, owner to app | Exact identity and reference of the BEGIN_REQUEST or registration being settled, same request ID or zero for lifetime. Detail=1 request,2 broker lifetime,3 hash lifetime. Issued only under the proof rules below. Only this message releases corresponding protocol occupancy; ordinary ACK does not. |
| 12 | SETTLEMENT_REPORT, app to owner | Exact identity/reference/request ID, deadline zero, closed proof detail below. Produced only by the injected ownership/manager boundary after real settlement, not by arbitrary task completion. Owner checks the active ledger and required proof before opcode11. |
| 13 | MUTATORS_CLOSED, app to owner | Irreversible whole-incarnation application-mutator closure and completed joins/receipts, per RECOVERY.md. All payload fields zero. This is not DB/pool/logger closure and is not by itself a durable handoff. |
| 14 | MUTATORS_SEALED, owner to app | Reference opcode13's sequence; all other payload fields zero. Owner has latched irreversible admission close and validated all registered broker/helper lifetimes settled; permits only RECOVERY.md's bounded final handoff transaction before DB close. No work reopening even if no record is committed. |

Reasons: 0 ordinary signal/request, 1 application failure, 2 control overflow,
3 malformed/EOF control, 4 ownership mismatch, 5 timing miss, 6 log failure,
7 lease authority lost, 8 resource admission invariant, 9 cleanup unknown.
Other reason/opcode/version values fail closed. The first substantive reason
stays primary. Secondary reason presence is a fixed 10-bit set and occurrence
counts use saturating u64 counters with an explicit saturation bit.

Nonce generation and typed HELLO binding happen before any mutator. After HELLO,
app and owner reject wrong nonces, stale/duplicate/out-of-order sequences, unknown
references, impossible direction/state transitions and nonzero unused fields.
REGISTER cannot name PID1, the application as a native child, a foreign UID,
an unrelated namespace member or a stale start identity. Validate parentage and
owned group identity before ACK. The finite count does not replace this check.
No synchronous manager spawn starts after drain, and pending registrations do
not receive admission after drain. Registration failure is containment failure,
not permission to fall back to direct spawn. All waits consume ADR558's existing
startup/recovery and request clocks. There is no additional handshake timeout.

## Stop Latch And Failure

The owner installs signal handling before child launch. TERM and INT set the
stop latch; the first observed stop/fatal event fixes t0. Repetition coalesces and
does not reset any clock. Owner-to-app TERM conveys stop even when a pipe is
full. The owner sends TERM to registered native groups concurrently with drain.

NEW internal SIGUSR1 semantics: the application uses its retained parent
identity to signal a fatal-control latch if its nonblocking FATAL/DRAIN record
cannot be sent. The owner never relies on that record reaching a full pipe.
SIGUSR1 always means unclean stop, never a request to reload or restart. A failed
signal itself latches failure in the application, closes all admission and
begins its bounded stop; owner pipe EOF/child failure also fails closed.
No retrying write loop, logging call or arbitrary formatter is permitted here.
Pipe-full on ordinary traffic is also a fatal control failure, not message loss.

Owner pipe loss in the application closes admission and revokes root authority.
Owner death does not magically transfer its records; RECOVERY.md applies.
If the owner itself cannot progress, external package teardown is the backstop.
The owner emits no shutdown-critical log; return status and bounded typed
control evidence are separate from the owned application logger.

Proposed owner exit mapping: 0 only for fully observed clean ordinary settlement;
70 for forced/unclean but proved complete absence; 71 for deadline/absence unknown;
72 for control/ownership invariant failure. Invariant failure takes precedence
in the exit code, but cannot overwrite the retained primary operation error.
All nonzero outcomes require root recovery; code 0 alone is never root authority.
These new signal, code, record and queue rules require explicit S2 approval.

## Required Proof, Not Performed Here

Independent known-answer bytes for every opcode, endian layout and 64-byte exact
bound; rejected version/opcode/reason/reserved/nonce/sequence cases; split reads,
partial EOF, pipe saturation, short write and SIGUSR1 fallback; sender death at
every transition; inheritance leak and stale-PID/PGID reuse rejection; startup
and recovery deadline composition; no blocked diagnostics on the owner; no
post-drain registration/BEGIN_REQUEST; all supported Linux page sizes; same-clock
loaded phase measurements. Neither this document nor either Perl experiment
implements the codec, Rust adapter, durable recovery or broker gates.

## Verified Settlement Is Not Broker Death Per Request

The application manager remains the parent of its broker; PID1 cannot wait a
live grandchild as though it were its own child. Owner verification means
validation of trusted bounded manager receipts against its lifetime/request
ledger, supplemented by PID1 waits for adopted children. It does NOT mean an
independent kernel wait on every tool by PID1. This trust boundary is explicit.
An untrusted application process is outside this same-binary design's premise.

For opcode12, detail is one of these exact closed values (not a bitmask):

- 1 `native_terminal_settled`: bound validated RVB1 terminal frame for the exact
  active request, native child reaped, both native output streams settled, all
  non-broker group members absent, no escape/unknown evidence. These are ADR554
  lines193-201's prerequisites to the existing terminal frame, not new RVB1
  fields or reinterpreted diagnostic strings. Opcode11 detail1 leaves the broker
  lifetime live/idle. The broker and its own pipes remain open for the next call.
- 2 `native_not_started`: manager has settled every preparation operation and
  proves zero RVB1 header bytes written, no child could be created. Opcode11
  detail1 clears this prospective request. No native ID was assigned; its next
  prospective value remains unchanged under ADR558. No abandoned control slot.
- 3 `broker_lifetime_settled`: manager has actual broker wait/no-child evidence,
  complete group absence, all three lifecycle pipes settled and no pending spawn
  or escape. Opcode11 detail2 retires that registration. It also resolves its
  one active request as forced/unclean, never successful. A replacement requires
  a fresh registration and ADR558 B3's existing incident clock and one attempt.
- 4 `hash_lifetime_settled`: retained helper child wait, bounded pipe settlement,
  no outstanding read/descendant and identity continuity. Opcode11 detail3
  retires it. Missing proof retains charged occupancy, regardless of lease age.

All mismatches/unknowns fail closed. Direct local child handles stay with their
current manager; owner records do not steal/reap them concurrently. The owner
reaps adopted descendants only after the actual parent dies, with SIGCHLD DFL
and no SA_NOCLDWAIT. WNOHANG zero means still outstanding; ECHILD alone is not
substitute for the complete retained identity/receipt chain. No raw reused-PID
probe proves an old request settled.

ADR558:304 keeps the native tool in its BROKER's group from spawn through reap.
There is no `kill(-tool_pid)` assumption or per-request new process group. Normal
verified terminal settlement permits broker reuse. Cancellation, deadline,
protocol/pipe ambiguity or failed supervision destroys the COMPLETE broker
group under554/558; lifetime settlement and any B3 replacement follow. Native
5s maximum remains clipped by earlier clocks. No third RVB1 lane or widened
wire/environment/descriptor contract is introduced.

Only opcode14 can precede a graceful handoff: both broker lifetimes, all hash
lifetimes and pending spawn operations must have settled, and the application
has irreversibly stopped every mutator. Owner's final opcode8/child wait still
occurs after DB closure. New opcodes11-14, request-ID binding and final handoff
are named S2 deltas and require known-answer/state/receipt-loss acceptance.
