# Exact DISCOVERY/FINGERPRINT Candidate Values: v2

> Current decision: accepted [ADR 594](../594-simple-discovery-worker-cleanup.md)
> replaces the named traversal-frontier, pre-release upgrade, distributed quota,
> lease and quiescence requirements in this historical record. Other discovery
> functionality and applicable bounds remain; do not restore replaced machinery
> or its acceptance tests as implementation prerequisites.

> Current approval: The operator accepted ADR 588's exact applicable choices
> on 2026-09-11 at reviewed commit `9575c077`. The [resolution](../588-first-release-decision-package.md#approval-resolution)
> releases only those named design holds. Earlier pending/candidate wording
> below retains its historical context; implementation and qualification are
> not certified, and all other conditions remain binding.

Status: PROPOSED ONLY, 2026-09-11. No production default, activation, shared
lease acceptance, or criteria change. Source
`fcdd95596fa062de9388d1be4f2ac96bbff571d1`. Parent incorporates the decision in
ADR 588 as DISC-1 through DISC-7, not the database D1-D5 decisions. This is the
single bounded useful-workload revision of the sealed history/v1 package.
This is the stable numeric tuple for reconciliation with S2; Linux
containment/enforcement proof belongs to the independent S2 agent.

`I` is the proposed initial policy value; `H` is the absolute candidate ceiling
for an explicitly versioned operator-selected policy. Increasing I toward H
is an explicit, coherent policy-version action, never independent field tuning.
Equal I/H means fixed in this candidate. GiB/TiB/MiB are binary; workload Mbps
means exactly 1,000,000 bits/s. Time limits revoke
authority; they are not guarantees that kernel I/O, KILL, or reaping returns.
Zero-sized regular files are allowed; limits themselves are positive.

Evidence keys: `F` real shallow/wide/deep filesystem lab, `R` fresh-process
relational-row resume/mutation model, `H` real descriptor-inherited hash child,
`A` bounded admission/fairness model, `B` finite resource arithmetic, `C` accepted
contract preserved. B includes conservative policy judgments, not measurements
of the corresponding maximum library size or deployment. Exact measurements
and limitations are in REPORT.md and results.json. v1 measurements are retained,
not repeated or relabelled as measurements of v2. New evidence is arithmetic only.

## Named Useful Workloads And Scope

The complete MEDIA_TRANSCODING.md was read at the pinned source. Its full-release,
multi-video, archival, Plex 4K/remux-first, audio, attachment and existing-subtitle
scope motivates these workloads; it supplies NO universal bitrate/duration/size
ceiling. The following are explicit sizing assumptions, not observed media or
codec-standard maxima. Admission uses exact file sizes, not inferred bitrates.
They do not limit the included feature/codec matrix to a small fixture subset.

| Named workload | Exact sizing inputs | Primary arithmetic | Proposed primary cap |
| --- | --- | --- | --- |
| I: `long-4k-remux-4h` | 14,400 s; one 100 Mbps video plus four 6 Mbps audio tracks; 10% allowance for mux, embedded subtitles, attachments and metadata | `14,400 * (100 + 4*6) * 1,000,000 / 8 * 11/10 = 245,520,000,000 B = 228.65831851959228515625 GiB` | 256 GiB = 274,877,906,944 B; 29,357,906,944 B headroom beyond that allowance |
| H: `dual-video-4k-archive-8h` | 28,800 s; two 120 Mbps video tracks plus four 8 Mbps audio tracks; same 10% allowance | `28,800 * (2*120 + 4*8) * 1,000,000 / 8 * 11/10 = 1,077,120,000,000 B = 1,003.14617156982421875 GiB` | 1 TiB = 1,099,511,627,776 B; 22,391,627,776 B headroom beyond that allowance |

For existing external image subtitles, allocate four large physical payloads at
0.5 Mbps for 4 h (900,000,000 B each) initially and 1 Mbps for 8 h
(3,600,000,000 B each) at H. Select 1/4 GiB per sidecar and 4/16 GiB total:
four payloads leave exactly 694,967,296 / 2,779,869,184 B for VobSub indexes,
text sidecars and remaining selected groups. Embedded streams are already in
the primary. These subtitle rates are allowances to qualify, not measurements.
The 32/64 logical-group and 65/129 physical-member bounds still apply; all
members cannot simultaneously take their individual byte maxima. Exceeding
any combined bound rejects the whole aggregate, never an omitted subtitle.

Thus exact request caps are **260 GiB = 279,172,874,240 B** and
**1,040 GiB = 1,116,691,496,960 B**. Longer, more heavily multiplexed or larger
files are explicitly outside this proposed envelope, not silently supported by
the phrase "full system". The full specification remains the implementation
scope. These are useful initial/hard admission proposals, not support certification.

## Scheduler And Traversal: ADR 516

| Value | I | H | Enforcement and evidence |
| --- | ---: | ---: | --- |
| Association schedule interval | No default | 1-43,200 minutes OR 1-720 hours | Must be explicitly selected; flags remain off. C/B. |
| Durable due/claim polling | 1 s | 1 s | One scheduler opportunity, not a promise of start within 1 s. A/B. |
| Due-association page | 32 | 128 | Keyset continuation; never truncate the total eligible set. B/C: accepted association cap is 128 per profile. |
| Continued-scan eligibility | Next fair opportunity, no cadence wait | 1 s poll | A batch does not wait for the association's next interval. R/A. |
| Active discovery scans, deployment | 2 | 2 | Durable global tokens; not the accepted media-job concurrency. A/B. |
| Active discovery scans, instance | 1 | 1 | Bounded owned scan execution. F/B. |
| Active discovery scans, source attestation/root | 1 | 1 | Includes distinct nonoverlapping associations on that same slot. C/B. |
| Active discovery scans, effective policy version | 1 | 1 | Skip unavailable policy/root tokens fairly. A/B. |
| Pending runs per binding | 1 | 1 | A newer trigger merges into one follow-up request. R/B. |
| Deployment pending binding requests | 1,024 | 1,024 | Above cap: explicit capacity diagnostic and disabled new trigger admission, not dropped work. B. |
| Discovery claim renewal | 5 s | 5 s | Independent of media-attempt heartbeats. Conditional inequality below. A/B. |
| Discovery claim expiry | 20 s | 20 s | Server-clock ownership only; no physical token reuse on unknown old-reader absence. A/B. |
| Claim/renew/checkpoint transaction | 2 s | 2 s | End-to-end client deadline; proposed server statement timeout <=2 s and lock timeout <=250 ms. Late commits reconciled, never assumed rolled back. B. |
| Lease-contention retry | Next fair 1 s opportunity | 1 s | No busy-spin, no attempt consumption, no jitter. A/B. |
| Transient non-contention retries | 3 retries at 1, 2, 4 s | 3 | Absolute run/request deadline never reset; then operator-remediable pause. B. |
| Missed intervals represented | 1 coalesced recovery run | 1 | Store first/last missed due plus exact bigint count, not N historical jobs. R/B. |
| Fairness quantum | 1 batch | 1 batch | Durable principal/policy/root round robin; one eligible service per ring turn. A. |
| Fairness cursor lifetime | Until explicit binding/principal retirement | Same | No TTL reset that rewards restart; cursor is one scalar service ticket, not an accumulating history. A/B. |
| Batch processed entries | 256 | 1,024 | Count all consumed frontier rows, including rejected/unmatched entries. F/R. |
| Batch selected primary files | 64 | 256 | Stop before adding the next complete candidate. F/B. |
| Batch selected primary observed bytes | 1 TiB | 4 TiB | Four maximum-sized primaries fit. Metadata totals, not bytes read. Yield BEFORE the next candidate exceeds the remaining batch; a fitting file is never permanently rejected for a full batch. B. |
| Batch selected aggregate observed bytes | 1,040 GiB | 4,160 GiB | Four complete 260/1,040 GiB aggregates; sidecars count as well as primaries. B. |
| Batch elapsed authority | 2 s | 5 s | Metadata execution/checkpoint quantum, not full hashing latency. Persist a pending candidate, yield, then publish complete identity in a later fenced quantum. Unknown physical work remains owned. F/B. |
| Descended directory depth | 32 | 64 | Association root is depth 0; record, do not silently skip, a child exceeding the selected bound. F. |
| Per-directory enumerated entries | 4,096 | 16,384 | All names, including unrelated files and rejected kinds; inspect at most cap+1 to detect overflow. F. |
| Per-directory raw name bytes | 1 MiB | 4 MiB | Counts bytes before decoding/retention; independent of entry cap. B. |
| Directory staging transaction rows | 256 | 256 | Incomplete epochs never publish; indexes required. R/B. |
| Per-run total examined entries | 65,536 | 262,144 | Count rereads and failed enumeration work too; no work hidden by deduplication. F/B extrapolation. |
| Per-run total directory visits | 4,096 | 16,384 | Includes repeated staging epochs; root counted. F/B extrapolation. |
| Per-run selected primary files | 16,384 | 65,536 | Stop before another selected file exceeds the cap. Exact-cap runs may finish only after clean EOF; otherwise incomplete/paused, never a manufactured absence census. F/B. |
| Per-run selected primary observed bytes | 16 TiB | 64 TiB | Retained, not shrunk. 64 maximum-sized primaries fit at either envelope; count and entry bounds remain independent. B. |
| Per-run selected aggregate observed bytes | 16.25 TiB | 65 TiB | Exactly 64 maximum complete aggregates. Includes all sidecars, even for cached observations. No Cartesian promise that every file/group/size maximum can coexist. B. |
| Per-run admitted fingerprint bytes | 16.25 TiB | 65 TiB | Includes failed/cancelled/retried discovery requests; no refund. Independent from observed-byte counters and downstream job reservations. Exhaustion pauses an incomplete run. B. |
| Per-run cumulative metadata execution | 6 h | 24 h | Counts enumeration, staging, metadata/DB service and retries; not hash or quota waiting. Former short WALL values are now a separate active-work budget. B. |
| Per-run wall age | 21 days = 1,814,400 s | 84 days = 7,257,600 s | Includes all queue, hash, quota, pause, downtime and metadata time; never reset by renewal/retry. Explicit new run after exhaustion; unresolved census cannot tombstone. See cold-library arithmetic below. B. |
| Live frontier entry rows per run | 65,536 | 262,144 | Consumed rows pruned after atomic checkpoint; restrictive references preserve evidence. R/B. |
| Live frontier name/path bytes per run | 32 MiB | 128 MiB | Transactional byte counters, not estimated row-size guesses. B. |
| Open directory handles per scan owner | 4 | 4 | Retained root + current + transient next child + enumeration duplicate; only one enumeration active. Never persist a native readdir cookie. B/C. |
| Unstable-directory immediate retries | 3 | 3 | Backoff 1, 2, 4 s; then pause that subtree/run as incomplete. R/B. |
| Clean absence observations before tombstone | 2 complete clean passes | 2 | At least 1 s apart, same active binding/root; any uncertainty resets absence evidence. R/B. |

## Watcher And Membership: ADR 516

| Value | I | H | Enforcement and evidence |
| --- | ---: | ---: | --- |
| Watcher/debounce drain poll | 100 ms | 100 ms | Fixed service opportunity; unlike profile cadence this has an initial value. B. |
| Quiet debounce interval | 1 s | 1 s | Key is binding plus exact aggregate owner. B/R. |
| Maximum debounce residence | 5 s | 5 s | `min(last_event+1s, first_event+5s)` prevents perpetual postponement. B. |
| Watch event + debounce records per root | 256 | 256 | Shared pool, not 256 twice; reserve before retaining names. A/B. |
| Watch event + debounce records per instance | 1,024 | 1,024 | Full/invalid/uncertain events latch durable rescan; no silent loss. A/B. |
| Watch records per deployment | 4,096 | 4,096 | Database admission cap on pending work, not a native watcher backend guarantee. B. |
| Pending watch bytes per root / instance | 2 MiB / 8 MiB | Same | Both bytes and record counts enforced before copying. B. |
| Active watched associations per instance / deployment | 128 / 256 | Same | Explicit enablement capacity check. Does not alter 128 associations per profile or 256 catalog slots. C/B. |
| Overflow/uncertainty rescan rows per binding | 1 | 1 | Reason join rows; latest trigger sequence remains unsatisfied until a clean newer pass. R/B. |
| Logical sidecar groups per aggregate | 32 | 64 | Selected by exact ADR 450 snapshot rules; no implicit extension fallback. C/B. |
| Physical members per aggregate/request | 65 | 129 | 1 primary + up to 2 files for each logical VobSub group; independent group cap. B/KAT. |
| Bytes per physical sidecar | 1 GiB | 4 GiB | Named long image-subtitle allowance above; full identity or typed limit failure, never a prefix digest. B. |
| Total sidecar observed/read bytes | 4 GiB | 16 GiB | Includes both VobSub files and sidecars later removed by target policy. Full totals fit the request deadline. C/B. |
| Canonical member path | 1-4,096 UTF-8 bytes | Same | ADR 557 path grammar; no normalization/case folding. Non-Unicode candidate is an explicit diagnostic. C/KAT. |
| Canonical manifest envelope | 1 MiB | 2 MiB | Checked before allocation. 129 physical records at two 4,096-byte names need <1.1 MiB. B/KAT. |

## Fingerprint Admission: ADR 535

One request is ONE complete aggregate. A multiple-path API command submits
independently identified/reserved requests, at most 16 paths per command, with
explicit per-path outcomes; it cannot hide an unlimited fan-out in one request.
All authenticated operator, service and scheduler principals use these same
limits initially. Internal requests carry a stable classified service principal,
never the nil UUID or a new identity per request/root. Principal provisioning is
not a way to mint capacity. Global class/deployment caps remain authoritative.

| Value | I | H | Enforcement and evidence |
| --- | ---: | ---: | --- |
| Running fingerprint workers per instance | 2 | 2 | Independent ADR 535 read-only helpers; no third RVB1 lane or native-tool pool. A/B/H. |
| Queued fingerprint operations per instance | 8 | 8 | Queues retain bounded metadata, not all open members. A. |
| Running fingerprint workers per deployment | 4 | 4 | Durable slots survive replica loss until settlement; no per-replica quota multiplication. A/B. |
| Queued fingerprint operations per deployment | 16 | 16 | Admission includes all replicas/classes. A. |
| Running operations per principal | 1 | 1 | Count across every replica. A. |
| Queued operations per principal | 2 | 2 | Excess gets bounded rejection, not detached work. A. |
| Running operations per source root | 1 | 1 | Avoid competing reads on one root/storage binding. B. |
| In-flight requests per principal / deployment | 3 / 20 | Same | Derived running+queued caps; completing/uncertain reservations still charge occupancy until settled. A/B. |
| Simultaneously eligible principals | 32 | 32 | Further principals get explicit capacity rejection; ring contains only admitted principals. B. |
| Aggregates per request / paths per command | 1 / 16 | Same | Caller cancellation propagates to every owned request. B. |
| Physical members / logical groups | 65 / 32 | 129 / 64 | Same membership bounds above; atomically reserve exact counts. B/KAT. |
| Observed bytes per request | 260 GiB | 1,040 GiB | Checked primary+all sidecar sum; exact named-workload sizing above. B. |
| Bytes per primary member | 256 GiB | 1 TiB | Useful long 4K envelopes, not derived from the cached lab. Above selected cap: explicit size hold, no automatic escalation. B. |
| Bytes per sidecar / total sidecars | 1 GiB / 4 GiB | 4 GiB / 16 GiB | Both physical and aggregate bounds apply. B. |
| Actual bytes read per request | Exact reserved observed bytes, at most 260 GiB | At most 1,040 GiB | No second full hash at enqueue; each claim/pre-mutation revalidation gets its own reservation/debit. B. |
| Hash read chunk | 64 KiB | 64 KiB | Check cancellation before and after every read/update; retain no content buffer beyond the chunk. H. |
| Parent cancellation/deadline check | 5 ms | 5 ms | Plus every state transition; control check is not OS preemption. C/H. |
| Queued duration | 30 s | 30 s | Counts from reservation acknowledgement, included in request time; explicit retryable queue-deadline outcome. A/B. |
| Member execution duration | 18,000 s (5 h) | 72,000 s (20 h) | Every member shares the request/aggregate deadline; not this duration multiplied by member count. B. |
| Aggregate active hashing duration | 19,800 s (5.5 h) | 79,200 s (22 h) | All sequential reads, member setup and final-manifest validation count. No progress-based reset. B. |
| Total request wall duration | 19,860 s (5 h 31 min) | 79,260 s (22 h 1 min) | Starts before admission lookup. Active budget + 60 s for queue/control/DB/settlement, subject to all earlier clocks. B. |
| Principal rolling-window admitted bytes | 8 TiB = 8,796,093,022,208 B | 32 TiB = 35,184,372,088,832 B | All discovery/claim/pre-mutation phases, failures and retries debit. 31 maximum-sized requests fit per window, not a completion promise. B. |
| Deployment rolling-window admitted bytes | 64 TiB = 70,368,744,177,664 B | 256 TiB = 281,474,976,710,656 B | Eight principal shares; 252 maximum-sized requests fit per window. Physical worker/root limits still bound service. B. |
| Rolling-window duration | 3,600 s (1 h) | 3,600 s (1 h) | Exact server-time `(now-window, now]`; not clock-hour buckets. B. |
| Ledger cardinality per principal / deployment per window | 4,096 / 32,768 | Same | Per ONE HOUR; includes tiny/empty, failed and retried requests. 16,384/65,536 file ceilings consume 4/16 windows of count allowance on one principal before byte/service limits. B. |
| Retained debit rows per principal / deployment | 8,195 / 65,556 | Same | Hard physical ledger caps: two windows plus 3/20 potentially unsettled in-flight rows. No unbounded expired-row accumulation; ledger-capacity failure closes new admission if pruning lags. B. |
| Ledger pruning cadence / active deployment pruners | 3,600 s / 1 | Same | Only expired AND physically settled debits; generation-fenced DB-only cleanup, never root mutation. B. |
| Ledger pruning rows per transaction / batches per run | 256 / 257 | Same | At most 65,556 deletions/run; 257 pages cover the entire bounded ledger. Each transaction <=2 s, lock <=250 ms. B. |
| Ledger pruning run wall | 600 s | 600 s | 257 * 2 s = 514 s plus 86 s control allowance; earlier app/owner deadline wins. Failure retains rows and explicit diagnostic, not a quota bypass. B. |
| Reservation renewal / expiry | 5 s / 20 s | Same | Generation fenced, server time; expiry is not worker absence. A/B. |
| Capacity retry delay | 1 s | 1 s | Explicit Retry-After; fresh client attempt uses same idempotency key until disposition resolved. B. |
| Window-quota retry delay | `ceil(blocking debit prefix expiry-now)` seconds | 3,600 s | Minimum 1 s; satisfy BOTH byte and request-cardinality limits. Ledger-capacity failure uses the 1 s capacity opportunity, not a false expiry promise. A/B. |
| Automatic queue-deadline retries | 0 | 0 | Typed paused/retryable result; no repeated large debits behind a known long reader. Scheduler checks eligibility before reservation and retains its pending frontier candidate. B. |
| Automatic source-change retries | 2 retries, after 1 s then 2 s | 2 | Re-observe and reserve again; prior debit retained; never retry after caller cancellation. H/B. |
| Cooperative cancellation grace | 100 ms | 100 ms | Then KILL request; earlier deadline clips this to zero. This is NOT a new native-tool grace. H/B. |
| Post-KILL settlement allowance | 2 s | 2 s | Within earliest remaining application/lifecycle deadline; failure latches unavailable and retains charged occupancy. H/B. |
| Open member descriptors per request | 1 read-only member | 1 | Sequential members; metadata keyset revalidation via root owner. Never 129 simultaneously open FDs. B. |
| Worker progress / terminal output | 32 bytes / 1 KiB | Same | Parent reconciliation: progress at most once per 5 s, retain latest counter only; terminal carries exact final count/digest. No filename/content output; malformed/extra output rejects whole result. B. |
| Reservation/terminal diagnostic retention | 720 h | 720 h | Starts only after settlement and no live run/job/evidence reference. Separate from the prunable 3,600 s debit ledger; long-run identities and compact audit remain pinned. C/B. |

### Deadline And Qualification Arithmetic

**16 MiB/s = 16,777,216 B/s is an ASSUMED qualification floor, not a measured
minimum.** Qualify actual supported Linux amd64/arm64 package digests and each
claimed filesystem/mount/storage configuration using real, non-sparse sustained
reads, hashing and full membership at permitted load. Cold/warm behavior, CPU
load, two local/four deployment workers and concurrent permitted media work must
be included. Neither the cached 64 MiB v1 lab nor sparse metadata can qualify it.
If throttling, an earlier owning deadline or measured service cannot meet this
tuple, report that limitation and fail closed; do not silently shorten the
supported workload or increase deadlines at runtime. No package is certified here.

| Exact calculation | I | H |
| --- | ---: | ---: |
| Primary cap / assumed floor | 16,384 s | 65,536 s |
| Member budget minus primary read time | 1,616 s | 6,464 s |
| Aggregate cap / assumed floor | 16,640 s | 66,560 s |
| Active budget minus aggregate read time | 3,160 s | 12,640 s |
| Non-active request allocation | 60 s | 60 s |

The 60 s includes 30 s queue, 2 s admission DB, 2 s terminal DB, 0.1 s hash
cancel + 2 s settlement, and 23.9 s remaining control/reconciliation allocation.
Lock 250 ms is INSIDE each 2 s DB bound. These allocations are not extra time
after expiry: start cancellation early enough to settle within the earliest
deadline; unknown physical settlement retains occupancy and closes readiness.
No change to native inspection/transcode, application or broker deadlines follows.

### Hourly Quota And Ledger Retention

The final correction replaces the unsealed daily-quota draft. Quotas are burst
and abuse ceilings, not IO throttles or four-file daily allowances. At the
illustrative **600 MiB/s, NOT measured here**, a root reader moves
`600 * 3,600 / 1,048,576 = 2.0599365234375 TiB/h`, below 8/32 TiB/h.
Even counting a request's entire debit at admission, a rolling hour of serial
maximum-file reads contains at most 9 I or 3 H debits (2.28515625/3.046875 TiB),
so neither byte nor 4,096-request quota imposes an artificial wait. Four such
readers would still be below deployment quotas, but no four-reader storage
performance claim follows. Concurrency, root/principal exclusion, deadlines and
actual settlement remain the real execution controls. Budgets are ceilings:
a fast completed request does not wait out its maximum wall duration.

An expired debit is not physical settlement. Hourly pruning removes only rows
with `admitted_at <= server_now-3,600s` AND confirmed terminal settlement.
It deletes no referenced reservation, identity, job or compact audit. Those
retain admitted bytes/time/phase for audit independently of quota-ledger rows;
no external FK may require a debit row to outlive its window/settlement purpose.
Pruning is keyset-paged and transactional, capped as above, and ends with an
explicit incomplete diagnostic on failure/deadline. Time, expiry or pruning
never releases a worker/root token. Physical ledger caps apply even after a
missed hourly run or a DB outage; reject new reservations rather than grow rows
or erase unsettled debits. The unchanged terminal-evidence policy is not a
30-day live quota ledger. No DB implementation or pruning experiment ran here.

### Batch, Run And Workflow Reconciliation

At the file cap, a batch fits exactly four primaries/aggregates, not all 64/256
allowed file-count slots. A smaller candidate must satisfy all count, entry,
byte, directory and elapsed counters. The old 32/128 GiB batch caps cannot
contain a new maximum primary; v2 replaces them, with no single-file exception.
Hashing is outside the 2/5 s metadata quantum but INSIDE the run wall age.
Only one pending fingerprint request per run is retained; its frontier row is
not consumed until complete identity publication. No unbounded batch fan-out.

For one root and one stable principal, name the cold-library sizing workload
`64-cap-aggregate-cold-discovery`: 64 complete maximum-sized aggregates,
no cache reuse, retries, outages or competing jobs, available service opportunities,
and each request finishing within its selected total wall budget. It occupies
16/64 TiB primary and exactly 16.25/65 TiB aggregate. A finite rolling-window
calculation charging each admission and serializing the one root reader gives:

| Calculation | I | H |
| --- | ---: | ---: |
| 64 request walls before quota waits | 1,271,040 s | 5,072,640 s |
| Including 8/32 TiB per 1 h principal quota | 1,271,040 s (zero quota wait) | 5,072,640 s (zero quota wait) |
| Plus full 6/24 h cumulative metadata allowance | 1,292,640 s (14.9611111111 days) | 5,159,040 s (59.7111111111 days) |
| Proposed run wall | 1,814,400 s (21 days) | 7,257,600 s (84 days) |

Old 6/24 h walls allow only 337.5/1,350 GiB of raw reads at 16 MiB/s; primary
16/64 TiB alone would require 776.722962963 MiB/s on the sole root reader.
Even four deployment workers cannot accelerate one root. The old walls were
not credible cold-library completion claims. v2 does not split libraries into
smaller associations to hide this mismatch. Run ceilings remain independent
admission guards, not a guaranteed SLA for every simultaneous maximum or load.
Quota contention, all-small-file setup overhead, retries, outages, changed
directories and service pauses can still exhaust them with an explicit incomplete
outcome. Package qualification must measure these cases; no "complete" census
or tombstone may be manufactured at exhaustion.

Discovery, successful job claim and pre-first-mutation revalidation each need a
complete read. Without cache/retries these are three distinct debits: 780/3,120
GiB per maximum aggregate, and 48.75/195 TiB for the named 64-aggregate library.
The per-run fingerprint budget pays only its discovery work; job-phase reads
have separate reservations but share the SAME principal/deployment quotas and
physical slots. They are not free and not included in the cold-discovery timing.
For 192 serial requests plus three 6/24 h metadata allocations, the same
slow-budget calculation is 44.8833333333/179.1333333333 days; this is an illustrative read-only
accounting envelope, NOT a transcode/job completion or native-deadline change.
Two source-change retries per phase could require nine debits per aggregate;
all nine debit the same hourly quota, and the run counter may exhaust first.
Busy-root/principal admission remains pending before reservation rather than
repeatedly burning quota on 30 s queue expirations. Manual multi-path commands
retain explicit per-path overload outcomes.

| Illustrative raw reads at 600 MiB/s, no measured-fast claim | I | H |
| --- | ---: | ---: |
| One maximum complete aggregate | 443.7333333333 s | 1,774.9333333333 s |
| Three identity phases for one maximum aggregate | 1,331.2 s | 5,324.8 s |
| 64 maximum aggregates, discovery only | 28,398.9333333333 s (7.8885925926 h) | 113,595.7333333333 s (31.5543703704 h) |
| Same library, all three identity phases | 85,196.8 s (23.6657777778 h) | 340,787.2 s (94.6631111111 h) |

These divide exact aggregate bytes by 629,145,600 B/s and exclude metadata,
DB/queue/pause time, media processing and other IO. Finite 64/192-request hourly
admission calculations, rounding each raw read UP to a whole second, produce
zero byte-quota waits. They demonstrate quota usefulness, not measured normal
storage performance. 21/84 days remain slow-storage run ceilings, not enforced
minimum runtime. Full-library claim/revalidation work is outside a discovery
run's 16.25/65 TiB fingerprint counter but never outside hourly accounting.

## S2 Coupling Candidates, Not Shared-Lease Acceptance

| Quantity | Exact proposed candidate | Assumption / approval boundary |
| --- | --- | --- |
| Discovery / fingerprint `H,L` | 5 s renewal, 20 s expiry | `H + J + T_db + U + S = 5 + .25 + 2 + .25 + 7.5 = 15 < 20 s`. |
| Sufficient discovery/reader settlement `S` | 7.5 s | .25 s stop dispatch + .25 s bounded CPU + 5 s reserved outer/native allowance + 2 s checkpoint. Fingerprint's 100 ms + 2 s cleanup must fit inside this, not add to it. A blocked syscall violates this conditional envelope. |
| Shared ADR 512 attempt/aggregate and ADR 513 recovery/cleanup `H,L` | **5 s renewal, 40 s expiry** | Separate compatible candidate ONLY: `5 + .25 + 2 + .25 + 30 = 37.5 < 40 s`. This deliberately does not import ADR 503's 10/60 tuple. S2/parent must own acceptance and proof of the 30 s settlement assumption. |
| Shared contention opportunity | 1 s | Existing claim cadence preserved; no attempt consumed on contention. Proposed reuse for cleanup/recovery needs separate approval. |
| Shared transient infrastructure retry candidate | 1, 2, 4, 8, 16 s; five burst retries, then 60 s half-open cooldown | Parent LIFE-1 reconciliation: zero jitter; reset burst only after 60 s healthy probation. Fatal/B1/B3/unknown-root/drain/expired-request cases excluded. No request revival or task restart. |
| `D_app`, external grace, native TERM grace | **30 s / 45 s / at most 5 s** | Already accepted, unchanged. Hash requests of 19,860/79,260 s are clipped by `D_app` immediately on drain. |
| Conditional external relation | `1 s signal handoff + 30 s app + 10 s settlement = 41 s < 45 s` | Allocated assumptions only. S2 owns actual Linux evidence and any proposed added control-process topology. |

For all relevant owners, start a work step only with
`remaining_lease > uncertainty + settlement_for_that_step`, and use
`D_effective = min(D_request, D_app, safe_expiry_of_every_applicable_owner)`.
Renewal never extends a request or application clock. Earliest revocation wins
over a simultaneously completed digest. DB ownership, source lease, scan lease,
and worker reservation remain distinct identities.

**Original-root quiescence is mandatory.** Server-proven expiry and a new fence
are not sufficient for destructive takeover, cleanup, root reuse, or reuse of
an uncertain worker-capacity slot. Require affirmative old owner/unit absence
and settlement of its original root's writers and retained operations through
the approved recovery barrier. Unknown kernel I/O or lost-node state keeps the
barrier closed. No new filesystem lock, storage fence, control process, broker
lane, or timing acceptance is smuggled in by these numbers.
