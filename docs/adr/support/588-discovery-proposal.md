# DISCOVERY/FINGERPRINT Consolidated Decision Evidence: v2

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

Status: **UNPUBLISHED, NONPRODUCTION PROPOSAL AND DISPOSABLE LAB ONLY.**

Source: `fcdd95596fa062de9388d1be4f2ac96bbff571d1`; branch:
`work/media3-discovery-evidence-20260911`; worktree:
`/private/tmp/revaer-discovery-evidence-20260911`. Date: 2026-09-11.
Tracked delta: none. All authored material is under the ignored
`artifacts/decision-evidence/discovery/`. Parent owns ADR 588 and C1; this work
does not touch C1, audio, ADR/index files, frozen migrations, runtime defaults,
production, remote state, or source/media uploads.

## Recommendation And Decision Boundary

Recommend normalized **run + directory-epoch + individual-name frontier rows**,
atomic fenced candidate/checkpoint publication, and an explicit `aggregate_v1`
SHA-256 contract over exact UTF-8 names, selected membership, content hashes and
snapshotted rule provenance. Observations are separate race/cache evidence,
never a cryptographic substitute. Recommend coalesced durable due work,
mandatory rescan on watcher uncertainty, two clean absence observations for
non-mutating tombstones, and automatic *re-evaluation requests* on explicit
configuration activation without following mutable target/policy heads.

The complete, exact numeric approval table is
[Exact Candidate Values](588-discovery-values.md). It is a normative part of this
proposal and the single numeric source, covering every held 516/535 field,
initial and hard envelopes, outcomes, reasons, and S2 inequalities. No cell is
"choose later". These are named useful-workload proposals, not production defaults
inferred from a small lab. In particular, the 16 MiB/s slow-storage assumption
and GiB/TiB library envelope are explicit engineering hypotheses, not measured
service guarantees. Limit rejection is an intended outcome.

**Approval requested, not recorded:** the parent should present DISC-1 through
DISC-7 below in
one consolidated package. Approval of these exact values would still not prove
PostgreSQL concurrency, Linux containment, source safety, or production
readiness. Automatic discovery and destructive aggregate use remain held until
coordinated implementation, frozen-init transition, and all mandatory gates.

## One Bounded Workload Revision

The complete original v1 package was checksum-verified, copied into
`history/v1/`, and independently checksum-verified there BEFORE editing any
active candidate/report/harness file. Its SHA256SUMS SHA-256 is
`86ebb65b077c4596ac9e5189e12e34ffa3a822a19380f14f36d0cb7804ca5605`.
The snapshot includes the original report, candidates, results, provenance,
commands, verification, harness and both earlier attempts. It is immutable
history, not current numeric policy. No completed experiment was rerun.

v2 reads all 2,761 lines of MEDIA_TRANSCODING.md at the pinned commit, including
the full-release, multi-video, archival, 4K/remux-first, audio/subtitle,
verification, runtime, caching and fixture requirements. Table whitespace was
collapsed only for readable inspection, not in source. Relevant authorities are
`:155-208`, `:764-820`, `:825-951`, `:953-1082`, `:1661-1860`,
`:1864-2222`, `:2226-2264` and `:2434-2470`. The spec defines no numeric
maximum bitrate, duration or file size. Small conversion fixtures are behavioral
fixtures, not the useful production workload envelope.

The exact named assumptions and arithmetic are in CANDIDATE_VALUES.md and the
`workload_revision` object in results.json. `envelope.rb` verifies them with
integer/rational arithmetic and a finite admission-time calculation; it runs no
hash worker, filesystem experiment, media probe, network request or package tool.
There were **zero new sparse experiments**, zero generated/downloaded media and
zero new scratch bytes. A sparse stat would not qualify sustained throughput, so
the optional one-experiment allowance was not used.

| Envelope change | v1 | v2 proposed initial / hard |
| --- | --- | --- |
| Primary size | 8 / 32 GiB | 256 GiB / 1 TiB |
| Sidecar physical / total | 64 MiB / 256 MiB; 256 MiB / 1 GiB | 1 / 4 GiB; 4 / 16 GiB |
| Complete aggregate | 8.25 / 33 GiB | 260 / 1,040 GiB |
| Batch primary / complete aggregate bytes | 32 / 128 GiB primary only | 1 / 4 TiB primary; 1,040 / 4,160 GiB aggregate |
| Member / active / request seconds | 600 / 600 / 630; 2,160 / 2,370 / 2,400 | 18,000 / 19,800 / 19,860; 72,000 / 79,200 / 79,260 |
| Principal / deployment byte window | 33 / 264 GiB; 132 / 1,056 GiB per hour | 8 / 64 TiB; 32 / 256 TiB per 3,600 s, all phases debit |
| Principal / deployment debit count | 128 / 1,024 per hour | 4,096 / 32,768 per 3,600 s, both envelopes; hourly bounded expiry/settlement pruning |
| Run primary bytes | 16 / 64 TiB | Unchanged; new observed-aggregate AND admitted-fingerprint caps each 16.25 / 65 TiB |
| Run wall | 6 / 24 h | 21 / 84 days including hash/queue/quota/pause; separate cumulative metadata caps 6 / 24 h |

`long-4k-remux-4h` uses 4 h * (100 Mbps video + four 6 Mbps audio)
and 10% mux/embedded-content allowance: 245,520,000,000 B, below 256 GiB.
`dual-video-4k-archive-8h` uses 8 h * (two 120 Mbps video + four 8 Mbps audio)
and the same allowance: 1,077,120,000,000 B, below 1 TiB. Four existing image
subtitle payloads at 0.5/1 Mbps respectively fit the new sidecar envelope with
explicit space for indexes and text. These are planning allowances, not real
media measurements, universal codec maxima, or claims that archival originals
have an intrinsic 1 TiB maximum. All selected content remains mandatory.

At the **assumed, unqualified** 16 MiB/s floor, aggregate read times are
16,640 / 66,560 s. Active budgets leave 3,160 / 12,640 s. Four maximum
primaries now fit each batch, preventing a file-cap/batch-cap starvation hole.
For one available principal/root reader, 64 maximum aggregates plus the full
metadata allowance take 1,292,640 / 5,159,040 s under the selected rolling
quota and request-wall upper bounds, below 21 / 84 days. This is cold-discovery
arithmetic under named conditions, not a measured or guaranteed library SLA.
The earlier 6/24 h walls required 776.722962963 MiB/s for primary bytes alone,
not 16 MiB/s. Deployment workers cannot make a single root parallel.

All-three-phase accounting is explicit: discovery + claim + pre-mutation reads
cost 780 / 3,120 GiB per maximum aggregate before retries, or 48.75 / 195 TiB
for that library. The hourly byte quota counts each phase; a completed discovery
run is not completed media processing. See the canonical table for the exact
serial window calculation and failure/competition caveats. No numerical choice
is deferred; production qualification and operator approval remain required.

The parent agent's final arithmetic correction replaced the unsealed 24-hour,
four-request-share draft with useful one-hour 8/32 TiB principal and 64/256 TiB
deployment limits. This was a proposal revision, not operator direction or
approval. v2 had not been sealed; only sealed v1 is historical proposal
evidence. No extra experiment or new investigation followed. At illustrative
600 MiB/s (not measured), one root reads 2.0599365234375 TiB/h, below either
principal ceiling. All-three-phase reads of the maximum 64-aggregate library
are 23.6657777778 / 94.6631111111 hours of raw IO, with zero quota waits in the
finite hourly calculation. Metadata, real service overhead, other IO and
transcoding remain additional; 21/84-day walls are slow-storage ceilings only.

Count quotas also use one hour. Retained debit rows are separately capped at
8,195 per principal / 65,556 deployment (two count windows plus 3/20 unsettled
requests). One fenced DB-only pruner runs hourly, at most 257 transactions of
256 rows, at most 65,556 deletions, within 600 s. Delete only expired AND
physically settled debits. Any backlog still hitting those hard caps rejects
admission. Referenced reservation/identity/audit records are retained separately;
neither the debit window nor pruning releases uncertain physical capacity.

## Evidence And Source Findings

| Evidence class | Scope |
| --- | --- |
| Current source | Exact SHA above; source file SHA-256 values in provenance.json; v2 final git/source verification retains zero tracked delta. |
| Real filesystem/process lab | Owned shallow/wide/deep trees, real creates/renames/deletes/lstat, regular and sparse file reads, inherited read-only member descriptors, cooperative stop, forced termination of a sleeping child, reaping and PID absence. |
| Semantic model | Scalar relational rows serialized for fresh-process resume; virtual-server-time admission/fairness, fencing, tombstones and configuration keys. No database transactions or storage durability proof. |
| Fixed known answers | Eight canonical byte vectors pinned in checks.rb, independent `shasum -a 256` verification, decode/re-encode, truncation/trailing rejection. |
| v2 arithmetic only | Exact named bitrate/duration/sidecar sizing, request slack, batch/file coupling, rolling bytes/counts, 64/192 serial requests and run-age reconciliation. No new measured throughput or filesystem result. |
| Reused S2 evidence | Read-only prior REPORT.md under the parent's `artifacts/media-verification/2026-09-11-decision-evidence/s2/`; hazards were not rerun. Prior S2 source is 059cca9d..., not this source. Its measurements are not imported as new measurements here. |
| Absent proof | No PostgreSQL, Linux image/container/cgroup, broker, production Rust fingerprint executor, actual native watcher, real authentication or multi-replica service was run. |

High-signal inspected facts, repository-relative references:

- `MEDIA_TRANSCODING.md:799-820`: manual discovery by default, per-association
  watcher/schedule, no default schedule cadence, immediate enqueue when enabled.
  `:2248-2264` binds source, target, policy and capability in planning/cache keys.
- ADR 516's unresolved choices select neither numeric limits nor aggregate
  bytes. ADR 535 explicitly holds every numeric admission value.
- ADR 557 `:598-701` binds discovery to immutable association versions and root
  attestations; the empty association prefix can mean the whole root, but a
  candidate path cannot be empty. Its catalog fence, 128-association bound and
  explicit stale-generation recovery remain unchanged.
- `media_discovery_scan.rs:14-18` embeds depth 32, 16,384 entries, 1,024 files,
  64 GiB and 30 s. `:96-117` repeatedly enumerates a directory and filters
  names <= its in-memory `after` key. That is not a stable filesystem snapshot.
  `:135-160` can advance past an over-depth or over-byte entry; the proposal
  instead retains an explicit incomplete/limit diagnostic.
- `media_discovery_runtime.rs:40-44,133-184` owns process-local cadence/history;
  `:195` uses a blocking scan; `:300-342` hashes and rehashes before enqueue.
  Its nil system principal and fixed watcher heuristics are not authorization.
- `media_discovery_fingerprint.rs:18-22,103-121` selects a 64 KiB buffer,
  4,096-directory cap, 64 logical sidecars, 256 MiB sidecars and an unversioned
  little-endian name/size/content stream. `:222-310,365-408` uses fixed
  extensions/longest-stem ownership and errors-to-absence behavior in the watcher
  owner resolver. Those facts do not approve the constants or the semantics.
- `crates/revaer-data/src/media/jobs.rs:45` calls the legacy profile/path
  enqueue procedure. It does not implement the new configuration-aware key.
- ADR 558 `:270-305,793-858` fixes two RVB1 lanes, one active native request per
  lane, exact closed categories, descriptor rules, earlier-deadline authority,
  and five-millisecond cancellation polling. Fingerprint helpers must not become
  a third RVB1 lane, a native-tool pool, or a direct-native-spawn fallback.

v1 reviewed full root/data/Rust/devops instructions and relevant
450/500/501/512-514/516/535/557/558 contracts. v2 reread root/data/Rust/devops
instructions, the existing report/candidates/results/harness, 516/535 and the
entire MEDIA_TRANSCODING.md. This is not a fresh full-stack code audit.

## DISC-1: Exact Relational Frontier Proposal

Use `public`, scalar columns and normalized join rows, consistent with the
accepted root/data grant model. All runtime access is via generation-fenced
stored procedures; this report introduces no SQL or migration. Names are
proposed internal names; their semantics, keys and bounds are substantive.

Shared types: identity ids are positive `bigint`; public/request/principal ids
are `uuid`; state/reason names are closed `text` lookup values; clocks are
`timestamptz` in PostgreSQL and monotonic deadlines in a process. Path columns
are valid UTF-8 `text COLLATE "C"`, byte-counted with `octet_length`, using
ADR 557's exact grammar. Database encoding must be UTF8. Digest columns are
32-byte `bytea`; device/inode are eight-byte unsigned big-endian `bytea` as in
557. All FKs are restrictive. No JSON/JSONB, arrays, generic extension map,
serialized cursor, persisted readdir cookie, or caller-owned absolute path.

| Proposed relation | Exact minimum columns, keys and invariants |
| --- | --- |
| `media_discovery_schedule_state` | PK association-version id FK to 557; `interval_quantity integer`, closed `interval_unit`, `anchor_due_at`, `next_due_at`, `last_coalesced_first_due_at`, `last_coalesced_last_due_at`, `last_coalesced_count bigint`, `updated_at`. Active version and approved numeric policy checked at every mutation. Disabled schedules have no due claim. |
| `media_discovery_rescan` | PK association-version id; `requested_sequence bigint`, `satisfied_sequence bigint`, `first_requested_at`, `last_requested_at`, `not_before`, nullable `requested_configuration_activation_id` only for legitimate absent activation. Sequence increment is checked; overflow fails closed. One row coalesces all unsatisfied triggers. |
| `media_discovery_rescan_reason` | PK `(association_version_id, reason_code)`; FK rescan and closed reason lookup; scalar `last_requested_sequence` and first/last occurrence times. Cap the set to `manual, schedule, overflow, watcher_uncertain, directory_changed, configuration_activated, restart_reconcile`. Repeated reasons update one row, never create a row per event/sequence; satisfaction of sequence R cannot clear a reason whose latest sequence is >R. |
| `media_discovery_run` | PK run id/public UUID; FKs exact association version, catalog generation, source attestation, effective policy version, target version, admission-policy version and principal; captured trigger high-water sequence; closed state; `claim_generation bigint`, nullable owner UUID/lease expiry only when unowned; due/start/end times; scalar examined-entry, directory, selected-file, primary-byte, aggregate-byte, admitted-fingerprint-byte, cumulative-metadata-time and live-frontier byte counters. At most one nullable pending-reservation FK plus pending candidate key/epoch while awaiting identity; null means no pending request. Root/binding identity cannot change in-place. Partial unique current run per binding. |
| `media_discovery_run_reason` | PK `(run_id, reason_code)`; immutable captured trigger reasons, first/last represented due and missed count remain scalar run/schedule evidence. A trigger during a run cannot be erased by finishing an older high-water mark. |
| `media_discovery_frontier_directory` | PK `(run_id, directory_id)`; unique `(run_id, relative_path)`; nullable parent-directory FK only for the association root; `depth smallint`, `enumeration_epoch bigint`, closed phase, pre/post device/inode/mode/size/mtime/ctime observations, nullable last-consumed name only before first consumption, entry/name-byte counts and invalidation sequence. Root path is the exact association prefix, including explicitly empty prefix. |
| `media_discovery_frontier_name` | PK `(run_id, directory_id, enumeration_epoch, entry_name COLLATE "C")`; validated one-component name, closed observed file kind, logical size, descriptor observations, closed pending/consumed/diagnosed state. No ordinal acquired from native enumeration is identity. Consume by byte-order keyset index, never OFFSET. |
| `media_discovery_observation` | PK `(association_version_id, source_relative_path)`; source attestation/root generation, last positive run, latest aggregate identity FK, observation epoch, nullable first-missing run/time, missing-clean-pass count, closed present/missing_once/tombstoned/unresolved state. Observation replacement never rewrites a job snapshot. |
| `media_aggregate_identity` | PK identity id; `contract_version smallint`, `digest_algorithm smallint`, `digest bytea`, primary root-relative path, physical/logical counts and exact observed bytes. Association/root and effective rule-snapshot references. Unique identity includes binding/version/digest, not digest globally. Immutable after complete validation. |
| `media_aggregate_member` | PK `(identity_id, member_ordinal)`; unique `(identity_id, relative_path)`; kind, exact path, size, content SHA-256, grammar version, rule ordinal, rule SHA-256, logical-group key. Ordinals are contiguous byte-sorted physical names; exactly one primary, complete VobSub pairs. Scalar observation columns retain dev/inode/mode/mtime-sec/nsec/ctime-sec/nsec; signed seconds and nsec 0..999999999 are lossless. |
| `media_discovery_intent` | Unique `(association_version_id, source_relative_path, aggregate_contract_version, aggregate_digest, target_version_id, effective_policy_version_id)`; exact identity FK and nullable job FK until enqueue. Insert intent + immutable job snapshot + source observation + frontier checkpoint atomically. Capability binds only on first successful claim, not in this key. |
| `media_fingerprint_reservation` | PK authenticated caller idempotency UUID; principal, replica, root, admission-policy, current fence generation, observed/member reservations, monotonic-progress counters, server admission/expiry/deadline times and closed queued/running/cancelling/complete/rejected/unsettled state. Reservation generation is never a scan or job generation. |
| `media_fingerprint_debit` | PK reservation id FK; principal, server admission time, exact reserved bytes, member/request counts. Immutable within its one-hour window; no refund on cancel, failure or early exit. Only expiry PLUS physical settlement permits bounded hourly pruning; reservations retain compact admission audit independently. No external FK pins a debit beyond that purpose. Enforce 8,195/65,556 retained principal/global rows before admission as well as window counts. |
| `media_discovery_fairness` and `media_fingerprint_fairness` | Separate PK class/scheduling-domain rows, next service ticket and last served principal/root/policy ids. Lock/advance with admission; index eligible work by ticket then stable id. No wall-time TTL or process-local cursor reset. |
| `media_discovery_diagnostic` | Bounded reason lookup, exact binding/run/directory/identity references, first/last occurrence and count; at most one current diagnostic per `(run, path-key, reason)`. Paths only in access-controlled evidence, not metrics. Retention 720 h after no live reference; coalesce repeats. |

Required indexes: due rows `(next_due_at, association_version_id)`; eligible
runs `(state, not_before, service_ticket, run_id)`; expired owners `(lease_expires_at,
run_id)`; names by the exact PK above; observations `(association_version_id,
last_positive_run_id, source_relative_path)`; debit ledger `(principal_id,
admitted_at, reservation_id)` plus global admitted-time index. Capacity rows
are separate normalized instance/global/principal/root scalar counters or fixed
slot rows, locked atomically with a reservation; they must not be inferred from
stale telemetry. Index design is proposed, not EXPLAIN/load-tested SQL.

Required procedure operations, each with named inputs, complete fence tuple and
typed bounded outcome: request/coalesce rescan; claim eligible quantum; renew;
begin/stage/seal directory epoch; append positive identity/intent and checkpoint;
finish clean census; record absence pass; release/pause; reserve/dispatch/renew/
complete/cancel/settle fingerprint. Unknown commit outcome is read back by
idempotency key, never replayed under a new key. Runtime has execute-only access,
not table mutation privileges. The accepted catalog fence/root-lock order comes
before resource/capacity locks; take same-class locks in ascending id order.
No lease is held open by a long database transaction.

### Directory And Restart Semantics

1. Claim a bounded quantum against the exact active binding, current catalog and
   source attestation. A new owner increments the run fence. Retain the original
   root descriptor identity; a path string does not restore authority.
2. Enumerate at most the approved directory cap plus one overflow witness,
   count raw bytes/entries, reject non-Unicode or invalid names, and no-follow
   lstat/open through the root owner. One bounded directory inventory is sorted
   by unsigned UTF-8 bytes. Its work is additionally charged to run budgets;
   the 256 processed-frontier-row quantum does not pretend enumeration costs
   only 256 entries.
3. Stage individual name rows under a fresh unsealed epoch in <=256-row
   transactions. Compare directory identity/change observations before sealing;
   a changed directory, incomplete stage, rejected name, deadline, or lost fence
   cannot publish a complete inventory. Enumeration/staging must fit the selected
   2/5 s quantum; a slow database can exhaust it and cause explicit retry/pause.
   There is no hidden unlimited staging-time exception.
4. Seal and consume only a complete epoch. Store the last consumed key and new
   child directory rows atomically with positive observations/intents. A candidate
   needing a full digest first persists its unconsumed key/epoch and one pending
   reservation FK in a bounded quantum. Hashing does not run inside the 2/5 s
   metadata quantum or DB transaction. The scan execution slot may be yielded
   only after actual scanner quiescence; its fenced run/frontier and the hash
   worker's separate physical reservation remain retained. After settlement, a
   later quantum rechecks binding/epoch/member evidence and atomically publishes
   the complete identity/intent and consumption checkpoint. Stale/failed work
   cannot consume the candidate as success. Crash
   before commit repeats safely; crash after commit resumes after that key.
   Lost ownership rejects every stale write. Partial staging left by restart
   is discarded in bounded pages and re-enumerated, never resumed via OS cookie.
5. Detect changed directories at consumption/end checks and watcher sequence
   boundaries. Old inventory consumption may produce only stale, safely fenced
   positive intent; it can never authorize mutation. It cannot count as an
   absence census. Re-enumerate with a new epoch / coalesced follow-up run.
6. Batch file/byte/time limits yield, retaining valid progress. Depth, directory,
   run-age, total-row/byte, membership, ambiguous-owner and repeated-instability
   limits record explicit incomplete `paused_limit`/`paused_unstable` evidence.
   I/O failure is `paused_io`; stale catalog/binding is `stale_binding`. Neither
   is empty-directory success. Independent safe subtrees may still produce
   positive observations; the run cannot certify deletions.

This is a convergent live-tree scan, not a global filesystem snapshot. An
external writer may change names after any observation. Descriptor proofs,
complete membership revalidation and the accepted cooperative sole-writer/source
lease are still required. The lab model uses pathname `lstat` for traversal and
JSON files for row snapshots: it proves neither the proposed descriptor-relative
race boundary nor fsync/crash durability nor stored-procedure concurrency.

## DISC-2: Exact Aggregate Contract

Select `aggregate_v1`, version integer 1, digest algorithm registry value 1 =
SHA-256 using existing project SHA-256 support. Do not assign legacy unversioned
fingerprints this version. Stored observations and content identity have distinct
purposes. The canonical aggregate commits to the membership/content/rules below;
dev/inode/mode/times are separately persisted audit/cache/race evidence, not
inputs that turn a harmless timestamp touch into different content identity.

All integers unsigned big-endian unless a separate observation column explicitly
uses signed seconds. No native-width encoding, padding, optional field, trailing
extension, JSON, normalized whitespace, Unicode normalization or case folding.

| Order | Width | Exact canonical field |
| --- | ---: | --- |
| Header 1 | 11 | ASCII `REVAER-AGG` followed by byte 00 |
| Header 2 | 2 | Version `00 01` |
| Header 3 | 1 | Algorithm `01` |
| Header 4 | 2 | Physical member count; 1..129 and selected policy cap |
| Repeated 1 | 1 | Kind: 1 primary, 2 subtitle sidecar (text or non-VobSub image), 3 VobSub index, 4 VobSub payload |
| Repeated 2 | 4 + N | Path byte length then exact root-relative UTF-8 path; 1..4096 |
| Repeated 3 | 8 | Exact descriptor-observed logical byte size, 0..i64::MAX, further bounded by admission |
| Repeated 4 | 32 | SHA-256 of all member bytes in file order |
| Repeated 5 | 2 | Snapshotted rule grammar version; 0 for primary, 1 for this candidate's ADR 450 grammar |
| Repeated 6 | 2 | Snapshotted rule precedence ordinal 1..65535; primary is 0 |
| Repeated 7 | 32 | Raw rule SHA-256; primary is 32 zero bytes |
| Repeated 8 | 4 + G | Logical group-key byte length then key; primary empty, ordinary subtitle exact path, VobSub exact common root-relative basename without `.idx`/`.sub` |

Physical records are sorted by unsigned path UTF-8 bytes, strictly increasing.
No duplicate name is permitted even with a different kind. The sole primary is
identified by kind, not forced to be first in lexical order. Distinct member
names resolving to the same physical inode make the selected aggregate an
alias/ownership conflict, not two silently deduplicated members.

Rule digest bytes are exactly:
`SHA256(ASCII("REVAER-RULE") || 00 || u16be(grammar_version) ||
u16be(rule_ordinal) || u32be(pattern_utf8_length) || exact_pattern_utf8)`.
The referenced immutable effective-policy snapshot supplies enabled rules and
their ordered token grammar; no runtime fallback grammar is introduced here.
The lab ambiguity fixture exercises a small exact stem/rule output model,
not the entire ADR 450 language compiler. The final implementation must prove
that compiler's language/role/extension domain and immutable snapshot readers.

Primary fields must be exactly zero/empty as specified. Other kinds require
nonempty group, grammar 1 and positive precedence. Every selected VobSub group
must contain exactly one kind-3 and one kind-4 member with coherent snapshot
provenance. Wrong/missing/duplicate companions reject the WHOLE aggregate.
Unknown version, algorithm, kind, impossible counts, invalid names, oversized
lengths, trailing bytes or incomplete records reject the whole message before
unbounded allocation. A reader must not infer or negotiate another version.

### Member Scope And Ownership

Include exactly one primary file and every adjacent sidecar selected by the
approved ordered snapshotted ADR 450 grammar through ADR 451's effective policy,
including both VobSub physical files. Include selected input sidecars even when
the output target would remove them: target policy cannot make destructive
input evidence disappear. Enabled-rule precedence resolves multiple matching
rules for the SAME owner by the lowest ordered matching rule. More than one
possible primary owner is `ambiguous_owner` regardless of stem length or rule
precedence; do not silently assign the longest prefix. Missing required rules
fail before sidecar probing; a genuinely empty validated rule set means no
sidecars, not a fallback extension list.

Exclude unselected NFO, artwork, checksums, playlists, external metadata,
directories and network inputs. They are not authorized for removal, backup,
quarantine or replacement by this aggregate. Embedded streams/attachments are
already bytes of the primary and need no separate physical member. No perceptual
or container-independent substitute is introduced. New companion classes require
an explicit scope/version decision and re-observation, never a best-effort append.

### Construction, Cache And Revalidation

Enumerate exact membership first; reserve checked sum of logical sizes and
physical/logical member counts. Reject unknown sizes, overflow and excess before
worker admission. Read only retained read-only regular-file handles, sequentially
in bounded chunks. Charge every actual byte, including sparse zero regions.
Read no extra prefix/EOF sentinel beyond the reserved size: early EOF rejects;
after exactly reserved bytes, descriptor size/mtime/ctime checks detect growth
or change. Reopen/revalidate names and the complete keyset before publication.
No member may be omitted on error. Malformed worker output, extra output and
nonmonotonic progress reject the request.

Stable observation equality permits reuse of the previous full content digest
only for bounded discovery deduplication/planning hints under the SAME binding,
rule snapshot and membership. Any membership/observation/rule change causes
rehashing. No second full hash is needed merely to enqueue: stale queued intent
is harmless only because claim revalidation is mandatory. First successful claim
and immediately before first source mutation each reserve and recompute the
complete aggregate, under the accepted source lease and earliest deadlines.
Recheck expected descriptor identity/keyset after hashing. Lease loss or cancelled
publication wins over a simultaneous valid digest. The parent registry retains
workers and capacity until actual settlement; no expired reservation implies
kernel or original-root quiescence.

### Fixed Known Answers

`results.json` retains every complete canonical hex string. `checks.rb` pins the
eight expected SHA-256 values, and `checks.json` contains independent Perl
`shasum` commands/stdout. The text-sidecar fixture intentionally contains the
seven bytes `68 65 6c 6c 6f 5c 6e` (literal backslash+n), not a line terminator.

| Vector | SHA-256 |
| --- | --- |
| `a.mkv`, content `abc`, primary only | `deb9f7261804948c4f0ac34c072ad5ae02a849b7d3f1315328a708c4ddfc1281` |
| Primary plus selected text sidecar | `4ac0b87c7b6637a240eef9a2369a63d7e5509c8bac56d8fe9ab1d939b03ac7fc` |
| Primary plus complete VobSub pair | `f90eb307a2ecfacf8ff56a583e0d4958a922b7c656e50da42781dc92b8baa7cd` |
| NFC cafe-with-acute filename | `8e45fc63bdccca410452eb981b25254cb0cd3a9aa4e95d5201a0ad4dc161fa8e` |
| Distinct NFD filename | `524fc2fd7e8c15ffa0c005db5260d108ab4f26aa2ffa14a92b20d574c0d2e2f9` |

Other fixed vectors cover uppercase, embedded newline and zero content. Input
order reversal leaves the digest unchanged; changed rule provenance changes it.
Invalid UTF-8, traversal, empty/oversized path, duplicate physical name,
ambiguous owner, missing companion, excessive member count, unsupported version,
ambiguous concatenation, truncation and trailing bytes are rejected/tested.
The physical-count 129 test uses generic sidecar records solely to test that
encoding ceiling; it is NOT a proof that 128 logical sidecars satisfy the
separate 64-group policy ceiling. Full grammar/group/alias enforcement is a
required implementation test, not hidden as a passed lab scenario.

## DISC-3: Upgrade Procedure

1. Register a new immutable aggregate contract/algorithm only through an explicit
   approval. Every persisted identity and intent retains its original version.
2. Add new known answers and supported-version readers before any writer changes.
   A version is a complete semantics tuple, never a configurable hash name.
3. Re-observe and fully hash existing sources under the new approved version with
   the same admission budgets. Do not transform old digest bytes or reinterpret
   legacy members. No digest-only backfill, destructive dual-write fallback or
   trusted timestamp migration.
4. Existing jobs retain old snapshots; if their exact version is no longer
   permitted, hold them with `aggregate_version_unsupported` and require explicit
   re-plan/re-evaluation. Never rewrite or silently revive them.
5. Explicitly activate the new writer only after validated re-observation and
   caller compatibility; rollback closes new admission and retains both histories.
   Garbage collection cannot remove a version/manifest referenced by a job,
   reservation, replacement, outbox, recovery or diagnostic record.

## DISC-4: Stale Deletion, Rename And Configuration Semantics

- A positive changed source invalidates cached observation evidence immediately.
  Disappearance marks `missing_once`, not a deletion job. Tombstone only after
  two complete, clean, same-binding/root passes at least one second apart.
  Overflow, unreadable directories, stale generation, budget exhaustion,
  cancellation, changed directory or unsealed epoch reset absence evidence.
- Tombstones suppress further old-path discovery and preserve last-seen identity;
  they delete no filesystem object, job, retained artifact or audit history.
  Existing queued intent encounters a typed source-missing/source-changed hold
  at claim. Do not rewrite its immutable source path or call it successful.
- Rename is old-path absence plus new-path observation. Even same device/inode
  and same content do not transfer intent, leases or destructive authority.
  A same-inode rename link may be recorded only as diagnostic correlation;
  both paths still follow full observation/keyset checks. An inode reused after
  deletion is not proof of continuity.
- Explicit activation of a new target/effective-policy/profile/association
  version creates ONE durable re-evaluation request per newly affected active
  binding. Merely creating a draft target/policy version does nothing. ADR 557
  associations bind exact profile versions and must not follow mutable heads;
  required explicit profile/association rebinding precedes the rescan.
- Enabled automation processes that request through its fair budgeted pipeline.
  For manual-only bindings it remains `manual_evaluation_required`, visible but
  unexecuted until the operator runs discovery. Disabled/manual-disabled bindings
  receive no surprise scan. This proposal does not invent automatic enablement.
- New target/policy version creates a new intent even if source digest is
  unchanged. Re-evaluation reuses a hash only as the observation-safe discovery
  hint described above. Identical complete intent key returns the same existing
  job/intent; explicit operator retry is a separate accepted workflow.
- Capability identity is deliberately absent from the discovery intent key.
  ADR 449 binds it at first successful claim; planning/cache/audit use the full
  source+target+policy+capability identity. A capability refresh does not silently
  revive a held old job or issue a configuration rescan.

## DISC-5: Admission, Fairness And Overload

Use the exact counts/bytes/windows/queue/request/member/grace tuple in
CANDIDATE_VALUES.md. One physical instance token, global token and principal/root
authorization must all exist before a worker starts. Queued requests reserve
exact counts/bytes durably before occupying local queues. Caller idempotency keys
bind the complete parameters/principal; reuse with altered parameters is invalid.
Cancellation/expiry/revocation latches disposition before accepting any result.
Queued cancellation releases queue occupancy; running cancellation retains its
physical/principal capacity while `cancelling` or `unsettled`. The completion
procedure requires current generation, uncancelled state, complete canonical
evidence and live binding. Exactly-once settlement is an idempotent procedure,
not a `Drop` assumption.

Window admission debits reserve bytes permanently for that one-hour window, including
failed/abandoned requests. No refund encourages retry churn or requires trusting
an unreceived progress report. Exact server-time rolling ledger and independent
request/cardinality caps prevent tiny or zero-byte inputs bypassing all quotas.
Capture one server `clock_timestamp()` after acquiring the reservation/capacity
locks; callers cannot submit the debit clock. The end-to-end transaction budget
and late-commit readback still apply. Quota retry computes the earliest prefix of expiring debits sufficient for the
new request, minimum 1 s and maximum one window; it does not blindly return the
oldest debit. The prefix must satisfy both byte and cardinality debits. Window
expiry never releases an unsettled physical slot. Arithmetic is checked before
reserve. Invalid/anonymous principal,
unknown size and unsupported policy fail closed.

Round robin serves one eligible principal per turn, FIFO by admission time then
request id within a principal, with independent local/global/root limits. Use
separate durable fairness cursors for scans and hashes; internal principals are
bounded. Queue saturation rejects explicitly; a 30 s queue deadline intentionally
permits rejection behind large reads rather than an unbounded backlog. There is
NO claim of bounded successful latency or equal byte throughput for mixed-sized
requests. With bounded scheduling and service time, admitted nonexpired work
gets a fair opportunity; stalled workers retain slots and trigger fail-closed
containment rather than replacement-token oversubscription. Model results cover
eight equal-cost principals, not adversarial weighted production fairness.

For discovery, retain one pending candidate/request per run, not 64/256 queued
requests from a batch. Check durable principal/root availability before making
a new reservation; on known contention defer to the next fair 1 s opportunity
without a byte debit. Already admitted work still has the 30 s queue limit.
A queue-deadline result has zero automatic retries, leaving a visible paused
candidate for explicit retry. Source-change retries retain their exact two-retry
bound and each gets a new charged observation/reservation under the original
outer deadline. No caller retry/idempotency operation may reset an unresolved
request's clock, refund old debits or bypass actual physical settlement.

Policy versions are validated as tuples: primary plus sidecars <= request;
one maximum request fits an empty batch and its principal/global window; batch
and run counters are checked independently; member/active/queue/DB/settlement
allocations fit request time under the named qualification assumption. No
operator field increase can silently leave an incompatible byte/time/window
combination. Every exact I/H value is in CANDIDATE_VALUES.md.

The 535 read-only helper pool is separate from RVB1 native-tool execution. A helper
may process one member and terminate, with the request owner continuing the next
member under its SAME deadline/reservation; do not require all 129 member FDs at
once. Bootstrap injects the existing-authorized 535 executor boundary. Actual
Linux child restriction, executable/handle transport and packaging still require
implementation review; this Ruby fork harness is not that sandbox. Do not widen
558's wire categories/FD inheritance or create an additional broker manager to
fit fingerprinting into RVB1. Any actual conflict discovered in implementation
must return as an exact approval delta, not a native direct-spawn fallback.

## DISC-6: S2 Composition And Original-Root Quiescence

The stable table was supplied to the parent for S2 reconciliation. The separate
agent in `/private/tmp/revaer-shutdown-bound-20260911` owns actual Linux closure
and any proposed added control-process topology; no such topology is adopted
or tested here. This report supplies candidate inequalities/values, not duplicate
timeout-hazard experiments.

Discovery/fingerprint: `H=5s`, `L=20s`, conditional sum `15s < 20s`.
Shared attempt/aggregate/recovery/cleanup: **candidate only** `H=5s`, `L=40s`,
conditional sum `37.5s < 40s`, requiring the whole 30 s settlement assumption.
All terms and retry choices are explicit in the table. Neither ADR 503's 10/60 s
tuple nor the replay's one-hour stale threshold nor a control poll is imported
as approved ownership timing.

Keep accepted 30 s application aggregate, 45 s external grace, 5 s native TERM
ceiling and exactly two serial RVB1 lanes. A 19,860/79,260 s hash request is clipped
by the earlier application/request/lease/recovery clock. Fingerprint 100 ms grace
and 2 s settlement are NOT additive to native or shutdown deadlines. Cancel,
lease loss and drain stop new work; a late complete digest cannot restore success.

**Server expiry is not original-root quiescence.** New destructive ownership,
cleanup, root reuse and uncertain-capacity reuse require affirmative old-owner
and original-root settlement/absence through the accepted recovery barrier.
No new mutator starts because a timer merely passed. KILL is an action request;
it is not a wall-clock guarantee through uninterruptible I/O, suspension, a lost
node, or failed supervisor. If absence cannot be proved, readiness stays closed.
The lab forced only a normally interruptible sleeping child, not kernel I/O.

## Measurements And Rationale

Preserved v1 main run: results.json (all original keys unchanged); independent
checks/concurrency: checks.json, byte-identical to v1. v2 adds only the
`workload_revision` result. No earlier experiment time is presented as a new run.
Host: Ruby 4.0.6, arm64 Darwin 25.6.0 on `/dev/disk3s5`, mounted at
`/System/Volumes/Data`. No cache drop, filesystem/storage-class attestation,
thermal/CPU isolation, network filesystem or cold-read experiment. `sysctl`
hardware detail was denied by sandbox and is recorded as unavailable, not zero.

| Test | Final observed outcome | Candidate implication and limit |
| --- | --- | --- |
| Shallow 16 x 128 files, 512 bytes each | 2,048 files, 17 dirs, 9 quanta, 25.407 ms | Real metadata traversal; normalized batches recover full set. Not media probing. |
| Wide 16,384 empty files | 16,384 files, 65 quanta, 13.196 s | Model's Ruby array row removal is intentionally not an indexed DB. Reject this implementation as production frontier; no throughput claim from normalization alone. |
| Directory 4,095 / 4,096 / 4,097 against 4,096 cap | Accepted / accepted / rejected; accepted staging about 107-109 ms | Boundary supported at local scale; 4,096 initial cap, not legacy ratification by coincidence. |
| Directory 16,383 / 16,384 / 16,385 against hard cap | Accepted / accepted / rejected; accepted staging about 459-465 ms | Hard bracket measured; query/commit overhead not included. Slow storage/DB can still hit 2/5 s quantum. |
| Depth 65 chain under depth-64 cap | 65 file observations, 65 dirs, explicit over-depth diagnostic | No silent complete census or traversal beyond selected depth. |
| Restart between each shallow batch | 9 fresh Ruby processes, 2,048 unique files, complete | Row-state serialization/reload only; no concurrent durable transactions, fsync proof or fencing races across real replicas. |
| Insert earlier name, rename, delete after first batch | Stale generation rejected; directory marked dirty; new clean pass found a/b/c exactly | Demonstrates why an `after` name alone is insufficient. Symlink was diagnosed rather than followed. |
| SHA-256 64 MiB file, 64 KiB chunks, three trials | 1,585-1,670 MiB/s including fork/progress/supervision | Cache-rich small-file measurement. 64 KiB sacrifices peak throughput for smaller cooperative byte quantum. |
| Same, 256 KiB / 1 MiB chunks | 1,827-1,841 / 1,861-1,888 MiB/s | Larger chunks are faster here but are rejected as initial quantum without slow-I/O cancellation evidence. |
| Two concurrent principals, two separate inherited read handles, three trials | 3,509-3,662 MiB/s combined; each complete 64 MiB digest | Six real child processes, not cross-replica database fairness or disk throughput. No justification for >2 local workers. |
| Cooperative cancel after about 20 ms, three slowed-read trials | Reaped 0.495-0.533 ms after signal; no digest | Delay injected between real read chunks; not a blocked-kernel syscall guarantee. 100 ms grace is a conservative proposed control allocation. |
| Child sleeps after first chunk and ignores cooperative channel | KILL action after about 20 ms test grace; reaped 20.669-20.704 ms after cancel; no digest | Reduced lab grace is a test control, NOT the proposed 100 ms or accepted native 5 s. Not a hard realtime bound. |
| Growth / shrink / same-size overwrite during read | source_changed / early_eof / source_changed; no digest | Real file operations; no adversarial timestamp restoration or Linux fs race proof. |
| Sparse logical 64 MiB | Full logical bytes charged and full digest | Sparse allocation cannot bypass byte accounting. No multi-GiB fixture used. |
| Eight principals, two modeled replicas, 16 queued operations | Four dispatch slots, one running each principal; p0..p7 then p0..p7; 16 complete | Model, equal-cost workload. Local/global/principal queue and rolling-window boundaries reject. |
| Cancellation versus publication / stale release | Late result rejected; unknown absence retains cancelling slot; wrong generation cannot release | State-machine model only, not durable exactly-once behavior under lost commit acknowledgements. |
| Fixed known answers and independent decode/hash | 8/8 pass, truncation/trailing rejection, Unicode/name ordering remains distinct | Byte-contract evidence; complete ADR 450 compiler matrix still outstanding. |
| Tombstones, configuration identity, cadence and debounce | Incomplete/dirty pass cannot tombstone; two clean passes can; version keys distinct; 11 missed intervals coalesce; continuous events cap at 5 s | Deterministic models, not a deployed scheduler/watcher. |

The lab exercised the 64 KiB read quantum, two concurrent children and finite
directory brackets; that does not prove these are optimal production values.
Run-level 65,536/262,144 entry limits, larger
GiB/TiB admission, long durations, DB latency, concurrency and retention are
resource-envelope decisions informed by the model/arithmetic, not measurements
at those maxima. They limit allowed work and failure behavior; they do not
certify cold 260/1,040 GiB aggregates, hours-long requests, a 64 TiB library or
storage SLA. 16 MiB/s is an assumption to test on actual supported package and
filesystem combinations, not a measured minimum or a newly approved floor.
Default 32/128 MiB frontier name counters cap retained name bytes independently
of the maximum row count; databases still incur additional row/index/WAL overhead.

## Rejected Alternatives

- Process-local cadence/cursors, ordinal offsets or persisted `readdir` cookies:
  restart and mutation do not preserve a stable work set.
- Lexical `after` alone over a changing directory: new earlier names are missed;
  the mutation lab reproduces the relevant sequence.
- Array/JSON frontier persistence: violates normalized application-state policy;
  the JSON lab checkpoint is explicitly a disposable model, not a proposed ABI.
- Sealing a partial or over-limit directory, skipping invalid names, silently
  omitting ambiguous sidecars, or tombstoning from incomplete scans: unsafe
  negative evidence and unreviewed identity degradation.
- Unversioned legacy digest, path+mtime identity, redundant full hashing merely
  for enqueue, sampled/prefix hashing, or automatic version conversion: rejects
  exact membership and immutable evidence requirements.
- BLAKE3/new serialization dependency: no need demonstrated; use existing
  SHA-256 and explicit lengths/endianness with known answers.
- Higher worker counts, unlimited internal principal, refund-on-disconnect,
  process-local quotas, unbounded watch queues: no representative storage/load
  evidence and permit abusive work amplification.
- Third RVB1 lane, configurable native pool, thread abort as a kill primitive,
  post-deadline success, or lease-expiry-only takeover: incompatible with the
  accepted broker/lifecycle and original-root quiescence requirements.
- Automatically follow newly created target/policy heads or enable manual-only
  scans on config changes: violates immutable binding and manual-default intent.

## DISC-7: Exact Approval Delta And Remaining Implementation Gates

The parent's consolidated ADR should ask for these named decisions, without
recording consent on the operator's behalf:

Discovery labels are deliberately DISC-1 through DISC-7. Database D1/D2/D3
approval and pending D4/D5 are separate and unchanged.

1. **DISC-1 frontier:** approve the normalized relations/keys, sealed directory epochs,
   bounded keyset checkpoint and invalidation semantics; positive-only progress
   from incomplete runs; typed pausing rather than silent skip; exact numeric
   scheduler/traversal/watch rows in CANDIDATE_VALUES.md.
2. **DISC-2 identity:** approve aggregate_v1 bytes/SHA-256, separate observation role,
   exact member/scope/ownership/companion rules, complete admission and revalidation.
3. **DISC-3 upgrades:** approve explicit version registration and bounded full
   re-observation, preserving old evidence and holding unsupported jobs.
4. **DISC-4 stale/configuration:** approve two-clean-pass diagnostic tombstones,
   rename-as-old/new, and coalesced re-evaluation on explicit immutable activation
   with manual-only bindings held for an operator command.
5. **DISC-5 admission:** approve every 535 numeric row, global/principal/root occupancy,
   debit-on-admission rolling accounting, no capacity refund before confirmed
   settlement, bounded overload/retry and helper cancellation semantics.
6. **DISC-6 S2 coupling:** reconcile the conditional 20/5 discovery/fingerprint and
   separate 40/5 shared-lease candidates with S2's actual closure results. The
   latter and shared retry choices require their OWN explicit decision; approval
   of 516/535 cannot make them accepted. Approve original-root quiescence evidence
   at takeover only through the consolidated ownership/recovery contract.
7. **DISC-7 activation remains separate:** approve the selected envelope for subsequent
   implementation, not production activation, migration changes, broker topology,
   E1/C1/audio choices, publication or relaxed gates. Existing accepted deadlines,
   dry-run restrictions, capability and root readiness remain prerequisites.

Before activation: real stored-procedure begin/stage/seal/restart fencing with
two or more backends; late commit/renewal outcomes; exact role grants and frozen
cutover; property/race and non-Unicode tree tests; whole grammar/companion/alias
matrix; maximum manifests/member/byte arithmetic; constrained local and network
storage, loaded CPU and cancellation at every read/publication boundary; actual
535 helper restriction; native watcher overflow/loss; adversarial principals and
replicas; actual original-root quiescence and Linux package enforcement from S2;
complete media conversion/fault matrix; strict Sonar/security/release criteria;
`just ci` and `just ui-e2e`. These are existing required implementation gates,
not an indefinite continuation of this finite decision investigation.

## Validation, Commands And Provenance

Executable work used the artifact-local Justfile only. Exact commands, exit
outcomes, failed-attempt explanations and evidence locations are in
`COMMANDS.md` in the retained discovery evidence; full child/source-inspection command stdout/stderr is
in commands.json and history/. Independent SHA commands are in checks.json.
The lab scripts themselves retain every experiment-control count and action.

v1 focused gates: syntax, lab, checks, verify and source-check passed as retained
history. v2 runs only syntax, envelope arithmetic, preserved-evidence/source
verification and sealing. It does not execute lab.rb or checks.rb again. Seal
checks include unchanged source and historical evidence, exact arithmetic/table
agreement, tracked delta zero, no owned scratch and the retained checks.json pass.
Full `just ci` and
`just ui-e2e` were NOT run: the task permits writes only inside ignored evidence
and prohibits image/build/pull/other unrelated resource work; the canonical gates
would invoke managed DB, release/UI builds, dependency/audit and generated source
surfaces outside this evidence boundary. The parent owns integrated handoff and
ADR 588. This report neither substitutes a focused gate for those gates nor
claims an application or repository handoff is complete.

Unchanged historical harness SHA-256 values (also verified against provenance.json):

| File | SHA-256 |
| --- | --- |
| model.rb | `1eecd6776c88ea6ae0ea767d32755af99b3e48e82838fe769466c01d43e03724` |
| lab.rb | `7f92d124135c19751927676fc3f63e2dd961a5db821d1d8dc2f7694852b30c65` |

The v1 artifact Justfile digest remains in history/v1/provenance.json. The v2
artifact Justfile adds arithmetic-only gates; current digests of every harness,
report and candidate file are in provenance.json and SHA256SUMS. No tracked
Justfile or instruction was changed.

SHA256SUMS covers the entire retained package, including report, numeric table,
supplementary checks/verification code and failed-attempt records. Source-file
hashes are in provenance.json. It intentionally excludes itself to avoid a
self-referential checksum. No binary or source was sent off-machine.

## Cleanup, Risk, Rollback And Policy Check

All scratch directories were created privately beneath this artifact directory
and removed in ensure/finally paths in v1. v2 created none. The largest regular hash fixture was
64 MiB; sparse logical fixture 64 MiB; mutation fixture at most 8 MiB+1,
sequential and removed before sparse creation. Shallow files total 1 MiB and
wide/boundary files are empty. Maximum simultaneous logical fixture payload was
about 130 MiB, far below the 512 MiB owned-scratch cap. Main pre-cleanup `du`
was about 72.5 MiB allocated; it is an end-of-run observation, not a measured
peak disk-allocation trace. Supplementary concurrency used one fresh 64 MiB
file after main scratch cleanup. No huge or acquired media corpus.

Final main run reaped all 19 hash children and 9 short-lived restart children;
supplementary check reaped 6 concurrent hash children. Earlier successful main
run also cleaned its own children; its results remain in history/. Every
completed run records scratch removal. No containers, image layers, volumes,
networks, ports, source-adjacent media files, installed dependencies, credentials
or parent evidence were modified. Worktree removal belongs to the parent after
preservation; this task does not delete it.

There is no production change to roll back. Discard this ignored package after
preservation to abandon the experiment. After a future approved implementation,
rollback closes automation/fingerprint admission, cancels through owned
supervision, preserves uncertain reservations/root barriers and immutable
diagnostics, and reverts only the reviewed coordinated slice. Never restore
unbounded hashing or silently reinterpret old identities as the new version.

Dependency rationale: Ruby standard library only for the lab; existing local
`just`, `git`, `shasum` and OS tools. No Node was required; no ambient Node or
non-NVM command was used. No production dependency is proposed. No PostgreSQL
capacity was requested, so no coordination or Docker action was necessary.

Stale-policy review: AGENTS plus scoped data/devops/Rust, live root freeze
authority, exact inspected spec/ADRs/source. Historical pre-v1 migration wording
does not override current freeze rules. Accepted architectural labels do not
unhold numeric selections. Current process-local/legacy aggregate implementation
drift is reported above, not repaired. All files/ADR ownership and criteria
boundaries remain unchanged; no exception, suppression or operator consent was
invented.

v2 removes the stale small-file-only size/time rationale, the incompatible
batch/file caps, the short cold-run wall implication, and discovery labels that
collided with database decisions. The root task-record text in the supplied
instructions and on-disk file differ; this ignored evidence does not resolve or
edit either policy. Per the explicit continuation scope, the parent alone owns
ADR 588 integration, tracked records and full handoff gates. Existing H5/L40
shared and H5/L20 reader candidates, DB 2 s / lock 250 ms, J/U 250 ms,
reader S7.5 s, hash cancellation 100 ms + settlement 2 s and earliest-deadline
clipping remain unchanged and conditional on S2. Hash workers remain 2 per
instance / 4 deployment; active scan execution remains 1 / 2. Physical
settlement precedes capacity reuse and any root mutation without exception.
