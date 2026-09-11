# S2 Bounded Shutdown Investigation: Revision 3 Final

> Current approval: The operator accepted ADR 588's exact applicable choices
> on 2026-09-11 at reviewed commit `9575c077`. The [resolution](../588-first-release-decision-package.md#approval-resolution)
> releases only those named design holds. Earlier pending/candidate wording
> below retains its historical context; implementation and qualification are
> not certified, and all other conditions remain binding.

Status: **DECISION EVIDENCE ONLY. CORRECTIVE MECHANISM OBSERVED; RELEASE NOT QUALIFIED.**
The original additional P1024 run failed. Exactly one separately parent-delegated
corrective P1024 run then tested the minimal owner at the existing28s/2s proposal.
It met that case's phase ceilings. No completed v1 trial was repeated.
No tracked/production change, build, image pull, prune, new service, upload,
commit or push. Parent owns ADR588 and all implementation/integration decisions.

Source/base: `fcdd95596fa062de9388d1be4f2ac96bbff571d1`.
Branch: `work/media3-shutdown-bound-20260911`.
Worktree: `/private/tmp/revaer-shutdown-bound-20260911`.
Ownership: ignored `artifacts/decision-evidence/shutdown-bound/` only.
This final revision incorporates explicit verified-settlement messages, normal
broker reuse, graceful durable handoff for planned restarts, lost-proof operator
recovery, final DISC values and authoritative parent LIFE-1. Conditional selection
for the parent architecture package is NOT implementation or release approval.

## Recommendation

Recommend owned cooperative shutdown plus a **same-binary, non-Tokio PID1
containment owner**, conditionally on implementation and integrated acceptance.
The minimal mechanism now has direct P1024 evidence; full S2 is not qualified.
Retain these
exact values for the parent's approval/acceptance package:

| Value | Exact candidate / authority |
| --- | --- |
| Aggregate application deadline | Existing 30s, one immutable origin |
| External package teardown grace | Existing 45s; disaster backstop, never application success |
| Native TERM grace | Existing maximum 5s; no new grace after a delayed callback |
| Native KILL wakeup target | NEW 4.750s from original drain, request required by 5.000s; earlier request/incident deadlines win |
| Whole-unit KILL wakeup target | NEW 28.000s, request required by 28.250s |
| Reserved final phase | NEW 2.000s inside 30s: 250ms dispatch + 1,500ms signal/exit/reap + 250ms final observation/classification |
| Linux task envelope | NEW proposed admission/release qualification constraint: 1,024 total tasks including PID1, application, every thread and descendant |
| Maximum all-single-thread descendant case | 1,023 descendants, not 1,023 plus arbitrary native threads |
| Owner records | 1,024 fixed slots x128 bytes; 128KiB, no unbounded reap history |
| Production CPU/thread settings | NONE selected or changed; no CPU affinity/count, Tokio, FFmpeg, libtorrent or helper tuning hidden in the envelope |

**Retain the predeclared2s reserve, now supported by this one minimal-mechanism
observation, not a universal settlement bound.** At P1024 the corrective run
requested KILL at28.076101429s, collected1,023 exact SIGKILL wait statuses and
ECHILD, and confirmed namespace/cgroup absence at28.396035429s. Classification
completed at28.396035721s, leaving1.603964279s inside30s. The phase allowances
have3.285x observed dispatch and4.698x observed signal/exit/reap headroom; total
reserve is5.050x observed target-to-classification time. This is conservative
engineering headroom for a proposed finite qualification set, not statistical
confidence or an OS guarantee. No P32 latency contributes to it.

The failed v2 run still proves no settlement:0waits,1,023remaining TIDs. Its
preregistered10x formula has no valid input; the spurious61s result is rejected,
not used to select a new schedule. The parent agent allocated exactly one bounded
mechanism correction at28s/2s under the user's general isolated-experiment goal.
That delegation is NOT operator approval of the schedule or architecture;
actual approval is pending. No additional cutoff/reserve or experiment is selected.
The corrective repeat-action target at15s was serviced at16.775503591s, so even
this case does NOT establish universal J250ms or native5s dispatch. Full
integrated phase/lease/recovery acceptance remains mandatory.

P1024 is useful because it retains the default Tokio blocking capacity and
leaves capacity for the full two-lane/native/discovery service; it is NOT proof
that all current settings or all legal media fit. A full-spec workload that
does not fit blocks qualification, rather than being silently removed from
the product. Admission must reserve growth before side effects, use typed
capacity failure and preserve prior state. A kernel EAGAIN after unaccounted
thread growth is a failed admission invariant, not graceful capacity control.

## Original Evidence Preserved

Before changing anything, all 24 original files (291,978 bytes), including the
report, harnesses, result JSON, numerical limits and command ledger, were copied
byte-for-byte to `history/v1/`. `history/v1/SNAPSHOT.json` and its `SHA256SUMS`
record the originals. Original REPORT SHA256:
`286ad974d4872cac7d89163fdb9f13a53aad73afbbefcb19efcd3e514159e2fc`.

The stopped v1 report claimed `COMMANDS.md`, `provenance.json`,
`verification.json`, `artifact-manifest.json` and `SHA256SUMS` already existed;
they did not. The snapshot records that discrepancy rather than retrospectively
inventing a v1 seal. The original `justfile` also declares verify/seal recipes
whose modes its harness does not implement. It is preserved, not invoked.

Unchanged v1 controls include nine Linux scenarios and 30 accounting/state
checks. Its actual-30s P32 earlier-force case requested KILL at 29.000720763s,
reaped 31 descendants by 29.003343304s and had 0.996656696s left. The short P32
case had an 84.089ms KILL-to-reap tail. The original deadline case finished at
30.002121972s, late/unclean. These remain controls only, never a release envelope
or source of the revised reserve. The external 3s control exited 137 with
stopped readback at 3.145920s; it did not test a real 45s deployment grace.

The exact failed revision-2 run harness and output were additionally copied to
`history/v2-run/` before assessment/verification code changed. `envelope.pl`
and all run results remain unchanged. No silent repair or rerun was performed.
Before the parent-delegated correction, all41 non-history v2 files were sealed under
`history/v2-report/`, including report hash
`58764bc2315683c0bfdaa91355b28b73e44c43f36aafdeebb95efc9034a12eff`
and its then-current SHA256SUMS. New corrective files are separate; no failed
raw timestamp is relabeled as a successful wait/absence observation.
The snapshot's administrative "operator authorized" reason is preserved text,
not an approval record: the precise allocation was parent delegation as above.

## Actual Task Contributors

Current source and proposed accepted topology are deliberately separated.
An async future is not an independent Linux task. The following is a census
model of actual contributors, not a claim that PIDs were measured in Revaer:

| Contributor | Source/config finding and Linux-task consequence |
| --- | --- |
| App main + Tokio workers | `crates/revaer-app/src/main.rs:22` uses plain `#[tokio::main]`. Cargo.lock:3862 pins Tokio1.52.3. No application `worker_threads` override found. One app main plus W worker threads; W follows available parallelism or TOKIO_WORKER_THREADS. No portable fixed W is declared in chart values. |
| Tokio blocking pool | Locked local Tokio `src/runtime/builder.rs:300,515-546` defaults to 512 additional blocking threads, separate from W, not necessarily all created. App spawn_blocking sites include capabilities (`media.rs:264`), discovery scan/hash (`media_discovery_runtime.rs:196,304,317`), retention (`media_workspace_retention.rs:124`), execution/recovery/verification/replacement (`media_job_runtime.rs:533,936,947,1098,1144,1353,1400,1434,1830,3727`). Tokio FS and DNS also use this pool. Its pending queue is not bounded merely by its thread cap. |
| Async service ownership | `bootstrap.rs:353-423` starts indexer, import, discovery, job and retention loops; `orchestrator.rs:251` adds config work. They share Tokio threads, but their receipts, cancellation and resource ownership must all settle. Each connection/SSE/config future is not a new native thread. |
| Libtorrent | `worker.rs:31-75` launches a detached Tokio worker and returns no handle; its poll/alt-speed branches are in that same async worker. `session/native.rs:24-27` constructs a native session; `ffi/session.cpp:556` creates `lt::session`, and `:2474` uses its destructor. Internal network, disk/hash/resolver threads are additional to Tokio. This shim does not set aio_threads, hashing_threads or a total native thread bound. No exact packaged L was measured. |
| Native media calls today | `process/system.rs:70-141` synchronously spawns one tool, drains both nonblocking pipes on its calling thread and polls; callers often occupy a blocking-pool thread. Direct paths still present in execute:269, verification:303 and app audio-analysis:323 add a stderr thread per invocation. They are not assumed gone merely because broker ADRs are accepted. |
| Full accepted broker topology | ADR554:55-69 and ADR558 require exactly two serial same-binary lanes, two dedicated manager threads, two broker processes and at most one active tool per lane. Include both control and job native processes simultaneously, not just one encode. Current direct paths must be replaced, not run as an additional hidden fallback. No third native-tool lane. |
| FFmpeg/ffprobe and tool internals | No `-threads`, `-filter_threads`, `-filter_complex_threads` or fixed codec thread count found in authored app/runtime command code. Pool sizes depend on CPU visibility, streams, filters, codec/library build and hardware driver. Two processes do not mean two Linux tasks. Per-process F includes all decoder/encoder/filter/muxer/library/helper threads. |
| Discovery/535 helpers | Current scanning/fingerprinting uses blocking tasks. The exact separate DISC proposal retains one active scan and two read-only fingerprint helper workers per instance; count both helpers and any implementation threads. They are not RVB1 lanes. All discovery/watch/schedule features remain included. |
| SQLx/listener | `config/loader.rs:173-203` sets migrator pool1, runtime pool8, both acquire timeout10s; `:560` uses PgListener::connect. Connection count is not a Linux-thread count, but invisible pool/listener ownership and server commit uncertainty remain settlement work. The proposed2s DB bound is not already implemented by these10s settings. External PostgreSQL processes are outside the application P1024 unit. |
| Logging/telemetry | Current `log_stream.rs:137-144` writes/flushed stdout on producers; not an existing isolated logger. Proposed S2 adds one owned writer. Optional OTEL uses batch exporter (`init.rs:265`); locked opentelemetry_sdk0.31.0 `trace/span_processor.rs:316` creates a dedicated thread. Include optional features rather than disabling them to fit. |
| Proposed PID1 | Exactly one additional Linux task. It must branch before Tokio initialization; wrapping the current Tokio main with another Tokio owner would add unbudgeted threads. |

Locked Tokio source is retained by hash in provenance, including its W selection
at builder.rs:1859 and `loom/std/mod.rs:85-103`. Upstream libtorrent documents
defaults aio_threads10 and hashing_threads1, but these are backend settings,
NOT a total L or proof of the deployed binary. [Libtorrent settings](https://libtorrent.org/reference-Settings.html).
FFmpeg documents CPU-dependent filter pools; codec/stream-specific counts remain
unmeasured. [FFmpeg threading options](https://ffmpeg.org/ffmpeg.html#Advanced-options).

For the intended full topology, write the budget as
`P = 12 + W + B + L + F_control + F_job + X <= 1024`, with `B <=512`.
The fixed12 comprises owner1 + app main1 + managers2 + brokers2 + native leaders2
+ read-only helper leaders2 + proposed log writer1 + optional OTEL thread1.
F excludes the already-counted tool leader; X includes all remaining helper,
watcher, library, driver, allocator and transitional threads, with no default0
assumption. App blocking threads used for a task are charged in B, not twice.
After reserving all512 blockers, 500 slots remain for W+L+F_control+F_job+X.
An illustrative W32/L32/F128+128/X32 mix totals876, leaving148. Those numbers
are arithmetic examples, NOT measured maxima or selected per-subsystem caps.

This is why P32 is unusable as a defensible release envelope and why P1024 is a
useful next qualification constraint. There is no source-derived universal
upper bound with automatic codec threads, unbounded CPU visibility/stream
fan-out and current admission gaps. Increasing the cap or changing CPU/thread
settings requires an exact approval delta and fresh qualification; not this run.

## Failed V2 Linux Experiment

Immutable existing image:
`sha256:da350429de6619c4172e0d3a7be5173fb9a0ee904028b0d9f1408924749631a7`,
Linux arm64, kernel7.0.12-linuxkit. This historical image is a laboratory
environment, not a package of the pinned source. No Revaer/FFmpeg/libtorrent
execution, media fixture, real DB or real filesystem mutation was run.

Constraints read back before start: private PID namespace (PID1 verified),
private cgroup namespace, read-only rootfs, networknone, capsALL dropped,
no-new-privileges, nonroot65534:65534, IPCnone, pids.max1024, memory1GiB,
memory+swap total1GiB (memory.swap.max0), scratch tmpfs16MiB with
noexec/nosuid/nodev, log drivernone, no healthcheck. `/evidence`, `/config` and
`/data` were read-only owned binds; the latter two masked declared volumes.
No Docker socket/host PID/host cgroup mount. One CPU quota was a lab pressure
condition, NOT a production CPU/thread setting or proof of owner isolation.
Normal runtime `/dev`/masked-proc pseudo-mounts were not scratch/test storage.

Topology: PID1 + one leader +1,022 new-session TERM-ignoring grandchildren.
All descendants ran non-yielding loops after a common gate. Each grandchild
wrote one21-byte tmpfs counter. Exactly1,024 tasks were counted, and task1,025
failed EAGAIN. The test deliberately retained full occupancy until whole-unit
KILL: it did NOT rerun a native5s phase or identify these Perl tasks as native
library threads. Existing native/group-escape controls remain separate.

| Measurement | Actual observation |
| --- | ---: |
| Setup to t0 | 0.491008876s |
| Initial / pre-force task count | 1,024 /1,024 |
| Writers with positive counter | 1,022 of1,022 |
| Memory peak | 590,876,672 bytes =563.503906MiB |
| OOM / OOM kills | 0 /0 |
| Whole-unit KILL target | t0+29.000000000s |
| Actual KILL request | **t0+34.797679723s** |
| Dispatch lateness | **5.797679723s** |
| KILL syscall return latency | 0.000732000s; signal request only |
| Final census attempt | t0+35.096974390s |
| Collected descendant wait statuses | **0 of1,023** |
| Remaining TID entries / pids.current before PID1 exit | **1,023 /1,024** |
| Container result | Exit2, OOMKilledfalse, later Docker state stopped/Pid0 and exact removal confirmed |
| Qualified settlement time | **NOT OBSERVED** |

Important raw-schema correction: envelope-result.json names timestamps
`all_reaped_at` and `absence_observed_at`, and elapsed fields
`kill_to_reaped_seconds`/`kill_to_absence_observed_seconds`, even on its failed
path. Here those mean the final *attempted* check at 0.291651167/0.299294667s
after KILL, NOT reaping or absence. `settled=false`, empty `reaped`, remaining
TIDs and exit2 are authoritative. Its `unclean_forced_containment` label means
force was attempted; it does not certify completed containment before exit.

The harness's25s counter census completed only at t0+32.389305264s; its subsequent
cgroup snapshot further occupied the deadline loop. The loop also uses the
iteration's earlier time after sampling. CPU cgroup throttling was present in
all356 observed periods. This confounds instrumentation cost and scheduling:
the run is evidence that this owner pattern misses the deadline, NOT a measured
kernel-only scheduling bound. Its five-second diagnostic extension was already
almost exhausted when KILL was finally requested; no successful tail was
obtained. Diagnostic time is never extra application budget. Preserve these
defects and observations. The original harness/result are not edited or rerun;
the separately authorized correction below addresses this identified violation.

The last namespace snapshot cannot distinguish every remaining task's live vs
exiting/zombie state. PID1 exit triggers namespace teardown, and Docker stopped
readback/removal confirms owned resource cleanup, but there was no independent
host cgroup.events unpopulated observation or collection of all wait statuses.
[Linux PID namespace semantics](https://man7.org/linux/man-pages/man7/pid_namespaces.7.html),
[Linux cgroup semantics](https://docs.kernel.org/admin-guide/cgroup-v2.html).

## One Authorized Corrective Case

Exactly ONE corrective run, not a sweep, using the same image, P1024,1GiB,
16MiB tmpfs, private PID/cgroup namespaces,1CPU laboratory quota, read-only binds,
nonroot/no-capabilities/networknone/security settings and descendant workload.
`corrective-one-run.json` recorded source hashes,28s/2s, phase ceilings and
single-use authorization BEFORE creation. No build/pull/prune/resource increase.
Same topology: PID1, leader,1,022 setsid TERM-ignoring counter writers. Full
pre-origin census and the1,025th EAGAIN establish occupancy; no pre-cutoff census,
counter or cgroup instrumentation occurs after t0. Children retain the same CPU
loops and tmpfs counters, read only after they have all been reaped.

`corrective-owner.pl` uses three preallocated1,023-slot arrays and fixed scalars,
no growing per-event list in the deadline path. That path performs clock,
signal, bounded16-result wait batches and1ms sleeps only. Current time is read
immediately before action. Full diagnostics/JSON/file reads run only after
affirmative complete waits. Perl heap overhead is bounded by cardinality but
this is NOT proof of the proposed Rust128KiB registry allocation or full codec.

| Measurement | Actual corrective observation |
| --- | ---: |
| Setup to t0 | 0.493061042s |
| Initial tasks / attempted task1,025 | 1,024 / rejected EAGAIN |
| Cutoff / actual owner wakeup | 28.000000000s /28.076089054s |
| Actual raw KILL request / return | 28.076101429s /28.077386596s |
| Dispatch / syscall duration | 0.076101429s /0.001285167s |
| Final successful descendant wait | 28.395392513s |
| Genuine ECHILD observation | 28.395399554s, errno10 |
| Signal-to-ECHILD phase | 0.319298125s, within1.500s |
| Namespace/cgroup absence confirmed | 28.396035429s |
| Classification complete | 28.396035721s |
| ECHILD-to-classification phase | 0.000636167s, within0.250s |
| Target-to-classification / reserve left | 0.396035721s /1.603964279s |
| Wait identities / statuses | 1,023 unique exact startup identities, every raw wait status9 (SIGKILL) |
| Remaining TIDs / pids.current | 0 /1 |
| Post-settlement positive writers | 1,022 of1,022, minimum counter450,560 |
| Memory peak / OOM | 595,759,104bytes =568.160156MiB /0 |
| CPU throttled periods | 289 of289 sampled after settlement |
| Repeated synthetic TERM action | Due15s, requested16.775503591s; original clock did not reset |
| Final outcome / container exit | Forced, UNCLEAN, absence proved within candidate /70 |

All time differences use the SAME Linux CLOCK_MONOTONIC origin. Host Docker
attach timing is not substituted. Startup and final diagnostics are not part
of the deadline loop; final absence/classification DO count inside30s. The
contingent extra5s diagnostic reap window was NOT used and is never success time.
The frozen exact harness hash is in corrective-one-run.json; argv, readback,
runtime Perl5.42.2/kernel7.0.12-linuxkit and complete waits are retained.

Signal/wait semantics were checked against primary documentation and the case:
positive Perl signal with PID-1 is a Linux broadcast, unlike a negative SIGNAL
which requests process-group semantics. Thus v2 is not evidence of a Perl
targeting bug. The corrective case explicitly invokes Linux arm64 syscall129
as `kill(-1,9)`, with signal-zero/self/namespace permission checks before t0.
Perl signal-zero returns1 successful argument; raw syscall returns0 success,
neither means1,023 processes have stopped. Linux excludes PID1 and the caller
from that broadcast. Exact waits and final absence provide the missing proof.
[Perl kill](https://perldoc.perl.org/functions/kill),
[Linux kill](https://man7.org/linux/man-pages/man2/kill.2.html),
[arm64 ABI](https://raw.githubusercontent.com/torvalds/linux/v6.6/arch/arm64/include/uapi/asm/unistd.h),
[syscall129](https://raw.githubusercontent.com/torvalds/linux/v6.12/include/uapi/asm-generic/unistd.h).

SIGCHLD was explicitly DFL with no SA_NOCLDWAIT, not IGN. Observed flags67108864
are the Linux restorer flag, not automatic reaping. WNOHANG0 never means absence;
ECHILD was retained with errno10 only after1,023 successful waits. Orphaned
grandchildren were adopted by namespace PID1. This case proves its complete
finite process set, not native threadgroup or external-storage settlement.
[Linux wait](https://man7.org/linux/man-pages/man2/wait.2.html),
[Perl waitpid](https://perldoc.perl.org/functions/waitpid).

Conclusion: the identified instrumentation violation is corrected and actual
28s/2s mechanism settlement is observed at the candidate cardinality. The
1.7755s repeat-action lateness remains scheduling counterevidence; no universal
wakeup, native5s, integrated-task, storage or lease-safety guarantee follows.

## Phase And Ownership Contract

Let t0 be the first owner-observed TERM/INT/fatal drain. Keep D_app=t0+30s and
the candidate D_force=t0+28s immutable. Every operation is clipped by the earliest
request, startup/recovery incident, cancellation and safe lease expiry. Duplicate
signals, late registration, queues, logging, retry and cleanup cannot reset it.
Use one Linux monotonic clock across owner/application, not serialized Rust
Instant or comparisons with the macOS command clock.

At t0, fail readiness and close engine/native/spawn/media admission atomically;
signal API, watcher, discovery, retention, indexer/import and resumable job pause
concurrently. Discard no ownership receipt just because its producer was aborted.
Configuration admission remains128 pending +1active +1preparation. Cancel queued
unstarted config commands as typed `not_started_shutdown`; active native apply
requires its actual acknowledgement. No applied revision advances after partial
preparation/application. The previous complete revision remains authoritative.

Native TERM starts at the original cancellation phase. The proposed owner wakeup
at4.750s reserves250ms dispatch to meet the unchanged5s KILL ceiling. Earlier
request deadlines shorten it. A missed native/control/hash phase is a real miss,
not a new grace. Request KILL for exact registered groups and verify identity,
wait/group/pipe settlement. Session escapes remain inside the private unit.
Ownership/containment uncertainty invokes earlier whole-unit failure, not a wait
until28s merely to reproduce this fault-injection experiment.

Before28s, settle all producer/worker/alert/fast-resume/session/internal native
work, blocking FS scanners/finalizers, listener and every owned pool. A watcher
join, empty queue, dropped SQLx future or successful native enqueue proves too
little. Current worker handle omission must be corrected. No detached blocking
work or independent task restart. Final logger flush/join shares this budget;
it has no extra grace after other tasks finish.
If choosing automated planned restarts, RECOVERY.md's NEW graceful handoff is
published by the application only after irreversible mutator/entire-broker
settlement and owner opcode14, BEFORE DB close and28s. It is not final-phase
work, a durable PID1 latch, or evidence manufactured after a crash.

At28s, missing settlement latches unclean and starts complete contained-unit
enforcement. During the2s reserve, only signal/exit/reap/absence observation and
bounded classification remain, not a new DB transaction, retry, log flush,
replacement or root release. Required acceptance ceilings: request by28.250s,
all contained mutators gone/reaped by29.750s, final observation/classification
by30.000s. Force stays unclean even when early. A miss/unknown remains failed
and root-blocked. These are proposed acceptance conditions, not OS guarantees.

The exact **64-byte control protocol, two pipe bounds,14 opcode/latch rules,
registration ACK versus verified-settlement distinction, signal fallback, record bounds and exit
codes are in CONTROL.md**. They are part of this same S2 approval delta, not
"choose a codec later." The owner never executes application futures, DB/media
I/O, stdout writes or arbitrary formatting. There is no third RVB1 lane.

Root quarantine is a safety state, not proof of durable storage. **RECOVERY.md
specifies same-owner automated recovery, NEW durable graceful handoff/one-time
successor consumption, and operator confirmation when proof is missing/uncertain.**
All paths require550/557 physical root authority and full512 recovery. Without
approval/implementation of the graceful handoff, v2's operator-on-EVERY-restart
availability cost remains, including clean planned rollout. No claim of current
unattended planned restart. PID1 writes no DB/media/death record. Old logs, exit
codes, catalog digests, expired leases or volatile latches do not authorize reuse.

## Shared Deadline And Retry Reconciliation

Parent LIFE-1 is authoritative as a proposal, not a production default; its
exact section is `source-evidence/parent-LIFE-1.md`. The FINAL DISC table is
`source-evidence/discovery-final-CANDIDATE_VALUES.md`, imported from the user's
preserved decision-evidence-b path, not the removed worktree. Older DISC copies
remain historical. Even the final DISC table's `shared fatal/transient restart`
phrase conflicts with LIFE-1: S2 explicitly uses transient infrastructure ONLY
and LIFE-1's60s cooldown/half-open rule, never fatal/process restart retries.

| Quantity | Exact composition / consequence |
| --- | --- |
| Shared512 attempt/aggregate,513 recovery/cleanup,514 lifecycle ownership | H5s/L40s; separate server-time fenced identities, no ownership collapse |
| Shared conditional safety inequality | H + J + T_db + U + S =5+.25+2+.25+30=37.5s <40s; margin2.5s, **conditional on actually settling in30s, not established here** |
| Read-only discovery/fingerprint | H5s/L20s; same J250ms, DB2s, U250ms; S_read7.5s gives15s <20s, margin5s |
| S_read7.5s components | stop dispatch.25s + bounded CPU.25s + existing outer/native allowance5s + checkpoint2s. A reader stuck beyond this violates this conditional envelope; do not replace it with30s while keeping L20s. |
| DB operation | End-to-end2s, including acquisition/network/transaction/response; statement<=2s, lock<=250ms **inside**2s, not2.25s. Uncertain commit requires stored-procedure readback. |
| Hash cancellation |100ms cooperative grace, then KILL request and at most2s settlement, all clipped to earliest request/app/recovery/safe-expiry deadline. The2.1s is inside S_read, native/whole-app budgets, never added after them. Late digest loses to prior revocation. |
| Long hash request | FINAL DISC19,860s initial /79,260s hard total request wall; immediately clipped by drain/cancellation/lease/incident. The former630/2,400s values are superseded, not altered historical measurements. |
| Useful full aggregate bounds | Primary256GiB/1TiB; complete aggregate260/1,040GiB; principal rolling-hour8/32TiB and deployment64/256TiB. These do not change the earliest shutdown clock or permit new hashing during drain. Full DISC table owns the remaining values. |
| Claim/control/cleanup/retention | Existing1s claim,250ms active control,1h cleanup sweep,24/720h retention and128cleanup candidates unchanged. Failed cleanup eligibility waits next1h sweep. Fairness cursor until root removed AND all references settled. |
| Transient infrastructure burst | Initial attempt +5 retries, waits1/2/4/8/16s after failure,0jitter, one in-flight control attempt |
| Exhausted burst | Stay recovering/degraded, cooldown60s, one bounded half-open attempt; continued failure repeats60s cooldown, not a new five-retry burst |
| Reset | Only after60s continuous pertinent healthy readiness and successful5s renewals. Half-open success starts probation; relapse before60s returns to cooldown. No permanent manual-only rule for >31s DB outage. |
| Exclusions | Fatal, B1, B3, drain, expired caller work, unclean containment, unknown-root quiescence never enter that automatic transient loop. No new process restart semantics. |

Renewal cadence is not a process-death detector; expiry alone never permits
mutation, destructive takeover, cleanup or uncertain physical-slot reuse.
Before a work step require remaining authority greater than uncertainty plus
that step's settlement allocation. Derive conservative local safe-expiry from
bounded server-time observations; do not subtract unrelated host clocks or
assume U250ms under unmeasured time uncertainty. If the DB2/J250/U250 assumptions
fail, authority closes. Safe successful renewal may maintain an existing lease;
it cannot extend request, incident or shutdown clocks.

The P1024 evidence does not establish J250ms, S_read7.5s or integrated S30s.
The failed v2 deadline loop and corrective repeat-action delay cannot be turned
into an approved larger J while keeping these inequalities.
Uncertain readers retain physical capacity after lease expiry; failed shared
settlement leaves root barriers closed. This is availability loss, not corruption
permission or a relaxed takeover rule.

RECOVERY.md includes precise cooldown/reset behavior, required outage acceptance
and the finalized LIFE-1 initialization: after existing startup succeeds, each
new permitted recovery owner receives its initial attempt plus one five-retry
budget. State is volatile/per-owner lifetime, so a new incarnation gets a fresh
burst; this is NOT a deployment-wide restart-rate bound or restart mechanism.
It cannot inherit/revive old callers/claims; fresh fencing and quiescence remain
mandatory. No new60s startup delay or B1/B3 retry. Within a live owner, reset
still requires60s continuous health/5s renewals. V2's unknown-history cooldown
suggestion is superseded by this complete parent rule; no retry ledger/schema.

## Other Held S2 Numerical Values

All original values below are retained byte-for-byte in limits.json. None became
approved through the P1024 experiment. Existing30 model checks were read, not
rerun; their scope is independent arithmetic/state models, not Rust/HTTP proof.

| Surface | Exact candidate |
| --- | --- |
| Blocklist body |16,777,216 wire bytes AND16,777,216 decoded bytes, actual streamed counts; byte+1 rejects without partial publication |
| Physical line |4,096 bytes excluding LF, including CR; overlong across chunks rejects |
| All physical lines |1,048,576 including blanks/comments/invalid/duplicates; raw work counted before dedup |
| Parsing quantum |4,096 bytes; cancellation checked before each quantum and bounded per-line work; production slices input |
| Downloaded unique rules |Existing100,000 ceiling; no implicit widening of inline/merged policy |
| Log fields |At most16 static scalar fields; string<=1,024 UTF-8 bytes; static key<=32 ASCII bytes; bounded64-bit integers; no nested/custom Display/Debug |
| Encoded record |4,096 bytes including LF; escaped JSON/output size charged before overflow, not only raw strings |
| Logger active+pending |1,024 records AND4,194,304 payload bytes; active write remains charged until settlement |
| SSE history |1,024 records AND4,194,304 bytes, plus existing120s maximum age; explicit oldest-history eviction, not silent durable logger loss |
| Engine command count |Existing128pending, plus1active and1config preparation |
| Engine payload |16,777,216 bytes one owned payload;33,554,432 total preparing+pending+active; actual capacities/nested allocations charged across ACK/cancellation; no uncharged clones |

Input size/format/capacity failures are typed rejection and retain the last fully
applied revision. Logger overflow/formatter/sink/incomplete flush failure latches
telemetry failure and unclean drain, with a bounded independent reason. Do not
recursively log a broken logger. Log an originating substantive error once;
propagate typed primary error plus bounded secondary timing/cleanup evidence.
The existing model's count-only engine case has no active item; it does not
prove production's128+1layout, allocator charging or real decoder bounds.

## External45s And Kernel Limits

The accepted external supervisor has its own origin. Signal delivery/setup
consumes the15s nominal difference between45 and30; it must not restart grace
when the application observes a delayed signal. Keep no blocking preStop hook
or implicit extension. Current chart values have no termination-grace declaration
and `resources: {}`; Dockerfile launches the app directly, not this owner. Package
wiring is an unimplemented adoption delta, not inferred from an accepted ADR.
[Kubernetes termination flow](https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#pod-termination-flow).

At external expiry, the deployment owner terminates a stuck PID1/unit and keeps
roots blocked. Neither Docker stopped state, signal delivery, exit137, nor45s
elapsed proves DB commit outcome or old-root quiescence. No fresh retry budget
or cross-incarnation clean result follows. DISC's conditional1s handoff +30s
application +10s external settlement =41s <45s is an allocation, NOT measured
45s proof; those10s are outside application success and cannot replace its2s
candidate reserve. External/lost-host evidence uses RECOVERY.md's boundary.

Userspace scheduling, CPU throttling, reclaim, uninterruptible kernel I/O and
host/storage failure preclude a universal Linux wall-clock guarantee. A finite
task count bounds cardinality, not these latencies. The proposed owner shares
kernel/cgroup fate despite escaping the application's Tokio/logging failure
domain. No real-time CPU reservation, priority/affinity/thread tuning, new host
watchdog or storage fence has been silently selected.

## Exact Integrated Acceptance

No acceptance run below was performed. Parent must integrate them, along with
unchanged `just ci`, `just ui-e2e`, strict Sonar and complete full-spec evidence:

1. Build the actual approved same-binary owner and both amd64/arm64 packages.
   Pin source, image/native-library digests, CPU visibility/quota, all thread
   settings, memory and supported filesystem topology. Exercise every optional
   feature, both RVB1 lanes, full admitted codecs/streams/hardware, libtorrent
   hashing/I/O, SQLx/listener/API/SSE load, two535helpers, watcher/scan, retention,
   replacement/recovery, OTEL and full log limits. Count `/proc/PID/task` and
   cgroup tasks including short-lived peaks. Never substitute async-future counts.
2. Prove full-spec admission fits <=1,024 total tasks with512blocking-thread
   potential accounted, no hidden per-tool thread/CPU caps and no missing native
   feature. Use a qualifying integrated stress scenario at exactly1,024 tasks;
   a1,025th attempted creation must reject before mutating authority. If a legal
   workload does not fit, candidate fails; request a new explicit envelope/tuning
   decision rather than narrowing the full-spec test matrix.
3. Test ordinary cooperative clean exit AND blocked writer/formatter/command,
   native call/destructor, helper, escaped descendant, listener/pool and startup
   failure. At final enforcement, measure target-to-request<=250ms,
   request-through-reap<=1,500ms, final absence/classification<=250ms, all within
   the original30s. Native KILL request<=5s with the4.750s wakeup; earlier clocks
   win. Retain every phase miss and failed run. At least30 independently started
   integrated trials per architecture for each final-owner stress condition,
   with zero timing/absence failures, is the proposed finite acceptance set;
   this is not a statistical hard bound. No trial result here satisfies it.
4. Separate uninterruptible-I/O, lost-host and stalled-PID1 cases: prove failure
   classification/quarantine, external45s behavior and operator-required recovery,
   not impossible universal timely reaping. Verify exact unit identity and
   manager-owned cgroup-unpopulated evidence where available without adding an
   app Docker socket/host scan. Observation must not itself delay enforcement.
5. Test CONTROL.md bytes, inherited-FD closure, full-pipe signal fallback,
   post-drain admission races, wrong identity and receiver death. Test all held
   body/line/decoded/log/queue boundaries using actual Rust paths, not models.
6. Prove DB2s/lock250ms/J250ms/U250ms, S_read7.5s, hash100ms+2s clipped deadlines;
   lost commit ACK/stale fence/lease-renew failure and old-descriptor mutation
   risks. Test RECOVERY.md's prolonged-outage/cooldown/half-open/reset cases with
   one control attempt and no request resurrection. Neither20s nor40s expiry
   alone may release root or unknown worker occupancy.
7. Crash at every recovery/graceful-seal/publish/consume/confirmation/ACK boundary.
   Same-owner proof and valid one-time durable graceful handoff permit automatic
   full512 recovery. Missing/uncertain proof defaults closed without a durable
   quarantine flag; old nonce/generation/consumed receipt cannot replay. Prove
   routine valid planned rollouts need no operator, while unknown-loss cases
   require exact550/557 operator confirmation. Full manifest, checkpoint,
   finalization and complete outbox reconciliation stay mandatory. Preserve all
   originals/protected attempt generations and clean exact test resources.

The30-trial integrated acceptance cardinality and phase allocations are NEW
qualification requirements, not approval to perform more runs in this task.

## Task Record And Delivery

- Motivation: replace an unusable P32 release-envelope inference with actual
  contributor accounting, preserved failed larger case, one authorized correction and
  honest release barriers.
- Design: proposed cooperative ownership + nonmutating PID1, P1024 admission,
 2sconditional final phase, exact control codec, explicit graceful handoff and
  lost-evidence operator confirmation; full recovery only with valid proof.
  Architecture, protocol, admission, logging and timing choices remain held.
- Test summary: original nine Linux scenarios/30models preserved; one FAILED
  larger P1024 case plus exactly ONE specifically authorized corrective case.
  The latter passed its finite mechanism ceilings, not release qualification.
  Focused syntax/source,
  original-byte, result-consistency, ownership and checksum verification only.
- Observability: separate requested signal, returned signal, wait result, absence,
  attempted observation, phase miss, aggregate miss, unclean outcome and blocked
  root. Raw misleading timestamp names are explicitly corrected above, not edited.
- Risk/rollback: no production behavior to revert. Reject/remove only owned
  evidence if unwanted. Any eventual implementation rollback must keep roots
  closed and protected recovery artifacts when predecessor quiescence is unknown.
  P1024 availability, early force, new bounded logging, graceful durable record
  and operator-required unknown-loss recovery are explicit approval deltas.
- Dependency rationale: existing Ruby/Perl standard libraries and immutable
  image only. No production dependency or lockfile change. Existing safe Linux
  adapters still need scoped Rust/FFI review before implementation.
- Stale policy: read pinned root, Rust, FFI, data and devops instructions and
  relevant501/512/513/514/535/550/554/557/558/559/588 contracts. Supplied root text
  differs from pinned root in task-record/Sonar wording; preserve both scopes,
  no tracked policy/ADR edit authorized here. Record remains ignored and parent
  owns ADR588. No scoped source or quality gate was weakened.
- NOT RUN: `just ci`, `just ui-e2e`, Sonar, current-source package/native runtime,
  DB/lease/outage/recovery tests, HTTP decoder/logger stress, real media/FS
  mutation, hardware/native thread census, new codec, cgroup-host absence proof,
 45sfull-package trial or kernel-uninterruptible fault. User forbade builds and
  additional services; both finite run allowances are consumed. Mandatory gates
  remain parent-owned and incomplete. No further experiment is authorized.

`COMMANDS.md`, `provenance.json`, `final-provenance.json`, `verification.json`,
`corrective-assessment.json`, `artifact-manifest.json` and `SHA256SUMS` are real
artifacts. The v2 assessment/ledger stay historical; final verification covers
both cases and archive integrity. `corrective-commands.jsonl` adds exact argv,
config, output, waits and cleanup. Both one-run guard files remain consumed.

The exact containers `revaer-s2-p1024-8920eb1c2190` and
`revaer-s2-corrective-433007b19a53` were stopped and removed with owned volumes;
all11 historical generated names are also checked absent (13 exact names total).
No media was created. The1,022counter files (21,462bytes logical), gate and other
ephemeral state vanished with tmpfs. All host CLI children were waited. The
evidence worktree remains for parent copy/preservation/removal; no unrelated worktree,
Docker cache, image or user resource was touched. Decision evidence is finished;
S2 production implementation, approval and release readiness are not.
