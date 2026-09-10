# RVB1 native process broker wire and lifecycle contract

- Status: Accepted
- Date: 2026-08-16
- Decision-review revision: 2026-09-09
- Operator approval: 2026-09-09: "ADRs 557-559 are approved according to the resolution"

## Approval Resolution

The operator approved reconciled B1-B3 and ADR 559's shared G1 boundary. E1's
exact environment path, values, bytes, digest, and package changes remain held
for Linux package evidence and separate exact-value approval, as the resolution
requires. Approval does not prove implementation or containment. Historical
proposal and approval-request wording describes the review leading to this
decision; this resolution controls its approval status.

## Decision Summary And Approval Delta

This is an accepted revised contract, not implementation evidence. Accepted
ADRs 514, 549, and 554 remain binding. Relative to the draft at `fb0f6eba`, the
operator accepted B1-B3 and G1 while retaining the separate E1 hold:

| Decision | Recommendation and delta |
| --- | --- |
| B1: Initial unavailability | Replace the blanket API-startup prohibition with a narrowly classified, fully settled, proven-clean initial-unavailability state and explicit outcome/reason mapping. Keep control-plane remediation available, BOTH media lanes closed, and require a deliberate operator restart after repair. Startup deadlines, unsafe containment, and failed recovery remain fatal. |
| B2: Executable authority | Retain file length plus SHA-256 for the same-binary handshake and the complete ADR 519 closure identity for native requests. Missing closure verification still prohibits production requests. |
| B3: Timing and recovery | Retain floor rounding and propose retirement at request-ID exhaustion, one replacement per independent established-lane loss, and the explicit recovery/outcome mapping below. Correct timer origins, bounded cleanup, and exit semantics; do not add retry time to accepted ceilings. |
| G1: Shared governance, ADR 559 | Refer to the proposed delegation in ADR 559, not a separate broker governance decision. It covers only semantics-preserving private internal names and unshipped codec assignments, never values/layouts/bounds already fixed by an accepted contract. |
| E1: Environment values, held separately | Keep the nine-key allowlist and all security validation. Retain the exact table only as a non-authoritative candidate pending Linux package evidence and separate exact-value approval. In particular, `/home/revaer` has an unresolved ownership conflict. Lifecycle approval does NOT approve these values. |

The exact codec remains reviewable in Annex A, not deleted or silently replaced.
No decision in this record changes public APIs, deployment support, retry
controls, required GitHub checks, Sonar criteria, or the goal by implication.
Approving some rows does not approve the others or establish implementation
readiness. The requested replacement goal is drafted in ADR 559; adopting or
activating it remains separate from revising these proposals.

## Context

- Accepted ADR 554 selects two serial same-binary broker lanes and fixes the
  protocol magic, broad field order, bounds, deadlines, environment allowlist,
  recovery count, containment model, and stable outcome names.
- ADR 554 does not assign numeric frame, lane, or response-status values. It also
  does not define complete handshake, request, response, or evidence byte
  layouts; the broker executable identity representation; the exact internal
  argument vector; deadline rounding; request-identifier exhaustion; health
  reason codes; or packaged environment values.
- Those omissions mix interoperability details with consequential failure,
  authority, and package decisions. This revision separates them for review;
  while governance approval is pending, agents cannot treat that distinction as
  permission to choose implementation values or override ADR 554.
- ADR 514 keeps health and control-plane remediation available during handled
  media degradation. ADR 554 permits non-media startup after both lanes are
  ready or explicitly unavailable, but requires fatal startup timeout and
  failed recovery. The original draft incorrectly made every initial broker
  failure fatal without classifying safe unavailability separately.
- The repository contains no earlier RVB1 implementation, numeric registry, or
  release environment table to preserve. The values below are therefore
  recommendations, not discovered compatibility requirements.
- Existing dependencies already provide SHA-256, Unix process operations, and a
  kernel random-byte API. No serialization, socket, random-number, or broker
  runtime dependency is required.

## Options

### Encoding And Registry

1. Use implementation-defined enum discriminants and native integer layout.
   Reject this because compiler layout and host byte order are not a protocol.
2. Use JSON, text, or a general serialization framework. Reject this because it
   adds parsing surface, permits noncanonical representations, handles Unix byte
   strings poorly, and conflicts with ADR 554.
3. Assign an explicit version-1 numeric registry and canonical big-endian byte
   layouts. Recommend this because every accepted or rejected byte sequence can
   be tested without a new dependency.

### Executable Identity

1. Echo only the executable path. Reject this because a path is mutable audit
   evidence, not an identity.
2. Use only the broker executable's file digest for both the broker handshake and
   native-tool requests. Reject this for native tools because ADR 519 requires a
   digest over the complete execution closure.
3. Use file length plus SHA-256 of the exact `revaer-app` bytes for the broker
   handshake, and use ADR 519's canonical `native_tool_identity_v1` SHA-256 for
   native-tool requests. Recommend this because the two identities prove
   different things without expanding either wire field.

### Environment Source

1. Forward every ambient application variable. Reject this because it forwards
   secrets and violates ADR 554.
2. Copy whichever allowlisted values happen to be present. Reject this for the
   packaged runtime because Helm or container configuration could silently
   change process behavior.
3. Propose one exact root-owned package manifest as the source for the nine
   approved keys, with byte/digest and filesystem verification. Recommend the
   source model, but hold its candidate path and values for package evidence and
   separate exact-value approval. An injected nonproduction source must pass
   the same key, byte, NUL, and filesystem validation and cannot claim
   production readiness.

### Deadline Rounding And Recovery Budget

1. Round a positive sub-millisecond remainder up to one millisecond. Reject this
   because it extends the caller's deadline.
2. Round down and reject zero whole milliseconds before writing a request.
   Recommend this because the parent remains authoritative and never grants
   time it no longer has.
3. Permit one replacement for the entire application lifetime. Reject this as
   stricter than ADR 554 requires and operationally surprising after a completed
   recovery.
4. Permit exactly one replacement attempt for each lane-loss incident, with no
   recursive attempt inside that incident. Recommend this interpretation of
   ADR 554's one-attempt rule.

### Startup Failure

1. Terminate the API for every initial failure. Reject the blanket rule: it
   conflates proven-clean media unavailability with an unsafe lifecycle and
   prevents ADR 514 remediation access without an explicit need.
2. Continue with one lane, automatically retry initial failures, or direct-spawn
   native tools. Reject all three: they violate ADR 554's closed media admission
   and recovery boundaries.
3. Continue only for the enumerated clean initial failures below after BOTH
   lanes are settled and every created broker is cleaned up. Recommend this
   bounded degraded state, with explicit operator repair and whole-process
   restart rather than an invented retry endpoint. All fatal classes stay fatal.

## Recommendation

B1-B3 are approved according to the reconciled resolution. E1 remains explicitly
held pending evidence and separate exact-value approval. The lifecycle and
authority sections define the approved semantics; Annex A retains the concrete
encoding contract. Approval does not establish implementation readiness.

### Shared Governance Reference (G1)

ADR 559 owns accepted G1, the narrow delegation for semantics-preserving private
internal names and unshipped codec assignments. This ADR creates no competing
delegation. The operator approved G1 with B1-B3; none of them resolves E1.

Annex A separates exact codec detail for review, not exemption. Values, layouts,
bounds, and every other obligation fixed in accepted ADR 554 or another accepted
decision remain unchanged. Any superseding contract change must be separately
named and approved. Safety, privilege, persistence, public API, support,
deadlines, retries, cleanup, secrets, and quality criteria remain
architecture/operator-gated, including their effects hidden behind internal
names or numeric assignments. G1 cannot authorize an unapproved semantic
choice, a public reason/exit change, or unvalidated package values.

### Executable Authority And Production Binding (B2)

The 32-byte request identity is the raw binary SHA-256 represented as lowercase
hex only at persistence or API boundaries by accepted ADR 519. It is not merely
the executable-file digest. It is the canonical digest over the complete
`native_tool_identity_v1` row and ordered dependency manifest.

Production creates a request only from a sealed `VerifiedNativeExecutable`
value supplied by an injected execution-identity authority. That value contains
the canonical absolute program bytes and the 32-byte canonical tool digest and
has no public unchecked constructor. Exactly two authorities may create it:

- bootstrap capability discovery uses the descriptor-verified, root-owned,
  non-writable packaged tool catalog and persists the resulting complete ADR 519
  identity in the new capability run;
- every later inspection, transcode, analysis, playback, and verification call
  uses the immutable tool identity from that job's bound completed capability
  snapshot and plan generation.

The parent verifies the currently opened execution closure against that digest
before queue admission. The broker independently reopens the canonical path,
proves a root-owned regular file with no group or other write bits, recomputes
the ADR 519 closure identity, and compares it immediately before native spawn.
The packaged image's root-owned read-only tree is the path-substitution proof.
If the complete ADR 519 verifier or immutable package proof is unavailable, no
production request is admitted; the broker path does not fall back to a path
digest, PATH lookup, or the in-process supervisor.

### Environment Candidate And Separate Hold (E1)

The following manifest location, table, bytes, length, and digest are an
explicitly NON-AUTHORITATIVE candidate, not a verified release configuration.
Do not install or enforce these proposed exact values as approved package policy
until Linux `amd64` and `arm64` evidence and separate exact-value operator
approval resolve E1. Approval of B1-B3 or G1 does not approve any environment
value, path, digest, or package-layout change.

The candidate source model is one manifest at
`/app/config/media-process-broker-env-v1`, sourced from the same relative path
under repository `config/`. The proposed file is root-owned, mode `0444`,
regular, and immutable in the image. Its candidate grammar is exactly nine ASCII
`NAME=value` records in the order below, separated by LF, with one final LF and
no blank line, comment, escape, quote, CR, duplicate, or additional byte:

| Name | Candidate value, evidence and approval pending |
| --- | --- |
| `PATH` | `/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin` |
| `LD_LIBRARY_PATH` | `/usr/local/lib` |
| `SSL_CERT_FILE` | `/etc/ssl/certs/ca-certificates.crt` |
| `SSL_CERT_DIR` | `/etc/ssl/certs` |
| `LANG` | `C` |
| `LC_ALL` | `C` |
| `TZ` | `UTC` |
| `HOME` | `/home/revaer` |
| `TMPDIR` | `/tmp` |

The candidate file bytes are:

```text
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
LD_LIBRARY_PATH=/usr/local/lib
SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
SSL_CERT_DIR=/etc/ssl/certs
LANG=C
LC_ALL=C
TZ=UTC
HOME=/home/revaer
TMPDIR=/tmp
```

These candidate bytes have SHA-256
`03eb399aa642dcaf17de8a6329d45be135c17277add98703d700d67fae77b2b1`
and length 227 bytes. A digest of proposed text is not release-image evidence.
Of the nine keys, only `LD_LIBRARY_PATH` is explicit in the inspected Dockerfile
at integration `7fd5df30`. No image was built or inspected for this revision;
path existence, ownership, modes, certificates, dynamic loading, and tool
behavior under this exact table remain unverified on both architectures.

In particular, candidate `HOME=/home/revaer` conflicts with the validation
requirement below if that directory is service-owned or writable by the service.
The Dockerfile creates the `revaer` account but contains no explicit step proving
a root-owned, non-writable home at this path. Do not infer compatible ownership
from account creation or silently exempt `HOME`. E1 must present actual image
ownership/mode evidence and an exact-value/package proposal: prove this path
meets the existing checks or propose a different verified home and its package
changes for approval. Relaxing ownership, permissions, or another security
check is a distinct architectural exception, not an environment-value fix.

After E1 is resolved, production bootstrap must open the approved manifest
without following a symlink, verify owner, type, mode, byte length, and digest,
parse the exact grammar, and snapshot the validated bytes inside the bounded
startup initialization before either broker spawns. It does not source
the file as shell code or copy values from ambient process variables. Each name
and value is nonempty ASCII, contains no NUL, and is at most 4,096 bytes; a name
also cannot contain `=`. `PATH` and `LD_LIBRARY_PATH` entries, and values whose
names end in `_DIR` or `_FILE` or equal `HOME` or `TMPDIR`, must be absolute.
No empty search-path entry or implicit current-directory search is permitted.
`SSL_CERT_FILE` is a root-owned, non-writable regular file; directory values are
root-owned, non-writable
directories. `TMPDIR` is the sole exception to root ownership: it must be an
existing sticky directory with mode `01777`, and the broker never creates its
IPC there.

Broker startup calls `env_clear` and installs exactly the snapshot. Media
children repeat `env_clear` and install the same immutable snapshot rather than
inheriting later broker changes. Database URLs, tokens, encryption values,
tracing exporters, arbitrary `REVAER_*` values, and request-provided variables
cannot reach either process. An environment failure blocks spawn and maps to
`broker_unavailable` with health reason `environment_invalid`; B1 distinguishes
a settled pre-spawn availability error from invalid grammar, identity, or
security evidence. No failure authorizes ambient-variable fallback. During an
established-lane replacement every such failure is fatal failed recovery.

An injected native-development source may use different values only on a
nonproduction support path. It must provide exactly the same nine keys and pass
the same structural and filesystem validation. It cannot set packaged media
readiness or weaken the Linux containment requirement.

### Lifecycle, Readiness, And Fatal Behavior (B1 And B3)

Bootstrap records the common monotonic startup instant immediately before the
first manager-thread creation attempt, not after a thread starts running. It
creates `control` first and `job` immediately afterward without waiting for
control completion. Each lane's 30-second outer deadline is recorded by the
parent before its thread-creation attempt, and the common startup deadline at
the first instant plus 30 seconds also bounds both. Scheduling delay, nonce
generation, environment/executable validation, pipe setup, spawn, handshake,
and initial-failure settlement are included. The startup limit is never 60
seconds and cleanup cannot reset it.

The one immutable environment snapshot is loaded/validated within this bounded,
supervised initialization and supplied identically to both managers before
either broker spawns. Neither manager chooses separate values; a consumer
waiting for snapshot validation still consumes its existing deadline. No
synchronous preparation is moved ahead of the startup clock or out of the
proven deployment boundary. This ordering replaces the old draft's conflicting
claims that validation occurred both before managers and after their clocks
started.

Each manager owns one FIFO queue, one broker child handle, one process-group ID,
and all three pipes. Each lane has at most one active request. No third broker
lane or configurable worker exists. The broker lifecycle stderr reader retains
the first 65,536 bytes and drains later bytes without retaining them. Receipt of
byte 65,537 marks `lifecycle_stderr_limit` and destroys the group. Before initial
readiness it is a fatal startup invariant failure, during established operation
it enters bounded recovery, and during replacement it is failed recovery. Raw
lifecycle stderr is never a health label or general log field.
The broker inherits only stdin, stdout, and stderr. Every other parent descriptor
is close-on-exec and descriptor validation before readiness fails closed if an
unexpected descriptor is inherited. The broker creates no socket, FIFO,
shared-memory object, lock file, or network endpoint. A native child receives
null stdin, dedicated captured stdout and stderr, and no unrelated descriptor;
it remains in the broker process group from spawn through reap.

#### Initial Failure Classification (B1)

"Initial" ends only after BOTH lanes have completed validated handshakes and
the global media gate can open. One lane temporarily reaching `ready` does not
turn failure of its still-starting companion into an established-lane recovery.

Only the following initial errors are candidates for recoverable unavailability:

| Settled error | Parent outcome | Lane reason |
| --- | --- | --- |
| Thread creation returns an error proving that no manager thread was created; not a panic or unexpected thread return | `broker_unavailable` | `startup_thread_failed` |
| Nonce filling returns a nonretryable error or zero bytes and the operation has ended | `broker_unavailable` | `nonce_unavailable` |
| The required manifest or application executable is missing, inaccessible, or has a completed read failure before spawn, without observed integrity/ownership/permission violations | `broker_unavailable` | `environment_invalid` or `executable_identity_invalid` |
| Pipe setup returns an error before any broker spawn and no descriptor or setup operation remains outstanding | `broker_unavailable` | `pipe_setup_failed` |
| Spawn returns an error with affirmative evidence that no child was created or can appear later | `broker_unavailable` | `startup_spawn_failed` |

An error code or lack of a `Child` handle alone is never that evidence. Admission
to the clean class also requires ALL of the following before the original
startup deadline:

1. The approved outer containment and executable trust boundary are proven;
   there is no security violation, unexpected descriptor inheritance, protocol
   corruption, or unresolved identity/process-group mismatch.
2. Both manager creation/setup/spawn operations have definitively settled; any
   created manager thread has joined. No pending callback or blocked operation
   can later create a broker or native child.
3. Every broker that did start, including a ready companion, is terminated and
   reaped with bounded pipe settlement and complete group-absence proof. A
   never-spawned lane has affirmative no-child proof. No inherited-output or
   escape evidence leaves cleanup uncertain. Native requests were never admitted.
4. BOTH lanes are latched `failed`/unavailable, queues are closed, and no media
   claim, capability probe, inspection, discovery, retention, or native work can
   start. A companion closed solely for this decision uses proposed reason
   `peer_startup_unavailable`; the originating failure keeps its reason.
5. Control-plane/database bootstrap and independent ADR 514 readiness
   prerequisites can succeed. Media degradation cannot hide a separate fatal
   application-bootstrap error.

If any condition is false or unknown, continuation is forbidden. In particular:

| Fatal initial class | Outcome and reason |
| --- | --- |
| Common or lane 30-second startup deadline expires, including unresolved spawn, validation, or cleanup | `media_broker_startup_deadline`; `startup_deadline` |
| Five-second handshake deadline expires | `broker_unavailable`; `handshake_deadline` |
| Malformed/mismatched handshake, premature expected-frame EOF, duplicate, over-bound, or out-of-order frame | `broker_protocol_failed`; `handshake_invalid` |
| Invalid environment grammar/digest/security metadata, executable digest/ownership/mode, or process-group identity | `broker_unavailable`; `environment_invalid`, `executable_identity_invalid`, or `process_group_invalid` |
| Missing/unsafe outer containment or possible escape | `broker_unavailable`; proposed `containment_unproven` |
| Unexpected descriptor inheritance, manager panic/unexpected return, or lifecycle stderr overflow | `broker_unavailable`; `pipe_setup_failed`, proposed `startup_invariant_failed`, or `lifecycle_stderr_limit` |
| Unproven cleanup without an already-selected primary, or another non-enumerated initial failure including handshake transport failure | `broker_unavailable`; `cleanup_unproven` or proposed `startup_invariant_failed` |

The tables classify cause AND proof, not reason strings alone. Already-selected
primary outcomes stay primary; later cleanup/timeout observations are bounded
secondary evidence. Outer expiry always enforces termination even when a prior
protocol failure remains primary. Fatal startup closes both lanes, cleans known
groups within the unchanged budgets, starts no API listener or media workers,
and returns a nonzero application error for main to exit 1. A blocked spawn
thread or otherwise unproven cleanup requires termination of the complete outer
deployment unit; the application must not detach it or claim a clean return.

#### Clean Degradation And Operator Boundary (B1)

Only the proven-clean class allows ADR 514 non-media control-plane and API
startup. Media-subsystem state is `degraded`, not `ready` and not the fatal
subsystem `failed` state. Each broker lane remains `failed`/unavailable; a lane
state is not an application lifecycle state. `/health/live` and `/health/ready`
retain ADR 514 control-plane semantics. `/health/full` must expose media
degradation and both closed lanes without claiming execution authority. No
existing endpoint contract or deployment probe criterion is changed here.

There is NO automatic initial broker retry, replacement, backoff, partial-lane
service, or direct-spawn fallback. Existing capability refresh or remediation
requests cannot reopen a lane, enqueue work, or start a broker; while closed,
native requests return `broker_unavailable`. ADR 514's handled capability/root
retries are not broker creation authority and cannot override this latch.

The deliberate retry boundary is operator repair followed by a whole-application
restart through the deployment's existing operator-controlled mechanism. The
new process reruns both startup lanes and every readiness barrier. This proposal
adds no retry/reset HTTP route, RPC, CLI command, flag, signal, or in-process
restart hook and does not repurpose an existing refresh route. No readiness or
liveness tuning may be used to manufacture automatic retries of this clean
degraded state. Approved deployment restart after a fatal nonzero lifecycle exit
is distinct and unchanged.

#### Established-Lane Recovery And Shutdown (B3)

A loss after initial readiness changes that lane to `recovering` and globally
closes new media admission. An already-active request on the other lane may
settle. Recovery first performs TERM, a grace capped at five seconds, KILL,
bounded reap/drain, and group-absence proof. The earlier request deadline or
aggregate application-shutdown deadline truncates graceful waiting. Failure to
prove absence skips replacement and is fatal failed recovery, even if a later
observation might become clean.

The accepted B3 budget interpretation starts the 30-second recovery-incident
clock when loss is detected. Cleanup and the single replacement must fit inside
it; they cannot consume 30 seconds each. The replacement's own outer clock is
recorded before fresh-thread creation, includes validation/spawn, and cannot
extend the earlier incident deadline. Its handshake limit is five seconds from
spawn return, also clipped by that earlier deadline. No replacement starts if
the incident budget is exhausted. This makes ADR 554's outer recovery limit
explicit without increasing it or silently assigning an extra cleanup budget.

The requested B3 reconciliation is specific: ADR 554's Process And Containment
Model requires outer recovery termination after 30 seconds, while its Startup,
Recovery, And Readiness section grants a replacement the same 30-second outer
deadline after cleanup. B3 selects one incident ceiling, with each replacement
limited to the remaining time rather than guaranteed a fresh 30 seconds. This
interpretation is authorized by the operator's explicit reconciled B3 approval,
not inferred from the August approval or delegated by G1. This named resolution
controls the predecessor's conflicting fresh-replacement-deadline wording;
all other accepted bounds remain unchanged.

After successful absence proof the manager makes exactly one replacement attempt
on a fresh thread with a new nonce and request ID reset to 1 after handshake.
No recursive attempt is permitted. A successful replacement completes that
lane's incident; global media admission reopens only when BOTH lanes are `ready`
or `busy`. A later independent loss may have one attempt. Concurrent lane losses
cannot turn global closure into serial recovery budgets: the earlier open
incident deadline remains authoritative, and either failed incident is fatal.

Any replacement failure, including an otherwise clean availability error,
requests cooperative global shutdown, prevents new work, settles active work
within ADR 514's aggregate 30-second shutdown budget, stops API and runtimes,
and reports `broker_recovery_failed` with main exit 1. The shutdown budget does
not extend an already-expired lifecycle or native request deadline; mandatory
outer containment remains authoritative. Production never calls `process::exit`,
detaches a manager thread, or falls back to direct spawn.

Calls made while either lane is `starting`, `recovering`, or `failed` return
`broker_unavailable` before entering a lane queue, subject to an already-selected
cancellation/deadline primary outcome. Cancellation while queued
removes the FIFO entry and returns `cancelled` without consuming an identifier or
writing a byte. Cancellation or expiry after the first request byte destroys the
complete broker group and preserves that primary outcome; cleanup or recovery
failure is attached only as bounded secondary evidence. Successful cleanup and
replacement do not require application shutdown; failed cleanup/recovery does.

Health uses exactly these state and reason combinations:

| State | Permitted reason codes |
| --- | --- |
| `starting` | `startup_begin` |
| `ready` | `startup_complete`, `request_complete`, `recovery_complete` |
| `busy` | `request_admitted` |
| `recovering` | `request_cancelled`, `request_deadline`, `broker_eof`, `protocol_invalid`, `supervision_failed`, `lifecycle_stderr_limit`, `request_id_exhausted` |
| `failed` | `startup_thread_failed`, `nonce_unavailable`, `startup_spawn_failed`, `startup_deadline`, `pipe_setup_failed`, `handshake_deadline`, `handshake_invalid`, `process_group_invalid`, `executable_identity_invalid`, `environment_invalid`, `cleanup_unproven`, `containment_unproven`, `startup_invariant_failed`, `peer_startup_unavailable`, `lifecycle_stderr_limit`, `recovery_nonce_failed`, `recovery_environment_invalid`, `recovery_executable_invalid`, `recovery_spawn_failed`, `recovery_pipe_failed`, `recovery_deadline`, `recovery_handshake_failed`, `recovery_process_group_invalid`, `recovery_cleanup_failed` |

This proposed catalog adds no lane state to ADR 554. B1 explicitly proposes
`containment_unproven`, `startup_invariant_failed`, `peer_startup_unavailable`,
and the initial-fatal use of `lifecycle_stderr_limit`; they are public semantic
review items, not G1 internal-name assignments. B3 replacement thread-creation
or unexpected-manager failures use `recovery_spawn_failed` with bounded
secondary evidence; replacement protocol/I/O/stderr-handshake failures use
`recovery_handshake_failed`. Outer expiry uses `recovery_deadline`, and failed
cleanup uses `recovery_cleanup_failed`. No unknown failure may open admission.

No other state or reason is proposed in version 1. Health exposes the lane name,
state, and reason only. It never exposes PIDs, paths, digests, arguments,
environment, protocol bytes, native output, or secondary evidence.

### Malformed And Boundary Outcomes

- Parent-side request construction failures return `supervision_failed` without
  touching a broker. Production types should make these failures unreachable
  from valid callers, but they remain fallible validation, not panics.
- A broker receiving a malformed `hello` or `request` writes no terminal frame,
  attempts bounded child cleanup, and exits nonzero; parent-owned group proof
  remains required. EOF while a handshake or response is expected is always
  `broker_protocol_failed`, including EOF before the first byte, as required by
  ADR 554. Idle broker loss with no frame expected is `broker_unavailable` and
  starts established-lane recovery; it is not premature expected-frame EOF.
- A malformed `hello_ack` is fatal initial startup or failed replacement, not
  B1 clean unavailability. A malformed `response` selects
  `broker_protocol_failed` only if no earlier primary outcome already won.
  Both destroy the group; later failures remain secondary evidence.
- Unknown kind, lane, status, flag, or protocol version; nonzero reserved data;
  over-bound length; integer conversion failure; length-sum overflow; NUL;
  zero request identifier or remaining deadline; relative or empty program;
  extra field bytes; invalid UTF-8 evidence; unsorted or duplicate evidence;
  wrong identifier; wrong direction; duplicate terminal frame; and premature
  EOF are all fatal protocol failures.
- Parent EOF makes the broker stop admitting work and attempt bounded native
  child cleanup; it cannot certify its own completed death or group absence.
  The surviving parent or mandatory outer containment owns complete group
  termination. During intentional application shutdown the parent owns cleanup
  and the manager does not start recovery.
- A valid native status maps only to the identically named ADR 501 outcome. A
  protocol or recovery error cannot convert cancellation, deadline, output
  limit, or exit failure into success.
- The internal broker exits 64 for invalid mode arguments and 70 for every fatal
  lifecycle, parent-loss, supervision, or protocol outcome that it can return
  voluntarily. Parent/outer TERM or KILL may instead yield a signal-derived
  process status; neither is evidence of complete group cleanup. The protocol
  has no shutdown frame and this proposal does not invent an exit-0 handshake
  after the broker has already been killed. The application exits 1 for fatal
  B1 startup or failed B3 recovery, but stays running for proven-clean B1
  degradation. Only an otherwise successful orderly application shutdown may
  return application exit 0 after accepted cleanup obligations are satisfied.
  These exit semantics are B1/B3 decisions, not private codec assignments.

## Consequences

- Parent and broker implementations have one canonical byte representation and
  cannot accidentally depend on Rust enum layout or lossy text conversion.
- The parent never rounds a deadline upward. Queue, pipe, spawn, and output
  consume the original execution budget; deadline expiry forces cleanup, whose
  mandatory settlement is bounded by lifecycle/outer containment, not falsely
  promised to have completed before the instant that triggered it.
- The executable and environment wire surfaces carry no path in the handshake,
  no secret, and no arbitrary request environment.
- Clean initial unavailability keeps remediation reachable while explicitly
  advertising media degradation and closing BOTH lanes. It sacrifices automatic
  broker recovery in that state: the operator must repair and restart. Unsafe
  startup, startup deadline, and failed established-lane recovery remain fatal.
- The candidate environment is not validated. HOME ownership and every other
  exact package value remain E1 evidence/approval holds, not permission to
  weaken security checks or silently accept runtime substitutions.
- A one-attempt-per-incident policy permits recovery from later independent
  failures but does not permit a recursive retry loop for one failure.
- Implementing the request authority depends on completing accepted ADR 519's
  closure verifier and immutable job binding. Until then the broker may be
  tested with injected identities but cannot admit production media requests.
- ADR 554's production boundary remains validated Linux `amd64`/`arm64` in one
  container/cgroup or equivalent supported Linux service-manager containment.
  Plain background launch, unsupervised Docker, Docker Desktop, macOS, Windows,
  network filesystems, and uncontained shells remain unsupported production
  media environments. Injected tests on another platform are not containment
  evidence; clean degradation does not upgrade its support classification.

## Implementation Boundary

- This revision records approval of B1-B3 according to the reconciled resolution.
  E1 requires package evidence plus
  separate approval of the exact manifest path, values, bytes, digest, and any
  package changes. Approval of lifecycle behavior does not resolve E1.
- G1 is solely the shared accepted governance item in ADR 559. ADR 554's existing
  exact-value constraints remain in force except for the named reconciled B1-B3
  decisions. G1 does not change an accepted constant, layout,
  bound, or semantic requirement. A superseding change must be separately named
  and approved; nothing in this record implicitly supersedes ADR 514, 519, 549,
  or any other part of ADR 554.
- Neither these decisions nor G1 authorize a socket, sidecar, additional lane,
  configurable pool, second recovery attempt for one incident, direct-spawn fallback,
  request-provided environment, path-only production identity, new dependency,
  unsafe code, higher media concurrency, non-Linux production claim, weaker
  containment, or criteria relaxation.
- The implementation must use dependency injection for nonce, clock, executable
  authority, environment source, lifecycle spawning, cleanup operations, and
  cooperative shutdown. Production bootstrap alone constructs concrete
  implementations.
- This proposal changes no Rust, package, Docker, Helm, workflow, health, API,
  database, or runtime behavior. No affected prototype may be committed or
  pushed before its applicable decision-specific operator approvals. Partial
  lifecycle approval does not authorize an E1-dependent production prototype.
- Public reason codes, exit semantics, bounds, deadlines, retries, cleanup,
  privilege, secrets, persistence, public APIs, support, and quality criteria
  remain architecture/operator-gated. The review annex is not permission to
  treat these as private assignments or to weaken required checks.

## Exact Validation

- Add known-answer vectors for every frame and payload, including exact hex for
  both lanes and every status. Encode then decode must reproduce the same typed
  value; decode then encode must produce the one canonical byte sequence.
- Cover empty, exact-bound, one-over-bound, truncated, trailing, duplicated,
  reordered, unknown, nonzero-reserved, wrong-direction, wrong-state, invalid
  UTF-8 evidence, NUL, length-sum overflow, and integer-conversion cases for
  every field. Run deterministic generated-input parser tests without adding a
  persistent fuzzing dependency.
- Prove a 16-byte injected nonce round trip, kernel-random production filling,
  nonce mismatch, duplicate hello, five-second handshake cutoff, PID/PGID
  mismatch, binary length mismatch, binary digest mismatch, and `/proc/self/exe`
  validation.
- Prove startup clocks are parent-recorded before thread creation and include
  scheduling, environment snapshot, identity validation, spawn, and cleanup;
  concurrent lanes share the earlier 30-second startup boundary, not a serial
  60-second allowance. A late handshake never produces readiness.
- Prove floor rounding at zero, one nanosecond, 999,999 nanoseconds, one
  millisecond, and the ADR 501 maximum; include queue and first-byte I/O time and
  prove the parent deadline remains authoritative.
- Set the next request ID to `u64::MAX` through an injected state seam. Prove one
  valid terminal exchange, no wrap, global admission closure, one replacement,
  and reset to 1 only after a valid new handshake.
- Exercise control and job concurrently; prove FIFO within each lane, queued
  cancellation without broker bytes, one active request per lane, global
  admission closure during recovery, and exactly two broker managers.
- Prove every valid response status, every inconsistent field/status pair,
  exact stream limits, the 50,335,744-byte response bound, canonical evidence
  ordering, duplicate rejection, evidence truncation, lifecycle stderr byte
  65,536 and 65,537 behavior, and no sensitive field in health or logs.
- Prove capability-discovery authority and bound-job authority create the same
  ADR 519 digest for the same closure. Test changed bytes, dependency, path,
  owner, mode, architecture, package tree, and capability binding before spawn.
- Map every production caller, including startup/on-demand capability refresh
  and manual preview/inspection, to its approved identity authority. Do not use
  the discovery authority as an escape hatch for an unbound execution request.
  Any caller that cannot satisfy an accepted binding contract remains blocked
  pending explicit reconciliation; codec completion alone is not call-site
  integration evidence.
- Build the release image for Linux `amd64` and `arm64`; inspect its OCI
  environment and filesystem; supply E1 evidence and obtain separate exact-value
  approval before wiring a production table. Prove all nine approved values and
  paths, HOME ownership/mode, dynamic loading and tool behavior, broker and
  child environment equality, absence of every secret and arbitrary variable,
  one-container cgroup containment, no socket or network endpoint, restart on
  nonzero exit, and complete cleanup of a session-escaping fixture by the outer
  containment unit.
- Test every B1 eligible cause with both lanes settled, no-child proof or full
  cleanup of a ready companion, closed queues, and no late-created process.
  Prove global control-plane readiness/remediation remains available while
  media is degraded and BOTH lanes unavailable. Refresh traffic, elapsed time,
  and ordinary health probes must not start brokers; only deliberate repair and
  a whole-process restart rerun startup. No new retry API is part of the test.
- For each eligible cause, independently negate every clean-proof condition and
  prove fatal startup. Include missing files versus corrupt/misowned files,
  spawn error without no-child proof, thread panic versus creation error,
  expected-frame EOF before any byte, handshake expiry, inherited descriptors,
  stderr overflow, and unproven containment. Verify stable primary outcomes and
  bounded secondary evidence when timeout or cleanup fails concurrently.
- Prove initial unavailability never enters replacement, established-lane loss
  has one attempt, clean failure of that replacement is still fatal, and
  concurrent losses cannot extend the earlier incident deadline. Cleanup,
  replacement spawn, and handshake fit inside the one 30-second recovery
  incident, while a shorter request or shutdown deadline remains authoritative.
- Deterministically test initial spawn block, handshake block, child spawn block,
  parent EOF, broker crash, malformed response, request cancellation, request
  deadline, output flood, descendant hold, cleanup failure, successful recovery,
  failed recovery, cooperative shutdown, and process exit values.
- Run focused protocol, broker, bootstrap, package, Helm, and media fixture
  tests; `just fmt`; targeted tests; `just check`; `just ci`; `just ui-e2e`;
  cleanup recipes; strict Sonar with positive coverage and retained evidence;
  both release architectures; all required GitHub checks; and a security diff
  scan before claiming implementation.

## Operator Questions

1. B1: Approve the enumerated proven-clean initial-unavailability class, API
   remediation with BOTH media lanes closed, explicit repair/restart boundary,
   and fatal-class/outcome/reason mappings, rather than blanket API termination?
2. B2: Approve broker file-length/SHA-256 handshake identity, ADR 519 closure
   identity for native requests, and the two closed production identity
   authorities, with no production request before the complete verifier exists?
3. B3: Approve floor rounding, request-ID retirement at `u64::MAX`, one
   replacement per independent established-lane incident, the clarified clock
   origins and recovery budget, and the stated outcome/exit semantics?
4. E1: Keep the exact environment candidate on hold until both architecture
   images supply filesystem/tool evidence and a separate exact-value proposal
   resolves HOME ownership without silently weakening security validation?
5. G1: Decide the shared governance proposal in ADR 559, not a separate broker
   delegation. Pending that decision, review Annex A under the existing
   approval rules; accepted values, layouts, bounds, and semantics stay fixed.

## Follow-Up

- Preserve the recorded B1-B3/G1 approvals and E1 hold in ADR 559. Continue
  independent approved work without claiming E1-dependent production readiness.
- After the applicable approvals and E1 evidence/approval, implement outside in:
  package containment and lifecycle tests, known-answer protocol tests, codec,
  internal mode, environment and executable authority, manager lifecycle,
  readiness/shutdown wiring, supervisor routing, package containment, and final
  direct-spawn removal.
- Keep implementation commits local and unpushed until their scoped validation
  and review are complete.

## Annex A: Exact Codec Draft For Review

This annex preserves the draft's explicit frame, lane, status, offset, width,
argument, and evidence assignments. It is review material, not an accepted
protocol implementation. B1-B3 own the semantic decisions referenced here; E1
is still held separately. Only ADR 559's G1, if explicitly approved, can govern
semantics-preserving private assignments, never layouts or values already fixed
by accepted ADR 554 or another accepted contract.

All integers are unsigned and big-endian unless explicitly described otherwise.
No implicit padding, native-width value, enum representation, optional field,
or trailing extension area exists in this version-1 draft.

### Frame Header And Numeric Registry

Every frame has this exact 12-byte header:

| Offset | Size | Field | Required value |
| --- | ---: | --- | --- |
| 0 | 4 | magic | ASCII bytes `RVB1` |
| 4 | 1 | frame kind | one value from the table below |
| 5 | 3 | reserved | three zero bytes |
| 8 | 4 | payload length | exact payload bytes following the header |

Frame-kind values are:

| Value | Name | Direction | Legal state |
| ---: | --- | --- | --- |
| `0x01` | `hello` | parent to broker | first and only pre-ready frame |
| `0x02` | `hello_ack` | broker to parent | response to one valid `hello` |
| `0x10` | `request` | parent to broker | ready and no active request |
| `0x11` | `response` | broker to parent | exactly once for the active request |

Lane values are `0x01` for `control` and `0x02` for `job`. Response-status values
are:

| Value | Wire status | ADR 501 application outcome |
| ---: | --- | --- |
| `0x00` | `success` | successful bounded output |
| `0x01` | `spawn_failed` | `spawn_failed` |
| `0x02` | `cancelled` | `cancelled` |
| `0x03` | `deadline_exceeded` | `deadline_exceeded` |
| `0x04` | `stdout_limit_exceeded` | `stdout_limit_exceeded` |
| `0x05` | `stderr_limit_exceeded` | `stderr_limit_exceeded` |
| `0x06` | `exit_failed` | `exit_failed` |
| `0x07` | `supervision_failed` | `supervision_failed` |

Every other frame-kind, lane, or response-status byte is invalid in version 1.
The lifecycle outcomes `broker_unavailable`, `broker_protocol_failed`, and
`broker_recovery_failed` are parent-side application outcomes and are never
encoded as broker response statuses.

The parser reads the 12-byte header into fixed storage before considering an
allocation. It rejects the payload length against the legal kind-specific bound
before allocating or reading the payload. A short header, short payload, extra
payload byte, invalid direction, invalid state, duplicate frame, or byte after a
terminal frame where a new request is not expected is a fatal protocol failure.

### Internal Mode And Handshake

The parent uses the same descriptor-verified absolute executable selected for
the application. The exact child argument vector is:

1. `argv[0]`: the canonical absolute path used by `Command::new`;
2. `argv[1]`: ASCII `media-process-broker-v1`;
3. `argv[2]`: ASCII `control` or ASCII `job` matching the manager lane;
4. no additional argument.

Missing, additional, non-ASCII, or unknown mode arguments make the internal
process exit nonzero without writing stdout. The nonce, executable digest,
environment, and native request data never appear in arguments.

The parent creates a fresh 16-byte nonce inside the manager thread after the
parent-recorded startup clock begins and before broker spawn. The proposed nonce
source repeatedly calls the existing `rustix::rand::getrandom` API with no flags
until all 16 bytes are initialized. A zero-byte return or nonretryable error is
`broker_unavailable` during initial startup; a blocked operation at that outer
deadline is `media_broker_startup_deadline`. During replacement either condition
is fatal `broker_recovery_failed` with bounded cause evidence. Tests inject a
deterministic nonce source. UUID
v4 is not used because its fixed version and variant bits do not preserve 128
random bits.

The `hello` payload is exactly 20 bytes:

| Offset | Size | Field | Required value |
| --- | ---: | --- | --- |
| 0 | 2 | protocol version | `0x0001` |
| 2 | 1 | lane | `0x01` or `0x02` |
| 3 | 1 | reserved | zero |
| 4 | 16 | nonce | parent-generated bytes |

The `hello_ack` payload is exactly 68 bytes:

| Offset | Size | Field | Required value |
| --- | ---: | --- | --- |
| 0 | 2 | protocol version | `0x0001` |
| 2 | 1 | lane | exact requested lane |
| 3 | 1 | reserved | zero |
| 4 | 16 | nonce | exact `hello` nonce |
| 20 | 4 | child PID | positive Linux PID, at most `i32::MAX` |
| 24 | 4 | process-group ID | exactly equal to child PID |
| 28 | 8 | executable byte length | exact descriptor length |
| 36 | 32 | executable SHA-256 | digest of the exact descriptor bytes |

Before sending `hello`, the parent opens and validates the application
executable as a root-owned, regular, non-group-writable, non-other-writable file,
records its length and SHA-256, and retains the immutable package proof required
by ADR 554. The broker opens `/proc/self/exe`, applies the same metadata checks,
hashes those bytes, becomes its process-group leader, and only then emits
`hello_ack`. The parent compares the PID with its existing child handle, verifies
the process-group ID through the operating system, and compares every payload
field. Paths are not carried in the handshake.

The parent writes one `hello` immediately after spawn and accepts one
`hello_ack`. The five-second handshake deadline starts when `spawn` returns and
is also bounded by the earlier initial-startup or recovery-incident deadline.
Any other first frame, duplicate handshake, mismatch, premature EOF, or late
acknowledgement destroys the complete broker group. During initial startup this
is a fatal B1 failure, even if cleanup succeeds; during replacement it is fatal
failed recovery under B3. A late acknowledgement never restores readiness.

### Request Payload

The request payload has this exact layout:

| Order | Size | Field |
| ---: | ---: | --- |
| 1 | 2 | protocol version, exactly `0x0001` |
| 2 | 1 | lane |
| 3 | 1 | reserved zero |
| 4 | 8 | request identifier |
| 5 | 8 | remaining deadline in whole milliseconds |
| 6 | 4 | maximum stdout bytes |
| 7 | 4 | maximum stderr bytes |
| 8 | 32 | raw `native_tool_identity_v1` SHA-256 digest |
| 9 | 4 | program byte length |
| 10 | variable | program bytes |
| 11 | 4 | argument count |
| 12 | repeated | one 4-byte length and that many argument bytes |

The payload is 69 through 1,048,576 bytes. The program length is 1 through 4,096
bytes. It is an absolute Unix path, begins with `/`, contains no NUL, and is not
decoded as text. Argument count is 0 through 4,096. Each argument is 0 through
4,096 bytes and contains no NUL. The sum of program bytes and argument bytes is
at most 1,000,000 bytes; length prefixes and fixed fields do not count toward
that sub-bound but do count toward the frame bound. Stdout and stderr limits are
each 0 through 16,777,216 bytes. Zero preserves a caller's request to accept no
bytes from that stream.

The `control` lane accepts only capability detection and inspection requests
whose complete timeout is at most 30 seconds. The `job` lane accepts only
transcode, audio-analysis, playback, and output-verification requests using ADR
501's duration-derived bounds. Lane selection is a closed typed request field,
not inferred from program or arguments. A parent-side category mismatch fails
before write as `supervision_failed`; a wire lane mismatch is a fatal protocol
failure.

The caller records its absolute monotonic deadline before checking the global
readiness gate or entering a lane queue. The readiness check never waits for
startup or recovery: closed admission returns `broker_unavailable`. Queue order
is FIFO within each lane. While waiting for queue ownership, pipe I/O, or
response completion, the parent checks cancellation and its absolute deadline
at ADR 501's five-millisecond interval and at every state transition. A queued
request not yet written rechecks readiness before write and is rejected without
an identifier if either lane has become unavailable.

Immediately before writing the first request-header byte, the parent computes
`floor(remaining_nanoseconds / 1,000,000)`. If the result is zero, it returns
`deadline_exceeded` without assigning an identifier or touching the broker. The
encoded value must fit `u64`; ADR 501's maximum 24-hour request already fits. The
broker records a local start instant when it receives the first byte of the
request header and sets its local deadline to that instant plus the encoded
milliseconds. The parent retains and enforces the original earlier deadline, so
pipe transit or broker scheduling can never extend the caller's authority.
When cancellation or a deadline begins cleanup, the TERM wait is capped by
ADR 501's five-second grace and the earliest applicable request, lifecycle, or
application-shutdown deadline. Expiry cuts short the grace and forces KILL,
bounded reap, pipe drain, and absence proof. Cleanup cannot restore success or
extend execution authority. Final reap/absence evidence may follow deadline
revocation; this is not extra execution time or permission for an unbounded
return. Unsettled cleanup at the applicable lifecycle/shutdown boundary is fatal
and relies on mandatory outer containment, never a detached spawn thread.

Identifiers begin at 1 for each successfully handshaken broker lifetime. An
identifier is assigned only after the final cancellation/deadline check and
immediately before the first frame byte is written. A partial write destroys the
lane, so no surviving broker can observe a gap. After the terminal response for
identifier `u64::MAX`, the lane enters recovery before another request is
admitted. A successful replacement restarts at 1. Wrap to zero is forbidden.

### Response And Evidence Payload

The response payload begins with this exact 28-byte prefix:

| Offset | Size | Field |
| --- | ---: | --- |
| 0 | 2 | protocol version, exactly `0x0001` |
| 2 | 1 | lane |
| 3 | 1 | response status |
| 4 | 8 | active request identifier |
| 12 | 1 | flags |
| 13 | 3 | reserved zero |
| 16 | 4 | stdout field length |
| 20 | 4 | stderr/detail field length |
| 24 | 4 | evidence field length |

The three fields follow in that order with no alignment or trailing bytes. Each
field is at most 16,777,216 bytes, and the complete response payload is at most
50,335,744 bytes. The only defined flag is bit 0,
`secondary_evidence_truncated`; bits 1 through 7 are zero.

The evidence field is itself canonical binary data:

1. a 4-byte item count;
2. for each item, a 4-byte byte length followed by that many bytes;
3. no trailing bytes.

An empty evidence field is encoded as four zero bytes and therefore has field
length 4. Every evidence item is nonempty, valid UTF-8 already normalized by ADR
501's evidence type, unique, and sorted by unsigned UTF-8 byte order. Item
length prefixes and bytes are included in the 16,777,216-byte evidence-field
bound. The truncation flag is set when ADR 501 omitted any item or byte to stay
within that bound. The protocol does not reinterpret, concatenate, or re-log the
messages.

The item count cannot exceed
`floor((evidence_field_length - 4) / 5)` because every item has a four-byte
prefix and at least one byte. It is therefore at most 3,355,442, and one item is
at most 16,777,208 bytes after the count and its own prefix. The decoder proves
both derived bounds before reserving item storage.

For `success`, stdout and stderr contain the complete bounded native streams,
the evidence field is empty, and the truncation flag is zero. For every failure,
stdout is empty. The stderr/detail field contains only the existing bounded
diagnostic allowed by ADR 501, capped by both the request stderr limit and the
global 16,777,216-byte limit. The evidence field contains only secondary read,
cleanup, or recovery evidence. A response with fields inconsistent with its
status is a fatal protocol failure.

The parent's stream limits and absolute outcome are authoritative. A response
after parent cancellation or expiry cannot become success; the parent destroys
the group and preserves the already-selected primary outcome. A field exceeding
the request limit, a wrong identifier or lane, or a second terminal frame is a
fatal protocol failure for the lane. If cancellation, deadline, or another ADR
501 primary outcome already won, later protocol/cleanup failure is only bounded
secondary evidence, not a replacement primary outcome.

## Task Record

- Motivation:
  - Revise the recovered draft for operator decision review only, separating
    substantive broker choices from encoding detail and reconciling remediation
    availability with the accepted fatal lifecycle boundaries.
  - Revision date: 2026-09-09. Base: accepted integration `7fd5df30` on branch
    `work/media3-broker-review-20260909`. Draft recovered with `git show
    fb0f6eba:docs/adr/558-rvb1-native-process-broker-wire-contract.md` and applied
    using `apply_patch`; no checkout-based file replacement was used.
- Design notes:
  - B1 permits only enumerated, fully settled, proven-clean initial
    unavailability to retain control-plane remediation, with BOTH media lanes
    closed until explicit repair and whole-process restart. Fatal containment,
    startup deadlines, and failed recovery are preserved.
  - B2 preserves broker versus native-closure identity and ADR 519 as a
    production prerequisite. B3 makes timer origins, the recovery-incident
    ceiling, primary/secondary outcomes, and process exit behavior explicit.
  - E1 remains evidence/approval-pending, including HOME ownership. The exact
    candidate is retained for review without claiming image compatibility or
    approving its values through a lifecycle decision.
  - Annex A preserves the explicit byte tables and layouts from the draft; its
    surrounding lifecycle prose is reconciled with B1/B3. G1 is a reference to
    ADR 559's shared proposed delegation, not an independent governance change.
- Test coverage summary:
  - This revision adds no tests and claims no runtime or release-image
    validation. The implementation test matrix above remains mandatory.
  - Review is limited to the accepted contracts, relevant instructions,
    Dockerfile source evidence, decision consistency, and diff hygiene.
    `just ci`, `just ui-e2e`, documentation generation/build/links, and combined
    gates are not run in this isolated revision; the integrating agent owns
    those checks. Their requirements are unchanged and no pass is inferred.
  - No dependency, test fixture, or media file is created or acquired.
- Observability updates:
  - No live telemetry or health-route changes. Proposed B1 reason assignments
    and B3 outcomes remain explicit semantic decisions; no path, PID, digest,
    argument, environment, protocol bytes, native output, or evidence is added
    to health labels.
- Status-doc validation:
  - The isolated broker subtask revised only this ADR. The combined local review
    integrates ADRs 557-559, indexes, and generated documentation surfaces;
    combined validation is recorded in ADR 559. Instructions, runtime code,
    workflows, and the active goal remain unchanged.
  - Status stays `Proposed`; operator approval stays `Pending`. Edits remain
    local for integration through `apply_patch`; no GitHub access,
    browser, push, acceptance, or architectural implementation is part of this
    revision. Worktree removal belongs to the integrating agent afterward.
- Risk & rollback plan:
  - An overly broad clean classification could hide unsafe startup; default
    fatal classification, both-lane settlement, and proof-negation tests are
    required. A strict classification may instead require more operator
    restarts, which is an explicit availability cost for B1 review.
  - Package compatibility and the HOME conflict are unresolved E1 risks, not
    reasons to weaken validation. Timing semantics in B3 and shared G1 require
    their own explicit decisions; no recommendation is recorded as consent.
  - Before implementation, rollback is a reviewed revision or supersession of
    this proposal. After an approved implementation, rollback must disable
    media readiness and revert the coordinated slice without direct spawning.
- Dependency rationale:
  - No dependency is added. A future implementation may enable the existing
    `rustix` random API feature and use existing SHA-256 support; adding another
    runtime dependency is outside this proposal.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`,
    `.github/instructions/ffi.instructions.md`, and
    `.github/instructions/devops.instructions.md`, plus accepted ADRs 501, 514,
    519, 549, and 554. Dockerfile inspection is source evidence only.
  - Removed this draft's blanket API-startup prohibition, unverified image-match
    assertion, contradictory environment/timer ordering, unconditional recovery
    on initial failure, unconditional shutdown after successful recovery,
    inconsistent expected-frame EOF mapping, impossible broker exit-0 cleanup
    claim, and broad internal-assignment approval wording. Revised primary
    outcome and cleanup timing prose without weakening accepted obligations.
  - Existing `rust.instructions.md` says it wins a conflict with the root;
    `AGENTS.md` explicitly says the opposite. Root precedence governs this
    review. That pre-existing instruction contradiction is reported, not edited,
    because instructions are outside this task's scope.
  - No workflow, required check, Sonar criterion, lint policy, or other quality
    requirement is relaxed. This documentation task does not assert overall
    completion or authorize implementation before operator decisions.
