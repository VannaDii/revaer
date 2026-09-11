# First-Release Decision Package

- Status: Proposed
- Date: 2026-09-11
- Operator approval: Pending for all new production decisions in this package.
- Supersedes: None. Existing approvals, conditions, and holds remain binding.
- Implementation status: In progress; isolated evidence work only.
- Ready for consolidated operator review: No. Remaining investigations below
  must produce their recommendations and evidence before the approval request.

## Research Authority And Scope

The operator activated the revised goal on 2026-09-11, explicitly permitting
bounded reproducible investigations in isolated disposable environments:

> Experimental candidate designs and values may be evaluated solely to produce
> evidence; they must remain clearly identified, unpublished, and outside
> production behavior.

This is evidence-work authority, not approval of the resulting architecture,
production defaults, cutover, external uploads, or criteria changes. In
particular C1, D4, D5, E1, S2, the binary exception, and the enumerated choices in
512-516/535 remain unapproved. D3 remains conditional, not certified.

This record consolidates the exact approval delta being prepared. The
[approval register](../media-approval-register.md) remains navigation to actual
operator decisions; the [completion ledger](564-media-completion-ledger.md)
remains the single requirement-to-evidence tracker. There is no new release
scope or parallel status ledger. Evidence is pinned to integration
`059cca9d7af584c14fd78c1b920ac821fcb7762d` unless stated otherwise.

## Investigation Completion Contract

Each known decision must end with a specific recommendation, exact scope and
values where relevant, reproducible evidence or a bounded safety argument,
alternatives, consequences, dependency ordering, validation requirements, and
rollback. Measurement is not proof of an absolute bound. A source listing,
candidate digest, successful experimental assertion, or partial fixture pass
does not certify a service, package, or supported workload.

Investigations are finite experiments with stated questions and stopping
conditions. Reuse prior evidence when its source and scope still match. Do not
repeat unchanged failure reports, rerun unrelated gates to wait for approval,
or treat an unavailable external resource as an architectural decision.
Only newly discovered or materially changed decisions reopen after the
consolidated checkpoint; no unknown is waived by this process.

## Decision Completion And Delivery Order

The active goal keeps the complete first-release service, not merely a first
working slice. Its immediate milestone is one decision-complete approval
package, followed by implementation, full verification and merge. Existing
architectural holds remain in force while their disposable candidates are
investigated. No additional operator decision is requested by this checkpoint.

1. Finish the independent database closure, package/startup and shutdown
   investigations. Include adjacent reproduced database defects, the missing
   compliance artifact delivery, exact environment bytes, and actual process
   enforcement in the same package as their dependent decisions.
2. Use the shutdown/ownership result to settle recovery, heartbeat, lease,
   takeover and retry values together. Resolve discovery identity, durable
   traversal and admission limits against that same lifecycle. Audio preset
   and listening evidence can proceed independently. Each workstream must
   finish with exact proposed values and consequences, not "choose later."
3. Verify native build capacity, the supported execution hosts and the actual
   Sonar analysis connection. Prepare any separately scoped upload request.
   Reconcile the corrected asset deliverable with the final linear stack
   boundary before proposing its exact exception. Access/capacity problems
   must not become hidden assumptions in the approval package.
4. Present one consolidated decision table with dependencies, exact approval
   deltas, rejected alternatives, evidence limits, acceptance tests and
   rollback. Record only the operator's actual decision. Do not split a known
   dependency into another surprise approval request after adoption starts.
5. After approval, implement outside-in: init/root/profile configuration and
   manual discovery/inspection/planning with safe dry-run; full execution,
   verification, replacement and recovery; remaining automatic discovery,
   audio and operator controls; both native packages and install/recovery
   rehearsal. These are delivery milestones, not optional release features.
6. Integrate through one reviewable linear stack within the unchanged canonical
   size rule. Require clean full CI/UI, applicable GitHub checks, strict Sonar
   with positive published coverage and resolved feedback before merge. Keep
   operator assignment, Copilot requests and exact-source evidence current.

One integration owner coordinates disjoint worktrees. Completed evidence is
retained before their removal; test media is deleted after each turn. A newly
discovered architectural question still requires approval, but the known
questions above must be answered before this package is called complete.

## Database: Reproduced Defect Family

### Question And Experiment

Can the exact D4/D5 proposals remove the known ingestion failures without
hiding adjacent failures, granting the runtime elevated privileges, changing
the caller's variable-conflict setting, or pretending D3 is complete?

The disposable PostgreSQL 16.14 experiment compares four independent variants:

1. Exact frozen reference candidate, under its original superuser authority.
2. Exact current final init, including conditional D3, under a directly
   connected constrained runtime role.
3. A clone of variant 2 with only D4 plus original IMDb-only D5 applied to the
   ingestion routine inside that temporary database.
4. A separate clone with D4 plus the matching partial-index predicates for all
   three ingestion external-ID branches: IMDb, TMDB, and TVDB.

Frozen migration bytes, the committed final init, its digest, the guard, and
runtime/bootstrap authority are unchanged. Every case uses a fresh database
clone. Repeated calls within a case stay on one observed backend; no reconnect
or `DISCARD TEMP` hides the warm-session failure. Candidate routine definitions,
exact calls, stdout/stderr, connected roles, settings, and before/after images
of all 18 existing ingestion-proof tables are retained.

The fourth bounded matrix has **69 cases and 1,065 passing experimental
assertions**, including one deliberately incorrect commit as a negative
control. Expected legacy failures count as successful reproduction, not
working application behavior. The matrix covers cold and committed repeated
calls, the unversioned procedure wrapper, helper-first compilation,
rollback/retry, an invalid request followed by a valid request, changed policy
snapshots, explicit same-transaction repetition, all three ID insert/upsert
paths, invalid-ID rejection, actual last-seen changes, and retained source-ID
links. Earlier 48-case, 56-case and 68-case runs remain as evidence, not
replacements for the reviewed matrix.

Independent review found that the earlier all-table rollback assertion was
vacuous whenever any call in a sequence succeeded. Run 04 adds snapshots of
all 18 tables immediately before and after each call, after savepoint error
recovery and after each transaction finish. It compares those snapshots with
independent pre/post-session observations. Each failed call preserves its
prior table image; explicit first-call rollback restores the initial image.
The negative control actually commits that first call and verifies that the
rollback-preservation predicate rejects the resulting state. A second
independent review found no material issue in these new assertions.

The observation helper is read-only, SECURITY DEFINER, confined to a new
disposable schema with a fixed table list, `pg_catalog` search path and PUBLIC
access revoked. Only its usage/execute grants are added for the experiment.
Those grants and the helper are not in the final init; caller-role checks do
not turn this instrumented experiment into production privilege certification.

| Observed Operation | Reference / Current Final | D4 + IMDb D5 | D4 + Three ID Predicates |
| --- | --- | --- | --- |
| Second call after commit, including wrapper | `42P07` | Success | Success |
| Different policy snapshot on second committed call | `42P07` | Success; new rule applies | Success; new rule applies |
| Second call in the same explicit transaction | `42P07` | `42P07` | `42P07` |
| IMDb insert/upsert | `42P10` | Success | Success |
| TMDB and TVDB insert/upsert | `42P10` | `42P10` | Success |
| Invalid ID | `P0001`, no table changes | Same | Same |

With D4, the policy table exists within successful transactions and is absent
after commit/rollback. The same-transaction limitation remains because the
routine's other scratch tables also have transaction lifetime. This experiment
does not propose multi-call transaction support. All final/candidate calls
retain a non-superuser, non-role-creating, non-bypass-RLS connection and leave
the caller's `plpgsql.variable_conflict=error` setting unchanged.

### Exact Recommendation For Review

Recommend D4 as already scoped in [579](579-ingestion-policy-temporary-table-lifetime.md),
and extend the still-unapproved [583 D5 proposal](583-ingestion-imdb-conflict-inference.md)
to the **three reproduced ingestion branches only**:

- `CREATE TEMP TABLE tmp_policy_rules AS` becomes
  `CREATE TEMP TABLE tmp_policy_rules ON COMMIT DROP AS`, once.
- The one IMDb `ON CONFLICT (canonical_torrent_id, id_type, id_value_text)`
  receives `WHERE id_value_text IS NOT NULL` before `DO UPDATE SET`.
- The two TMDB/TVDB `ON CONFLICT (canonical_torrent_id, id_type, id_value_int)`
  targets each receive `WHERE id_value_int IS NOT NULL` before `DO UPDATE SET`.

These are final-init-only proposals. Keep existing index definitions, null
semantics, inserted values, update assignments, signatures, grants, and
timeouts unchanged. Do not edit the frozen reference or normalize its failures
into a successful D3 result. The two analogous canonical-merge statements are
outside this recommendation: no reachable same-hash multi-row merge has been
demonstrated under the existing unique infohash indexes. Do not disable those
constraints to manufacture a production reproducer.

Alternatives are to retain the demonstrated failures or redesign scratch-table
and uniqueness behavior more broadly. Neither fixes these reproduced cases
with a smaller semantic change. A pool-reconnect workaround moves ownership to
callers and does not establish correct procedure reuse.

### Limits And Next Proof

This experiment is not the complete D3 certificate. It does not yet establish
the complete branch matrix, mutating-helper-first compilation, observations
inside all reachable helpers/triggers/dynamic calls, or complete cold/warm
existing-data parity. Database-only execution does not exercise the Rust
`PgPool` wrapper or full installed service. Runtime rank changes, all alternate
identifier conflicts, and cancellation during ingestion still need their
required acceptance cases. No final-init approval or cutover follows from a
passing experimental harness.

Finish the D3 closure using unchanged reference evidence and separately named
candidate deviations. The next database investigation must map and exercise
the remaining helper/branch cases, not merely rerun the successful matrix.
After actual approval, regenerate and pin the exact final bytes, extend the
canonical delta guard narrowly, run full final-init/role/baseline proof and
the complete service gates, then perform the coordinated cutover.

Rollback before adoption is deletion of the disposable candidate only. After
approved implementation, reject a candidate that fails the exact proof and
preserve the prior reference and failure evidence. This is not authority for
resetting a user's database or reverting other work.

### Reproducibility

The experiment uses the already-pinned local image
`docker.io/library/postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`,
reporting PostgreSQL 16.14 on `aarch64-unknown-linux-musl`, with data checksums
enabled. Its network is disabled and data resides on an owned 2 GiB tmpfs.
That storage choice is not disk-durability, power-loss, capacity, or performance
evidence. The container and its databases are removed after every run.

Local, unpublished evidence is retained under the primary checkout's
`artifacts/media-verification/2026-09-11-decision-evidence/database/`. The final
run directory is `database-run-04`; the executable harness stays at the
database evidence root, with a byte-identical snapshot inside each run.

- Harness SHA-256: `fd83472382a680a4b40da1244f267345e70b67ac0687e27eacb94a0fb63feb32`.
- Final report SHA-256: `4cee6aa67478af56d5b22cf7ccc67893a8b5ee749e9848a9ed0231867ab467b8`.
- Exact frozen candidate SHA-256: `fb0d371f1afe365a72bbe34e9c8fe628e318be6de0e1f43b6f73c37897c95858`.
- Current final-init SHA-256: `1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.

Run from an isolated checkout at the pinned source, placing the retained
harness at `artifacts/decision-evidence/database_probe.rb` and providing the
verified frozen candidate:

```sh
just --command ruby --disable-gems artifacts/decision-evidence/database_probe.rb \
  /absolute/path/to/verified/init-candidate.sql artifacts/decision-evidence/new-run
```

The harness verifies the frozen source and current final bytes before starting
PostgreSQL. It refuses an existing output directory and requires the local
pinned image; it neither pulls an image nor uploads source.

## Package Environment And Compliance Startup

The package investigation inspected five immutable local arm64 image IDs and
ran bounded, network-disabled probes in two historical Alpine runtime images.
Neither runtime image identifies the current integration source; the three
larger validation images are not release packages. Native amd64 execution and
current-source package provenance are not established by these observations.

### E1: Exact Candidate And Observed Contradictions

Both inspected runtime images lack the proposed PATH entry `/usr/local/sbin`
and have `/home/revaer` owned by the service (`100:101`, mode `02755`), contrary
to the root-owned HOME contract. Both also lack the environment manifest.
The candidate for further package verification is exactly these nine records,
in this order, with an LF after every record including the last:

```text
PATH=/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
LD_LIBRARY_PATH=/usr/local/lib
SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
SSL_CERT_DIR=/etc/ssl/certs
LANG=C
LC_ALL=C
TZ=UTC
HOME=/app/media-home
TMPDIR=/tmp
```

The 214-byte candidate SHA-256 is
`494940de65f3d8e4ef01b434b76e10d9a1e42deab28d2fb7ef249cb64f5bb4af`.
Propose an empty root-owned `0555` `/app/media-home` and a root-owned `0444`
manifest at the already-proposed `/app/config/media-process-broker-env-v1`.
Leave the unrelated service-account home and application HOME unchanged.
Preserve exact-byte, no-follow, ownership, mode, path and closure checks; do
not move environment validation outside ADR 558's common broker startup clock.
These bytes and package actions remain unapproved and uninstalled.

Direct `env -i` probes produced exactly the nine records and excluded public
ambient canaries. Real historical arm64 tools started, generated and inspected
tiny synthetic media, extracted PCM, remuxed and decoded it, and exercised
temporary-file creation. A root-owned read-only tmpfs HOME supported those
operations; it does not establish installed final-layer metadata. The original
attempt to extract FFV1 video with mkvextract failed as unsupported and remains
failed evidence. The successful follow-up deliberately tested PCM extraction,
not the unsupported operation. All synthetic media was deleted.

The remaining seven values have no additional directly observed contradiction,
not blanket validation. Certificate parsing is not TLS trust-chain proof;
version responses are not complete interpreter/library/data closure proof;
`env -i` is not the actual parent/broker/child snapshot implementation. E1
approval readiness still needs the real candidate in both current-source
native packages and the original descriptor, clock, containment and fixture
acceptance requirements. Keeping the original values or creating the unused
PATH directory and re-owning the old home are alternatives; the dedicated
HOME avoids changing the unrelated account's writable state.

### C1: Earlier Boundary And Missing Artifact Delivery

Source inspection shows `BootstrapDependencies` construction already creates
database infrastructure, a native session and a real Tokio worker. The start
of `run_app_with` is too late, and router construction occurs later still.
Recommend one typed preflight before either public entry path constructs
those dependencies, with a required successful digest witness passed through
the dependency/API wiring. A task-free diagnostic sink must report a bounded
failure once before optional telemetry exporters start. No production entry
may bypass the witness, select a test fixture, return `None` on failure or
substitute the unavailable sentinel. This sharpens C1's proposed fail-closed
boundary; it does not approve whole-service startup failure.

A standalone unpublished Rust 2024 prototype on aarch64 macOS passed 24 final
subprocess cases: 18 rejected before its continuation and six accepted. The
continuation actually bound a loopback listener, spawned/joined a thread and
spawned/reaped an env-cleared child. This is not actual Revaer bootstrap or
Linux package acceptance. Existing trim/JSON parser behavior, including
duplicate-key, whitespace and symlink observations, is retained rather than
misrepresented as an authenticity or no-follow guarantee.

An additional prerequisite must be resolved with C1: the observed images and
current Dockerfile/chart do not deliver
`/app/compliance/final-image-compliance-bundle.json`. The workflow generates
the digest-bound bundle after building the image. Its source-compliance
artifact also embeds the final image identity; copying those bytes back into
the image changes that identity and invalidates the binding. A static inventory
digest, fabricated bundle or relaxed validator is not an acceptable substitute.

The next bounded experiment must select and exercise post-build delivery of
the verified artifact set at the existing path, including exact image binding,
read-only ownership, absent/foreign/mutated rejection and actual early startup.
A verified read-only deployment mount is a candidate, not an approved or
completed delivery design. Do not request C1 approval while leaving this
known package dependency unanswered.

All 16 owned containers and prototype fixtures were removed. The existing
`just image-compliance-test` fixture mutation suite passed, without certifying
a real release bundle. Local evidence, exact scripts, both failed attempts,
final results and the 211-file checksum manifest are retained under the
decision-evidence `package-startup/` directory. No image build, pull, upload,
registry publication or workflow dispatch occurred. Adoption requires the
consolidated approval and full package/service gates; rollback of this
experiment is removal of owned disposable artifacts, not a production change.

## Corrected Asset Review Boundary

Read-only Git inspection, with Node 24.19.0 selected through NVM, establishes
the corrected two-commit asset deliverable below. The
[machine-readable boundary](support/588-corrected-asset-boundary.json) retains
every text path, mode, old/new blob identity, and no-rename line count. Its
binary inventory is independently rehashed and exactly equals all 213 entries
in [585's manifest](support/585-binary-deletions.json), including path, mode,
blob OID, SHA-256, byte length, and deletion status. No binary additions,
modifications, renamed entries, or unknown entries are admitted.

- Base: `945e81d610c320bdb62e3f3eb728e6f0a218e0da`.
- Head: `0d725dce044bea934ec36ca96dc7863bbed2526b`.
- Commits: corrected conversion `3892b126c8d82c3342ad1168ca4a74476e77c70c`
  followed by the malformed-asset regression tests in the head commit.
- Complete boundary: 317 paths, comprising 104 text paths and 213 binary
  deletions. The text subtotal is **3,191 additions plus deletions**. Binary
  deletions account for **21,064,113 original bytes**, not zero review effort.
- Shared binary inventory SHA-256:
  `3e1e4af169880c3aee57e967a01c58f9d4f368bb7ac9b72666622e7ce1918d7a`.
- Canonical `just stack-changed-lines BASE HEAD`: exit 1, as required by the
  unchanged rejection of binary/uncountable entries. The numeric subtotal is
  not a passing canonical guard result.

The proposed selection is this corrected representation only, retaining
585's exact-identity, deletion-only boundary, unchanged 9,999-text-line maximum,
expiry on this one deliverable's merge or abandonment, and no future binary
bypass. Prior original/replay identities remain historical evidence, not
additional uses of an exception. No executable exception is installed.
Existing correctness, branding, licensing, desktop/mobile and platform-icon
acceptance requirements remain; this inventory run does not rerun their
historical validation or certify missing visual/platform checks.

Before the consolidated approval request, reconcile this deliverable's actual
stack position and exact base/head identity. A later rebase does not silently
inherit a SHA-bound exception: retain fail-closed behavior, and present any
proposed identity-preserving replay allowance as an explicit criteria delta
with negative tests. This work does not approve such an allowance.

The experiment source, raw NUL-delimited Git inventories and failed canonical
guard output are retained with the unpublished decision evidence. The
historical 585 proposal is not rewritten as if it already covered these bytes.

## Shutdown: Reproduced Hazards And Design Delta

An independent isolated agent ran 18 final trials on aarch64 macOS with Rust
1.96.0 and the repository-pinned Tokio 1.52.3. All trials matched their expected
outcomes and all child processes were reaped. The lab directly includes the
unchanged production log writer and byte-identical extracted parser functions;
surrounding types/error adapters and candidate cancellation models are not the
complete application. Offline locked build, focused warnings-denied Clippy,
formatting and extraction checks passed. No Linux package or integrated
shutdown proof follows from these lab results.

- With stdout intentionally unread, the real writer prevented task destruction
  after abort. A 100 ms join timeout returned while the writer was still blocked.
  Returning from async work then blocked runtime destruction. In all three
  unreleased trials, the external lab watchdog killed/reaped the child at its
  two-second control point. That is a measured stalled duration, not a proof
  of mathematical infinity or an approved production timeout.
- The extracted parser consumed 30 MB of comments, 11 MB of duplicate CIDRs,
  or 8 MB of invalid lines despite a one-millisecond Tokio timeout. Unique-rule
  limits do not bound input bytes or non-yielding work. The duplicate cases
  took approximately 121-130 ms and invalid cases 185-200 ms in this lab.
- A candidate incremental parser stopped scanning at its cancellation boundary;
  exact/over-limit body and line checks were exercised. Its 16 MiB body, 4 KiB
  line and 4/64 KiB quantum values are experimental controls, not selected
  release defaults or a scheduler-latency guarantee.
- A queue model retained an admitted command after its producer was cancelled
  and joined. A started blocking-task model ignored abort until explicitly
  released. Neither model executes the actual libtorrent worker.

The inspected source does not wire the accepted signal-driven API drain, and
the chart does not declare ADR 514's accepted 45-second grace. Current stop
helpers allocate sequential per-task 30-second waits rather than the accepted
single aggregate budget. Those are implementation gaps, not permission to
change the existing accepted requirements.

The design direction is **owned phased cancellation and complete downstream
settlement**, not an unconditional join of the watcher alone:

1. One monotonic shutdown deadline and admission latch; stop admitting work and
   signal owners concurrently before waiting. Preserve the existing 30-second
   aggregate application budget and five-second native escalation inside it.
2. Bound decoded HTTP bytes, individual lines, retained data, formatting and
   parser work between cancellation points. Count comments, invalids and
   duplicates before discarding/deduplicating. Never install a partial parse.
3. Retain explicit ownership of listener pools, engine workers, accepted command
   outcomes, blocking work and log-writer settlement. Queue admission is not
   acknowledgement of applied configuration, and watcher join is not quiescence.
4. Keep uncertain cleanup unclean and retain original/recovery evidence.
   Destructive takeover must not equate database lease expiry with termination
   of the previous filesystem mutator.

Exact telemetry backpressure behavior, byte/cardinality limits, command
cancellation semantics and any additional lifecycle evidence need explicit
approval after their remaining experiments. The lab's suggested fatal response
to log-queue overflow has an availability/DoS tradeoff and is not adopted or
presented as measured production policy here.

One important acceptance gap remains explicit: a 45-second external pod grace
does not prove process cessation at the 30-second application boundary when
the runtime is blocked. An independent enforcement mechanism at that boundary,
or an explicit operator-approved amendment of the deadline's semantics, must be
part of the final proposal and package evidence. Do not relabel late external
cleanup as a successful 30-second shutdown, add an unapproved sidecar/wrapper,
or claim a hard wall-clock guarantee through kernel or supervisor failure.
The next S2 experiment must test that actual containment/enforcement boundary
and maximum retained-work cleanup, not repeat the already-proven timer hazard.

The full report, extraction identities, source/lock, commands, logs, 18-trial
results and cleanup record remain local under the decision-evidence `s2/`
directory. The report distinguishes accepted 512-514 constraints from proposed
lease-safety inequalities; no lease, heartbeat, expiry or retry value is selected
from these timing samples.

## Other Decision Investigations

The following are work still required for this package, not requests to approve
unknown values. Their linked accepted constraints remain unchanged.

- **E1/C1:** The exact candidate and early boundary are above. Resolve actual
  digest-bound artifact delivery, real environment admission and current-source
  dual-native package proof. Retain architecture and source mismatch evidence
  rather than calling a development image a release.
- **S2 and 512-514:** Produce a bounded-work/termination argument and experiment
  for the actual watcher, then a compatible proposal for downstream completion,
  recovery/lease/heartbeat/cleanup and retry budgets. Preserve existing approved
  deadlines; do not infer a hard bound from elapsed samples.
- **516/535:** Resolve traversal persistence, aggregate encoding and membership,
  rename/deletion/rescan semantics, and limit outcomes alongside representative
  scheduling, fairness, traversal and hashing measurements.
- **515:** Select candidate product intents, versioned presets, processing order,
  bitrate meaning and tolerances for measured fixture comparisons and an
  operator listening decision. Do not silently substitute preserve-only audio.
- **585:** The corrected boundary inventory is established above. Finish actual
  stack-position reconciliation and remaining asset acceptance evidence before
  requesting the bounded exception; do not weaken the canonical guard.

## Execution Prerequisites

Read-only preflight on 2026-09-11 establishes the following, without a source
upload, remote mutation, workflow dispatch, or package publication:

- GitHub CLI authentication is available. The repository self-hosted runner
  endpoint returns zero runners. This does not establish that GitHub-hosted
  runners are unavailable or that their package jobs have run.
- Local Docker supports isolated PostgreSQL experiments; unrelated MCP and
  build containers are not owned by this task and remain untouched.
- Sonar CLI 0.10.0 `auth status` validates the keychain credential. Its displayed
  default organization is `tsoa-next`, while the explicit component metadata
  request identifies `VannaDii_Revaer` in organization `vannadii`. Project read
  access works; agentic-analysis entitlement and write scope are not proven.
- `SONAR_TOKEN` is absent from this task's process environment. A functioning
  CLI keychain connection is not proof of scanner credential injection. No
  credential value was printed or copied into an evidence file.
- Additional source uploads still need their separately enumerated consent.
  No analysis command was run, no analysis result is claimed, and no settings,
  exclusions, issue dispositions, or required checks were changed.

The Sonar preflight commands were `just --command sonar auth status` and
`just --command sonar api get '/api/components/show?component=VannaDii_Revaer'`.
Resolve the actual analysis connection and scanner injection after the scoped
upload consent, rather than treating read access as full analysis readiness.

## Task Record

- Motivation: Replace repeated approval-blocker reports with a finite,
  evidence-backed consolidated decision package while retaining full scope.
- Design notes: Candidate changes execute only inside disposable databases;
  package and shutdown investigations use disjoint isolated worktrees. No
  unapproved candidate enters committed production sources or ordinary tests.
- Test coverage summary: Database matrix and limitations are above. This is an
  intermediate research checkpoint, not a clean handoff or completed package.
  Full CI/UI, authoritative Sonar and package acceptance are not established by
  these experiments. `just docs-index`, `just instruction-drift`, all 1,263
  documentation links, and `git diff --check` pass. `just docs-build` exits zero
  but retains the large search-index WARN, so it is not warning-free. The exact
  commands and output are retained with the local evidence. Full `just ci` and
  `just ui-e2e` were not rerun for this intermediate research checkpoint.
- Observability updates: Only local experiment evidence and explicit decision
  provenance; production telemetry, levels, readiness and health are unchanged.
- Status-doc validation: Reviewed the specification's first-release scope/open
  questions, existing approval register, verification matrix and completion
  ledger. No operator support claim or accepted behavior is changed.
- Risk & rollback plan: Misreading experimental success as approval is the main
  risk. Keep this package Proposed until an actual operator decision, retain
  original failures, and remove only owned temporary resources. Preserve the
  primary checkout's existing staged changes and conflicts.
- Dependency rationale: No new dependency. The probe reuses the existing Ruby
  transition-proof transport and pinned local PostgreSQL image.
- Stale-policy check: Reviewed AGENTS.md, data/DevOps/Sonar scoped instructions,
  ADR template, 559's G1 boundary and 587. Research permission does not rewrite
  existing approvals or weaken scope. Earlier D5 evidence remains historical;
  the newly reproduced integer-ID failures are an explicit proposed extension.
