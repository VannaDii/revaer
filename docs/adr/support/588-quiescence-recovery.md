# S2 Quiescence And Recovery Proposal

> Supersession: [ADR 593](../593-simple-checkpoint-recovery.md) replaces the
> custom supervisor/recovery machinery in this historical record. Unrelated
> approved decisions and transfer permissions remain in force; this appendix
> must not reintroduce the superseded implementation as a prerequisite.

> Current approval: The operator accepted ADR 588's exact applicable choices
> on 2026-09-11 at reviewed commit `9575c077`. The [resolution](../588-first-release-decision-package.md#approval-resolution)
> releases only those named design holds. Earlier pending/candidate wording
> below retains its historical context; implementation and qualification are
> not certified, and all other conditions remain binding.

Status: NEW, UNAPPROVED contract details for parent ADR588. ADR512's per-root
barrier, ADR513 preservation, and ADR550/557 attestation stay binding. Nothing
below adopts architecture, creates a service, edits SQL or resets user state.

## What Survives

PID1's stop/unclean latch, nonce, registry and wait records are volatile.
It does no database/media I/O. A kill, host crash or loss of the owning process
can destroy those records. Every new process begins with each root barrier
closed without needing to discover a durable `quarantined=true` flag. The NEW
graceful handoff below is written by the still-live application before DB close,
not by PID1, and only after irreversible mutator settlement. It can authorize
automated planned-restart reconciliation. Missing/uncertain proof, including an
unavailable DB, means unknown, not clean. No owner-death record is fabricated
after a kill, and the volatile latch does not become durable by description.

| Evidence outcome | Allowed action |
| --- | --- |
| The same current owning process retains exact child/thread/group identities, wait/absence and outstanding-operation records; it proves every relevant prior operation stopped and retains the root's sole-writer boundary | Automatic fenced ADR512 reconciliation, complete outbox/checkpoint/replacement classification and acknowledgement; reopen only after all root/source/capability/binding checks pass. No operator action merely because a transient DB outage lasted >31s. |
| Current owner still has records, but a wait, native destructor, mutating syscall, server commit or group absence is unknown | Keep barrier and uncertain physical slots closed; continue permitted observation/control-plane reconciliation inside the existing clocks. No new destructive owner. Deadline/containment failure is unclean, not a transient infrastructure retry. |
| New incarnation finds a valid, unconsumed graceful handoff and acquires the original root lock/attestation plus a fresh fenced recovery claim under the exact rules below | Atomically consume once, perform full automated512 reconciliation, and reopen only after every barrier predicate succeeds. No operator approval for a routine planned restart with this proof. |
| PID1/app died or host crashed without a valid committed handoff; current owner cannot authenticate the original record chain; or only logs/leases/exit codes survive | Operator-required containment recovery under the existing dedicated-service/RWOP sole-writer attestation boundary, then automated512 recovery. No timer, TTL or retry clears it. A crash after a valid irreversible handoff does not invalidate that proof by itself, but an old/missing/ambiguous record never qualifies. |
| Operator cannot prove all prior writers stopped and exclusive original-root authority was restored | Remain blocked, even after expiry or explicit retry. Rebinding/reset/deletion is not a workaround. |

Same-owner automatic recovery includes accepted B3 broker recovery when its exact
old broker group and work have settled, as well as ordinary infrastructure
recovery. It does not add application-child or media-task restart inside PID1.
Whole-process failure remains ADR514 whole-process failure. The graceful protocol
below is an explicit new cross-incarnation evidence path, not task restart or
lease-only takeover. Unknown-loss recovery remains operator-required. Neither
path omits resumable recovery, manifest rollback/finalization or any full feature.

## NEW Graceful Root-Quiescence Handoff

This section replaces v2's major availability cost: its rules required operator
confirmation on EVERY new incarnation, including a clean planned rollout. That
v2 proposal remains sealed in history. Recommend this exact additional
architectural delta instead. It is UNIMPLEMENTED; without approval and integrated
proof of this delta, **EVERY restart still needs operator confirmation**, even
exit0. Do not claim current unattended planned-restart recovery.

Publication prerequisites, in this order, all inside the ORIGINAL deadlines:

1. Irreversibly close application and PID1 admission for the whole incarnation.
   Stop every API/native/background mutator; join all owned async and blocking
   mutating work, libtorrent/session/destructor/fast-resume work, scanners,
   finalizers, cleaners and pending spawns. Resolve every server commit and
   mutating filesystem syscall. Paused durable jobs/outbox work may remain, but
   no live operation may later write. Unknown/uninterruptible work forbids a
   handoff. A cancellation token or empty queue alone is insufficient.
2. Settle BOTH entire broker lifetimes and read-only helper lifetimes, not just
   their last requests. Normal per-request broker reuse is unaffected. Managers
   retain real wait/group/pipe receipts; PID1 validates those receipts and any
   adopted-child waits. App sends CONTROL opcode13; owner latches no future
   registration/BEGIN_REQUEST and returns opcode14 only with complete proof.
   In-process join truth remains the trusted application ownership boundary,
   not something a nonmutating PID1 can independently inspect.
3. Keep the original descriptor-bound root lock and attestation until app exit.
   Only one final DB handoff publisher, its bounded commit-readback, pool close
   and nonmutating shutdown/telemetry may now run. No retry, reconfiguration,
   cleanup, health recovery or "handoff failed" branch can reopen media or root
   writes in this incarnation. These are irreversible state-machine invariants.
4. The application invokes `media_publish_quiescence_handoff_v1` through the
   stored-procedure boundary. Publish one atomic normalized batch for the full
   owned root set (at most the existing256 catalog slots). No per-root partial
   publication is accepted as a complete unit handoff. The transaction includes
   definitive durable ownership checkpoints and handoff records; it does not
   claim all resumable jobs/outbox items are completed. Preserve them for512.
5. Transaction plus any necessary bounded readback and DB closure must fit
   strictly before `min(D_force, earlier request/incident/safe-lease cutoff)`.
   Each DB transaction remains end-to-end2s including lock<=250ms; reserve the
   second2s readback before starting when commit acknowledgement could be lost.
   No new grace or use of the final2s enforcement reserve. No blind retry of
   publication. Unknown result stays closed; a successor may use a positively
   committed matching record, never the predecessor's assumption of rollback.
   Finally close pools/listener, settle logger, send opcode8 and exit/wait under
   ordinary S2. Late/forced shutdown remains unclean even with a valid handoff.

Exact normalized record schema (new application state, NOT JSONB):

| Record | Fields / bounds |
| --- | --- |
| Batch header | `handoff_id` fresh128-bit UUID; `version` exactly1; `predecessor_incarnation`16bytes; `predecessor_executable_sha256`32bytes; `owner_seal_sequence`u32 nonzero binding opcode14; `root_count`1..256; `committed_at` server timestamp; `admission_irreversibly_closed`true and `all_mutators_settled`true constrained flags. No arbitrary text/evidence payload. |
| Per-root member | Header FK; original `root_public_id`; current `root_attestation_id`, `catalog_generation`, `binding_generation` and `recovery_owner_generation` using their existing canonical identities/types; NEW `quiescence_epoch`positive SQL bigint; `state` exactly available/consumed/invalidated. One unique member per original root and epoch. |
| Consumed member | Additionally `successor_incarnation`16bytes, `successor_recovery_generation`existing fenced generation, `consumed_at`server timestamp; these are all null while available, all present after consumption. Invalidated members store only closed reason code and server time, not reusable authority. |

NEW root `quiescence_epoch` is a monotonic positive bigint, never reset/reused;
overflow is fatal admission failure. The publish procedure locks each original
root in stable root-ID order, checks current distinct512/513 fences and exact
catalog/binding/attestation, increments that root epoch, invalidates older unused
handoffs, and inserts the complete batch in one transaction. No success merely
because a lease remains live or a client asserted the flags. Bootstrap permits
this procedure only after the trusted ownership boundary has validated opcode14
and all application receipts. The database records that attestation; it is not
an independent kernel quiescence oracle. Direct arbitrary API callers cannot
publish this record. Same handoff ID + identical fields is idempotent; any
conflicting duplicate fails. No unsigned-to-bigint truncation or JSON fields.

Successor rules:

- Acquire the SAME original root's process-lifetime descriptor lock and reprove
 550/557 root identity, protected ancestry, dedicated namespace/deployment
  exclusivity (including exact RWOP where applicable) before consumption.
  A DB record cannot override an old retained lock or restore lost storage
  exclusivity. An upgrade may have a different executable digest; both packages
  must satisfy the approved trust contract. Digest equality is not quiescence.
- `media_consume_quiescence_handoff_v1` atomically claims the next distinct512
  recovery generation and CAS-consumes the matching available member/epoch,
  binding this live successor nonce. Existing claim eligibility still applies
  (prior terminal/released owner or server-proven expiry); no changed40s lease.
  New generation AND this affirmative handoff plus physical attestation are
  required. Advancing any competing root recovery/write authority invalidates
  unused handoffs in that same transaction; nobody can later consume an older
  epoch. Two successors cannot consume the same member.
- Consumption increments the root epoch and never directly opens the barrier.
  Run full512 manifest/checkpoint/finalization/complete keyset outbox and513
  reference recovery. Lost commit acknowledgement requires fenced readback for
  this SAME live successor, not another consumption or a lease-only fallback.
  Only after full reconciliation may new mutating admission open.
- If that successor dies after consumption but before producing its own valid
  handoff, its old consumed record is NOT reusable. Missing, invalidated,
  ambiguous, mismatched root/binding/epoch or untrusted evidence requires the
  operator path below. An expired claim or later healthy DB is never enough.
  A crash BEFORE publication also needs operator proof; a crash AFTER committed
  irreversible publication can recover automatically when all checks hold.
- Retain available/referenced members without age-based authorization changes.
  Prune only consumed/invalidated, fully unreferenced history under the existing
 720h diagnostic retention. No TTL creates authority or deletes resumable state.

This batch/epoch/schema, two procedures, opcode13/14 binding, irreversible seal,
and successor CAS are exact NEW approval deltas. SQL/migrations are not authored
here. There is no daemon, Docker socket, host PID scan, new service, reset API,
or implicit durability of PID1 memory. Required integrated crash acceptance:
before/after each join, seal, publish commit/ACK, DB close, lock release, claim,
consume commit/ACK, full reconciliation and first mutation; concurrent successors,
rollouts, dirty storage identity, lost ACK and epoch overflow. Zero incorrect
barrier openings; routine valid handoffs recover without an operator. None of
these tests is supplied by the synthetic corrective owner run.

## Exact New Operator Confirmation Delta

The existing closed sole-writer enums are not a prior-incarnation death token.
Merely rereading the same root catalog or acquiring an expired lease cannot
serve as confirmation. To make the operator-required path actionable, propose
this NEW explicit control-plane operation and normalized record, not an implicit
reset or a field silently added to ADR550's immutable source document:

`POST /v1/media/roots/{root_public_id}/recovery/quiescence-confirmations`

Use the existing root-administration authorization, one root per request,
maximum 4,096 UTF-8 body bytes, reject unknown/duplicate fields. No files,
credentials, paths, arbitrary process IDs or evidence uploads are accepted.
Exact body fields:

- `owner_incarnation`: the current CONTROL.md 16-byte nonce as 32 lowercase hex
  digits, bound to this live bootstrap and its current recovery-owner identity.
- `catalog_generation`: existing canonical positive decimal generation string.
- `recovery_claim_generation`: the current positive fenced recovery generation,
  same canonical decimal grammar. It cannot revive an expired claim.
- `disposition`: exactly `prior_unit_terminated`, `host_recovered_exclusive`, or
  `first_use_no_prior_writer`.
- `predecessor_incarnation`: 32 lowercase hex digits if positively known, JSON
  null only for a genuinely unknown predecessor or first use. Null does NOT
  waive absence proof; it requires proving the entire prior writable deployment
  boundary for that original root, rather than identifying one known unit.
- `evidence_sha256`: 64 lowercase hex digits identifying the operator's retained
  external containment/sole-writer evidence. This is an audit reference, not
  cryptographic proof that the claim is true.

`prior_unit_terminated` requires affirmative complete-unit termination and no
remaining/deferred writer, associated with the actual prior containment
generation, not a reused container/Pod name. `host_recovered_exclusive` requires
affirmative predecessor-host teardown/storage return and no other writer before
re-attestation. `first_use_no_prior_writer` is for evidenced first installation,
not missing records or a recreated DB. All modes require the existing protected
ancestry/ownership, descriptor-bound process-lifetime root lock and deployment
exclusivity. For Kubernetes that remains exactly ReadWriteOncePod plus the other
ADR550 checks, not ReadWriteOnce or a replica-count assertion alone.

The operator uses existing deployment/storage controls out of band. The app
does not gain a Docker socket, cgroup host mount, host-wide PID scan, privileged
helper, new daemon, remote reset, or ability to fence a lost node by assertion.
Acknowledgement without the stated evidence must not be presented as safe.
The remaining external truth is an operator attestation within the already
approved trust boundary, not a new automatic absence detector.

Application/bootstrap code, not PID1, calls a new stored-procedure boundary
`media_root_confirm_quiescence_v1` and records normalized scalar columns for the
above fields plus root-attestation ID, authenticated actor ID, server confirmation
time, and consumed state. At most one current confirmation per root/recovery
generation/incarnation; exact duplicate is idempotent, conflicting duplicate is
rejected. Consume once as part of fenced barrier reconciliation. A new boot,
new recovery claim, changed catalog/binding, authority loss or new uncertainty
invalidates it. Historical confirmation is audit only, never reusable authority.
Retain referenced evidence while needed; unreferenced audit retention follows
the existing 720-hour diagnostic policy. No JSONB application state.

The procedure verifies current owner/generation and admissible root state;
application re-proves the descriptor/attestation boundary. Stale/conflicting
confirmation cannot reuse a graceful token: successful confirmation advances
the same root quiescence epoch and invalidates all older available handoffs.
The existing512 recovery claim remains distinct from this proof. Stale/conflicting
generation returns 409; invalid input 422; absent permission 403; DB/deadline
failure 503 with commit-uncertainty readback, never an assumed rollback.
Successful confirmation does NOT directly mark the root ready: it permits the
current fenced leader to run the full existing recovery and then acknowledge
the barrier. Preserve old attempts, generations, manifests and outbox rows.

This endpoint/record/owner-nonce binding is a separate named S2 approval delta.
It does not exist in the tested source. If it is rejected, roots with missing
graceful/surviving-owner proof remain blocked; there is no undeclared fallback.
The graceful record above is the only proposed automated cross-incarnation
path, not a durable owner-DEATH record and not permission for crash-time writes.

## Shared LIFE Values And Latest Parent Correction

Use parent ADR588 LIFE-1 (source-evidence/parent-LIFE-1.md), not the superseded
manual-only retry draft or discovery's earlier `shared fatal/transient` wording:

- Attempt heartbeat, aggregate lease, recovery leadership and cleanup claim:
  renew every 5s, expiry 40s. Separate fenced identities and server-time leases.
- Preserve claim cadence 1s; active cancellation/control poll 250ms; cleanup
  sweep 1h; retention 24h/720h; cleanup candidate page/count 128. Failed cleanup
  eligibility waits for the next 1h sweep, not a new 1s busy loop. Preserve its
  fairness cursor until the root is removed AND all references are settled.
- Transient infrastructure only: initial attempt plus five retries with waits
  1, 2, 4, 8, 16s after failure completion, zero jitter. At most one control
  attempt in flight per pertinent owner. Each DB attempt has an end-to-end 2s
  maximum, with <=250ms server lock timeout inside that 2s.
- After failure of retry five, stay recovering/degraded for a 60s cooldown,
  then allow one bounded half-open control recovery attempt. A failed half-open
  starts another 60s cooldown. It does NOT restart a five-retry burst. The sum
  of burst sleeps is 31s; it is not a total outage deadline or total retry cap.
- Half-open success begins health probation. Reset the burst only after 60s of
  continuous pertinent healthy readiness with successful 5s renewals (all 12
  renewal opportunities in that interval). API liveness alone is insufficient.
  Any failed/missed required health/renewal breaks continuity. Before reset,
  failure returns to the already-exhausted cooldown state, not a fresh burst.
- A caller's request/lease/startup/recovery deadline is never extended to wait
  through cooldown. That caller ends at its original deadline; an independent
  permitted control-plane recovery opportunity is not resurrection of its work.
- Fatal error, B1 initial broker failure, B3 replacement, intentional drain,
  expired work, unclean containment and unknown root quiescence are excluded.
  Preserve B1/B3's exact existing rules, including B3's one replacement and
  earliest incident deadline. No new process/task restart or deployment backoff.
- Explicit operator retry is not an absence attestation and cannot bypass the
  closed root barrier. Ordinary prolonged DB outage does not require it.

Parent LIFE-1 now supplies the exact initialization clause: AFTER the existing
startup contract succeeds, each newly constructed permitted recovery owner
starts with its initial attempt and one five-retry budget. State is volatile
and per-owner process lifetime, NOT a durable/deployment-wide restart-rate
bound. The explicit consequence is a fresh burst in a new permitted incarnation;
repeated whole-process restarts are not bounded by this per-owner policy. This
does not create or authorize any app/task restart mechanism.

A new incarnation cannot inherit, resume or revive an old caller or claim.
Fresh fencing and affirmative quiescence remain mandatory. No new60s startup
delay, B1/B3 retry, retry ledger or retry schema is added. Within a live owner,
reset still requires60s continuous pertinent health and successful5s renewals.
V2's unknown-history60s initialization suggestion is superseded by this complete
parent proposal, not silently retained. The new
graceful-quiescence record has no retry-reset field; its consumption is not an
in-lifetime60s healthy episode.

These values are model/proposal only. Require outage/load tests at 31s, >31s,
multiple cooldowns, half-open success followed by <60s relapse, exactly60s reset,
DB commit uncertainty and caller deadline expiry. Verify no parallel attempts,
no request revival, no retry of fatal/B1/B3/unknown-root paths, and automatic full
reconciliation after health and valid quiescence return.
