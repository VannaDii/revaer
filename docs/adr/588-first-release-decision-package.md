# First-Release Decision Package

- Status: Proposed
- Date: 2026-09-11
- Operator approval: Pending for all new production decisions in this package.
- Supersedes: None. Existing approvals, conditions, and holds remain binding.
- Implementation status: Proposal prepared; no production implementation or
  release qualification is established by this evidence package.
- Ready for consolidated operator review: Yes, for the exact choices and
  explicitly conditional qualification below. No approval is recorded here.

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

This record consolidates the exact proposed approval delta. The
[approval register](../media-approval-register.md) remains navigation to actual
operator decisions; the [completion ledger](564-media-completion-ledger.md)
remains the single requirement-to-evidence tracker. There is no new release
scope or parallel status ledger. The first database/package/watcher experiments
are pinned to integration `059cca9d7af584c14fd78c1b920ac821fcb7762d` unless stated
otherwise. The follow-up delivery, shutdown containment, audio, discovery,
asset replay and prerequisite work is pinned to
`fcdd95596fa062de9388d1be4f2ac96bbff571d1`; every evidence set retains its own
source, dirty delta, commands and limitations. No historical image is presented
as either of those current-source packages.

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
investigated. The consolidated choices below are requests, not consent.

1. Review the exact bundled choices once. Record the actual answer for each
   named decision and the separate upload scope. Existing accepted decisions
   are not submitted for reapproval; conditional D3 is still an evidence gate.
2. Repair the earliest mergeable boundary: reconcile the linear stack, apply
   only approved database corrections, complete the single-init certificate,
   and make root/profile configuration work through the real API/UI. Complete
   one manual discovery -> inspection -> planning dry-run with byte-identical
   originals. Preserve both sides of every divergent branch until reconciled.
3. In parallel, implement the approved environment, compliance installation,
   native broker and independent lifecycle owner. Build and verify both native
   architectures. These are prerequisites for a real mutating operator journey,
   not a reason to wait before fixing independent configuration/database work.
4. Integrate execution -> independent output verification -> safe replacement
   -> recovery, with explicit operator opt-in from default dry-run. Prove
   original preservation and crash/late-commit/fencing behavior before enabling
   destructive work. A single successful conversion is only a milestone.
5. Close the complete included matrix: watcher/scheduled discovery, every
   supported operation/codec/layout/rate, selected audio contracts and listening
   acceptance, operator controls, installation and backup/recovery rehearsal.
   Missing tests cannot silently narrow the first-release specification.
6. Merge the reviewed dependency-ordered stack only with the canonical size
   gate, clean full CI/UI, all applicable GitHub checks, strict Sonar with
   positive published coverage and resolved feedback on the exact revisions.
   Keep conventional titles, operator assignment and Copilot requests current.

One integration owner coordinates disjoint worktrees. Completed evidence is
retained before their removal; test media is deleted after each turn. The known
choices below now have exact recommendations. Newly discovered or materially
changed architecture still requires approval; this is not a claim that future
implementation can uncover no new issue.

### Consolidated Approval Choices

The linked sections and appendices define the exact proposed contracts. A row
is not a request to choose unspecified values. Approving a design authorizes
implementation within that design, not release, successful qualification or
removal of a conditional hold. A failed implementation test is fixed within
the approved contract; only a genuinely necessary change to that contract
returns for a new decision. In particular, no unmeasured timing, storage or
audio assumption becomes a support claim merely because its candidate was
approved.

| Decision | Recommendation And Consequence | Dependencies / Acceptance Boundary |
| --- | --- | --- |
| D4 + D5 ingestion family | Drop the policy scratch table on commit; add the existing partial-index predicates to IMDb, TMDB and TVDB ingestion conflicts. No wider schema redesign or multiple calls per explicit transaction. | Complete conditional D3, constrained-role, cold/warm, rollback and bootstrap proofs before single-init cutover. Frozen reference remains unchanged. |
| C1 + C1-D | Fail the entire application before infrastructure starts if compliance is invalid. Install the exact verified immutable nine-file bundle separately and mount it read-only, bound to one platform-image digest. Operator setup now requires prepared storage; heterogeneous Helm installations use separate architecture-pinned releases. | Real signature/predicate verification, descriptor/race/durability tests, actual early bootstrap, PVC/install and both native packages. No fabricated digest, installer sidecar or degraded fallback. |
| E1 | Select the exact 214-byte nine-key candidate, root-owned read-only `/app/media-home` and manifest layout below. Preserve the unrelated account home and all existing isolation rules. | Requested selection is conditional on real current-source amd64/arm64 closure and broker evidence before activation. Historical arm64 probes do not discharge the existing package hold. |
| S2 | Adopt the independently scheduled same-binary Linux lifecycle owner and exact bounded control, cancellation, completion and restart contracts below. Preserve 30 s application, 45 s external and at-most-5 s native deadlines; distinguish forced/unknown exits from clean shutdown. | Depends on E1/B1-B3 and actual process containment. Full native package, saturation, blocked-I/O, recovery and timing qualification remains mandatory; finite samples cannot guarantee progress through kernel failure. |
| LIFE-1 / 512-514 | Couple 5 s renewals/40 s shared expiries with affirmative prior-owner quiescence; five bounded burst retries then 60 s half-open recovery, plus the precise retained cleanup rules. Expiry alone never permits mutation. | S2 and generation-fenced database implementation; late commits, lease loss, fairness and uncertain ownership tests. Failed assumptions close admission, not soften authority. |
| DISC-1 through DISC-7 / 516,535 | Persist a normalized restart-safe frontier; use exact versioned aggregate bytes, bounded read-only helpers and the complete initial/hard quota tuple. Initial primary/aggregate caps are 256/260 GiB; the explicit hard envelope is 1 TiB/1,040 GiB. | E1/S2/LIFE-1, immutable bindings and normalized init. Prove real multi-replica fairness, slow/fast storage, mutation/cancel/restart and intended limit outcomes. The 16 MiB/s qualification assumption is not measured support. |
| AUDIO-1 / 515 | Select the three explicit `/2` music/dialog/speech intents, exact processing/measurement tolerances and configurable per-channel bitrate/cap semantics. Music's measured scalar path is an explicit two-pass-contract delta. | Full codec/layout/rate and independent-meter qualification, short-form cases and operator listening acceptance before activation. Keep failed `/1` evidence; do not relabel it as `/2` success. |
| ASSET-1 / 585 | Permit only the exact 213 binary deletions on PR 130, while ordinarily counting/reviewing all text. Fresh provider identity/history is mandatory and first close/merge expires the exception permanently. | Explicit criteria exception with consent and guard/instruction changes together. No other binary/ancestry/size, Sonar, review or required-check relaxation. Asset/runtime/mobile validation remains required. |
| External transfers | Approve the separately enumerated GitHub, exact Sonar project and gated existing GHCR release destinations for approved implementation iterations. | Separate affirmative consent, no secrets/test media/unpublished experiments, no criteria/server/billing changes or deployment. Read access alone proves no scanner/write entitlement. |

Full numerical/control details remain in the normative sections; this table is
not permission to substitute a shorter interpretation. If the operator changes
one row, reconcile only its actual dependency closure before implementation.
Do not silently treat omitted rows as accepted.

### Updated Goal

Complete and merge Revaer's full first-release media transcoding feature as a
production-usable v0 service under MEDIA_TRANSCODING.md and operator-approved
ADRs. First finish one evidence-backed, decision-complete approval package for
all known architectural holds and execution prerequisites; keep experiments
isolated, unpublished and outside production, and obtain explicit approval for
the exact choices plus separately scoped external transfers. After approval,
deliver configuration, discovery, inspection, planning, execution, verification,
safe replacement and recovery with dry-run enabled by default. Finish the
single-init cutover and qualify native Linux amd64 and arm64 packages. Require
real end-to-end, failure/recovery and original-preservation evidence, clean
`just ci` and `just ui-e2e`, all applicable GitHub checks, strict Sonar with
positive published coverage and resolved feedback. Deliver one linear,
reviewable stack within the canonical changed-line limit, assign VannaDii and
request Copilot review. Prioritize the next mergeable deliverable, parallelize
independent approved work under one integration owner, preserve user changes,
remove completed worktrees and delete test media after every turn. Do not
invent approval, weaken criteria, drop included features or claim perfection.

This is the active goal's scope and sequence, not a replacement that discards
unfinished work. Package approval is the transition into implementation;
qualification is the transition into a supported release. An unavailable
credential, runner or review is an execution problem to resolve, not a new
architectural choice or proof that the feature is complete.

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
activation still needs the real candidate in both current-source native
packages and the original descriptor, clock, containment and fixture acceptance
requirements. The consolidated request is to approve these exact values for
implementation **conditionally on that qualification before activation**, not
to assert that package evidence already exists or remove the E1 evidence hold.
This requested conditional sequencing is explicit; it is not inferred from
B1-B3 approval. Keeping the original values or creating the unused
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

The follow-up below proposes the missing delivery contract explicitly. It is
not C1 approval or completed production implementation.

All 16 owned containers and prototype fixtures were removed. The existing
`just image-compliance-test` fixture mutation suite passed, without certifying
a real release bundle. Local evidence, exact scripts, both failed attempts,
final results and the 211-file checksum manifest are retained under the
decision-evidence `package-startup/` directory. No image build, pull, upload,
registry publication or workflow dispatch occurred. Adoption requires the
consolidated approval and full package/service gates; rollback of this
experiment is removal of owned disposable artifacts, not a production change.

### C1-D: Exact Read-Only Delivery Proposal

Recommend the early C1 failure boundary together with a separately prepared,
immutable compliance directory for each architecture image. A trusted
operator-invoked Just installer must run the existing signature/attestation
verification and unchanged canonical compliance validator itself. It must bind
the authenticated subject and complete predicate to the exact supplied image
and manifest. A nonempty saved verifier file or caller-supplied success flag is
not verification; reject duplicate JSON keys and do not drop comparison fields.
[Sigstore distinguishes signature verification from predicate-policy validation](https://docs.sigstore.dev/cosign/verifying/attestation/).

Compose the existing v2 manifest and five referenced artifacts with the same
image's original static files, producing exactly nine runtime files:
`SOURCE-OFFER.txt`, `THIRD-PARTY-NOTICES.md`, `build-inputs.env`,
`declared-runtime-inventory.spdx.json`, `exiftool-exception.md`,
`final-image-compliance-bundle.json`, `image-package-inventory.spdx.json`,
`media-runtime-inventory.spdx.json`, and `source-compliance.json`.
Require byte equality for overlapping notices/build inputs and for declared
inventory versus the original image inventory. Do not source these originals
from an unrelated checkout or hide required static files with an incomplete
directory mount.

Use descriptor-anchored private staging, complete no-follow regular-file
snapshots, exact hashes, an exclusive installation claim and no-replace
publication. Validate the staged closure before installing it. Store each set
under `<image-sha256-hex>/<manifest-sha256-hex>/`, root:root directories `0555`
and files `0444`, and mount it read-only at `/app/compliance`. Finish and release
all installer write access before application admission. Never mutate an active
set or silently retarget a `latest` link. A post-rename durability error leaves
an unreferenced set for reconciliation, not a successful activation receipt.

The proposed Helm inputs are explicit, with no working invented defaults:

| Input | Required Meaning |
| --- | --- |
| `image.digest` | Exact verified platform-image `sha256:` digest, not a tag or substituted multi-platform index digest. |
| `image.architecture` | `amd64` or `arm64`, also enforced through `kubernetes.io/arch`; conflicting selectors fail. |
| `compliance.existingClaim` | Separately prepared existing PVC, read-only in both volume and container mount declarations. |
| `compliance.manifestDigest` | SHA-256 of the selected manifest; selects the immutable subPath and pod-template checksum annotation. |

Reject a simultaneous nonempty image tag or caller override of the reserved
compliance checksum annotation; preserve unrelated node selectors. Docker
uses the equivalent operator-managed read-only bind/volume. One Helm
deployment is pinned to one architecture; heterogeneous installations use
separately configured releases. Both supported native architectures remain in
scope. Requiring prepared read-only storage and explicit architecture changes
operator setup and availability, so these are **new C1-D approval deltas**, not
silent details of C1 or G1. No registry client, installer or new init sidecar
runs inside the application; E1's startup-clock contract is unchanged.

The final isolated delivery run passes **20 cases**: two accepted, eighteen
rejected, including signed fixture statements with a failed release gate or
empty inventory rejected before installation. A real historical arm64 container
reads all nine expected byte hashes with root ownership, `0444` files and a
`0555` directory through a mount reported read-only; append, delete and create
attempts fail. The separate root copy helper is removed before those reads.
An isolated chart copy passes **13 local Helm cases** for architecture/digest
binding, exact subPath, read-only declarations and invalid-value rejection.
Independent review found no material evidence overclaim in this scope.

These are synthetic signed-statement models and fixture metadata, not actual
Cosign/Fulcio verification, matching release-image content or Revaer startup.
The Linux image is the historical immutable ID recorded above; its config ID
is not the synthetic fixture's registry digest. The 16 MiB tmpfs is only a lab
cap and provides no durability proof. The chart was rendered, not installed;
no cluster was contacted. The model's absolute paths/final rename are not
production descriptor/race/no-replace proof. Original failed runs, fixes, exact
commands/hashes, model/manifest bytes, chart deltas and cleanup are retained
under local `decision-evidence-b/compliance-delivery/`.

Full corresponding-source archives, specification-required `/usr/share`
notices, licenses, inventories, OCI annotations and all release gates remain
mandatory. This nine-file delivery set does not substitute for them. Actual
signed current-source amd64/arm64 packages, hardened installer fault tests,
real PVC behavior and actual early bootstrap are required after approval.
Rollback selects the previous jointly verified image and immutable set;
failure never authorizes a C1 bypass or mixed image/bundle pair.

Alternatives are baking incompatible post-build bytes into the image, adding
runtime retrieval/lifecycle behavior, or using immutable ConfigMaps. The latter
has a documented 1 MiB object ceiling and would require a separately justified
size/fragmentation contract; its subPath mounts do not receive live updates.
The prepared filesystem set avoids that additional artifact-size assumption.
[Kubernetes ConfigMap constraints](https://kubernetes.io/docs/concepts/configuration/configmap/).

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

### ASSET-1: Content-Bound Replay And Permanent Expiry

Recommend replacing 585's repeated SHA-by-SHA approval requirement with this
**explicit, still-unapproved criteria relaxation for one deliverable only**.
Do not generalize it to another PR, binary type or partial deletion. Binary
identities are fixed; text changes remain ordinarily counted and reviewed.

1. The exception belongs only to GitHub repository node `R_kgDOQJiaFw`
   (`VannaDii/revaer`), PR 130 node `PR_kwDOQJiaF8749-ef`, same-repository head
   `stack/media3-53-sonar-asset-inputs`. Keep that PR in the single linear chain
   at its existing asset ownership point after the generated API schema and
   before its downstream consumers. Repair ancestry and replay intervening
   prerequisites first. Do not use an ancestor-to-tip comparison to qualify it.
2. Match **all 213 binary deletions** with exact path, mode, blob, SHA-256 and
   byte count, and no other binary/uncountable entry. Recompute from immutable
   Git objects with `--no-renames`; require immediate base/head ancestry.
   Partial, extra, changed, renamed or nonregular binary entries fail.
   Count **every** text addition/deletion, including future bug fixes, guard
   code, instructions and records, under the unchanged 9,999 maximum. No
   text path, generated file, deleted SVG or ancestor comparison is exempt.
3. Allow changed base/head identities when the exact binary inventory remains
   identical and fresh provider evidence binds that pair to this open PR.
   The corrected 104-path text manifest is the recommended asset implementation
   and review evidence, not a permanent whole-blob text allowlist. Text remains
   subject to ordinary architectural authorization, asset ownership, full checks
   and PR review at its current SHA. This distinction allows normally counted
   instruction updates and rebases without manufacturing repeated binary consent.
   A text change never receives architectural approval from this accounting rule.
4. Read authoritative PR identity, refs and complete close/merge/reopen history
   through the GitHub CLI/API at each canonical exception evaluation. No cached
   success, caller-supplied "verified" flag or synthetic JSON is authority.
   Pagination must finish; unavailable or malformed evidence fails closed.
   A local unpublished candidate can receive diagnostics, not a canonical pass
   against an imaginary provider head. No extra token privileges are proposed.
5. Expire permanently at the first close or merge of PR 130. Reopening, changing
   branch names, creating another PR or restoring a prior commit cannot reactivate
   it. Remove the exception through a separately counted cleanup after landing.
   A concurrent close between checks cannot authorize another PR; before merge,
   re-read current provider state and the exact checked SHAs.
6. Implement the guard, actual consent record and scoped instruction update in
   one normally counted, reviewed change at the earliest owner that needs it.
   Including them in PR 130 consumes that PR's text budget; placing them in a
   prerequisite requires its own ordinary passing boundary. Each successor keeps
   the ordinary guard. This exception changes review accounting only; asset
   correctness, Sonar scope, all checks and review remain mandatory.

On 2026-09-11 the live API still reports the original base/head shown in 585,
an open same-repository PR, and **no close/merge/reopen events**, with complete
pagination. No provider state changed. A local experiment rederived both real
Git inventories and passed **33 predicate-model cases plus two numeric-boundary
checks**, including extra/missing/drifted binary entries, uncountable/negative
text counts, the 9,999/10,000 complete-predicate boundary, wrong repository/PR,
non-ancestor, unavailable metadata, incomplete history and permanent expiry
after reopening. Its changed-endpoint case is a synthetic
identity model, **not** an executed Git rebase or authenticated future head.
The original PR matches the proposed accounting predicate but still has the
documented asset defects; accounting is not asset acceptance. The current
canonical guard still rejects these binary entries. Run 01 retains the rejected
whole-text-blob model; run 02 is this revised recommendation.

This selects the already corrected deliverable and a precise restack allowance;
it does not certify the final reconstructed stack. The alternative is fixed-SHA
approval again after every ordinary replay, or retaining the existing binary
block. The proposed allowance avoids repeated identity-only decisions without
authorizing a changed binary deletion or exempting text review. Remaining
desktop/mobile/platform-icon and
licensing acceptance are implementation/release gates, not waived conditions.
Rollback is removal of this one exception and fail-closed binary handling;
rejected visual behavior must be repaired and reviewed, not concealed by the
exception. Any changed binary inventory requires a new decision.

The earlier raw inventories and failed canonical guard output remain in the
first decision-evidence package. The follow-up source, fresh API response, raw
Git inventories and predicate results are retained in
`decision-evidence-b/asset-replay/run-02/`, with run 01 retained. The
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

The full report, extraction identities, source/lock, commands, logs, 18-trial
results and cleanup record remain local under the decision-evidence `s2/`
directory. Those earlier watcher/timer findings were not rerun or relabeled as
packaged enforcement. The follow-up below answers the remaining design question.

### S2: Exact Lifecycle And Recovery Proposal

The [shutdown evidence and exact limits](support/588-shutdown-proposal.md),
[lifecycle control contract](support/588-lifecycle-control.md), and
[quiescence/recovery contract](support/588-quiescence-recovery.md) are normative
parts of this Proposed package. They select candidates, not production policy
or a claim of timing qualification.

Recommend a same-binary non-Tokio Linux PID1 owner, before application runtime
construction, supervising one application child inside its private containment
unit. The owner has no database/media/network/stdout/formatter work. Keep
application ownership/join receipts; the owner validates bounded manager
reports and waits adopted children, not live grandchildren owned by a manager.
Do not describe a trusted app receipt as an independent kernel measurement.

| Exact New S2 Choice | Consequence |
| --- | --- |
| Whole-unit enforcement target 28.000 s | KILL request by 28.250 s, mutators reaped/absent by 29.750 s, observation/classification by the unchanged 30.000 s aggregate deadline. Final 2 s comprises 250 ms dispatch, 1,500 ms exit/reap and 250 ms observation, not extra application work. |
| Native enforcement target 4.750 s after its cancellation origin | Request KILL by the existing at-most-5 s limit, clipped by earlier request/incident/drain/lease clocks. Group uncertainty escalates earlier, not only at 28 s. |
| 1,024 total Linux tasks | Includes every application/native/helper thread and descendant, not merely processes. Fixed owner registry is 1,024 x 128 bytes. Full-spec work must fit the qualified envelope with growth reserved before side effects; existing thread settings are not thereby certified. |
| Two bounded anonymous control pipes | Exact 64-byte records and 14 closed opcodes; at most 64 outstanding records per direction; actual pipe capacity 4,096-65,536 bytes. Validate registration, request identity, nonce and original deadlines. ACK is receipt only; only verified settlement releases occupancy. |
| Separate request and broker lifetime settlement | A normal verified request can leave its broker alive and reusable. Native tools remain in the broker group under B1-B3/RVB1. Cancellation/ambiguity requires complete group settlement and existing B3 rules, not a new per-tool group or extra lane. |
| Owned bounded telemetry and configuration work | Wire/decoded blocklist cap 16 MiB, physical line 4 KiB, all-lines cap 1,048,576, parsing quantum 4 KiB. Log record 4 KiB, 16 bounded scalar fields; active+pending logger 1,024 records/4 MiB; bounded SSE history. Configuration retains 128 pending + 1 active + 1 preparation, 16 MiB single/32 MiB aggregate payload. Exact byte/field/cancellation rules are in the appendix. |
| Failure and exit classification | Internal SIGUSR1 is an unclean-stop latch when the ordinary control path cannot deliver a fatal record. Exit 0 is clean verified shutdown, 70 forced but absence proved, 71 deadline/absence unknown, 72 control/ownership invariant failure. Force is never a clean pass. Logger overflow or unflushable sink triggers unclean drain, not silent loss. |
| Clean planned-restart handoff | After irreversible mutator/broker/helper settlement and owner sealing, publish one normalized fenced quiescence batch before DB closure and the 28 s cutoff. Successor acquires original root lock/attestation, atomically consumes the epoch-bound handoff once, then performs complete recovery before opening admission. |
| Lost/uncertain quiescence recovery | Use the exact new authenticated per-root confirmation endpoint and normalized audit record in the appendix. The operator must establish predecessor termination and exclusive original-root authority. Confirmation is not a reset or direct ready flag. Without valid handoff/surviving proof, crash recovery needs this operator action. |

The final bounded Linux experiment used the immutable historical arm64 image,
private namespace/cgroup, 1,024 total tasks, one-CPU pressure, 1 GiB memory cap
and 16 MiB scratch cap. It created one leader and 1,022 new-session,
TERM-ignoring busy descendants. Admission of a 1,025th task failed as expected.
No current Revaer package, production configuration, native five-second phase,
RVB1 control codec or database handoff ran in this experiment.

The first larger run failed: diagnostic filesystem/cgroup sampling blocked the
deadline loop; KILL was requested at 34.797679723 s and **no descendant wait or
absence was established**. Its raw timestamp labels do not turn an attempted
census into observed settlement. The parent allocated one corrective run under
the user's isolated-research authority, removing diagnostic I/O from the
deadline loop and using the actual Linux namespace broadcast signal semantics.
That run requested KILL at **28.076101429 s**, collected all **1,023 exact
SIGKILL wait statuses**, observed ECHILD and confirmed namespace absence at
**28.396035429 s**. Classification finished at 28.396035721 s; the cgroup then
contained only PID1 and reported no OOM. All owned resources were removed.

This supports that minimal mechanism at the tested envelope, not a universal
bound. The same run serviced an earlier 15 s repeat-action target at
16.775503591 s; it does **not** establish a universal 250 ms scheduling bound.
Both failed and corrected runs, scripts, all identities and raw results remain
in `decision-evidence-b/shutdown-bound/`. Native amd64/arm64 full-service stress,
blocked-I/O, thread growth, ownership, control-codec, real restart and deadline
acceptance remains required before activation. Kernel/storage failure can
prevent userspace progress; such cases are failures with closed root barriers,
never a relabeled 30 s success or permission for lease-only takeover.

This proposal changes process topology, protocol, bounded input/log behavior,
failure availability, and durable recovery schema/API. All are explicit S2
approval deltas, not G1 refactoring. The normalized handoff includes a fresh
batch identity, exact root/catalog/binding/recovery identities, monotonically
increasing quiescence epochs and one-time successor consumption; the appendix
specifies complete fields, procedures, late-commit handling and crash tests.
Ordinary planned restarts should recover without manual confirmation only when
this new handoff is approved, implemented and proven. An uncertain crash still
needs external sole-writer evidence; the application gains no host-control
socket, privileged daemon or automatic lost-node fencing authority.

Alternatives rejected for this proposal are an async timer in the blocked
runtime, unconditional join, lease-expiry-as-quiescence, or counting external
45 s teardown as a successful application shutdown. No arbitrary new timeout
replaces the accepted budgets. Logger fail-closed behavior has an explicit
availability/DoS cost; capacity and exact rejection must be tested. Rollback
stops new admission, retains original media and durable recovery evidence, and
uses a previously qualified package with operator reconciliation where proof
is unavailable. It never claims a consumed handoff can be reused.

### LIFE-1: Coupled Lease, Recovery And Retry Values

The following are exact **proposed** values for the remaining 512-514 holds,
not a timing certificate or adoption of the older 503 proposal. The S2
containment contract and affirmative old-owner quiescence are prerequisites.

| Field | Proposed Contract |
| --- | --- |
| Attempt heartbeat, aggregate lease, root recovery leader and cleanup claim | Each separate fenced owner renews every 5 s with server-clock expiry 40 s. No identity is collapsed into another lease. |
| Stale detection/takeover eligibility | Server-proven expiry of the applicable 40 s lease, never a fixed one-hour heartbeat heuristic. New generation plus affirmative prior-owner/root-writer settlement is mandatory before mutation. |
| Lease/claim/renew/checkpoint transaction | End-to-end client deadline 2 s; server statement limit no greater than 2 s and lock wait no greater than 250 ms. Unknown or late commit requires fenced readback; a local timeout is not rollback. |
| Control, claim/contention cadence | Preserve the accepted 250 ms control poll and 1 s job-claim scan. Use the next fair 1 s opportunity for lease/leader contention without consuming an attempt. No random jitter. |
| Cleanup claim abandonment | Same 5/40 s renewal/expiry pair; preserve files/evidence and quarantine on uncertain cleaner settlement. Expiry alone cannot authorize a replacement cleaner. |
| Cleanup fairness cursor lifetime | Persist one stable keyset/service cursor per root until explicit root retirement and settlement of every referenced attempt/cleanup record. No time-based reset that rewards restarts. |
| Cleanup failed-candidate retry | Preserve eligibility at the next accepted hourly sweep, not a new fast deletion loop. The recovery barrier must first reopen. |
| Recoverable infrastructure failure | Initial attempt, then at most five burst retries after 1, 2, 4, 8 and 16 s. One in-flight control attempt; zero jitter. This is not a native-job retry or consumption of an attempt number. |
| After burst exhaustion | Remain `recovering`/`degraded`; wait 60 s, make one bounded half-open recovery attempt, and repeat that 60 s cooldown on failure. Do not restart the five-retry burst on each failed probe. The bounded policy limits attempt rate, not total lifetime probes. |
| Burst reset | Only after 60 s continuous successful relevant dependency health and, for active owners, successful 5 s renewals. A transient successful connection alone does not reset it. |
| New-incarnation initialization | After the existing startup contract succeeds, each newly constructed permitted recovery owner starts with its initial attempt and one five-retry budget. Retry state is volatile and scoped to that owner's process lifetime, not a durable deployment-wide rate limit. A new incarnation does not inherit, resume or revive an old caller or claim; root quiescence and fresh fencing remain mandatory. No new 60 s startup delay, app/task restart or B1/B3 retry is introduced. |
| Excluded failures | No retries through this policy for C1/E1 invalid startup inputs, fatal task loss, B1 broker initial failure, B3 replacement beyond its accepted contract, intentional drain, expired caller requests or unknown root quiescence. |

Preserve accepted initial job concurrency one, 1,024-row outbox pages with
complete keyset continuation, 1 GiB source staging reserve, immediate post-barrier
cleanup followed by hourly sweeps, 24/720 h workspace/diagnostic defaults and
128 examined cleanup candidates. Do not change the accepted larger ingress
bounds or introduce independently leased operation jobs. The 30 s aggregate,
45 s external and at-most-5 s native deadlines remain unchanged.

The conditional shared inequality is
`5 + 0.25 + 2 + 0.25 + 30 = 37.5 < 40 seconds`: heartbeat interval,
scheduler-delay allowance, database transaction, clock/observation allowance
and sufficient settlement time. Discovery/read-only owners separately propose
`5 + 0.25 + 2 + 0.25 + 7.5 = 15 < 20 seconds`. The two 250 ms allowances and
settlement terms are **qualification assumptions**, not measured universal
bounds, added clock slack or extra shutdown time. The corrected S2 mechanism
settled its test case within 30 s, but neither it nor the earlier failed run
establishes that term for the integrated service or all supported packages.

Use the earliest caller, drain, safe-lease and operation deadline. Do not start
a retry or work step unless its complete bounded operation and necessary
settlement fit strictly inside remaining authority; renewal never resets the
caller's or shutdown clock. Failed assumptions revoke authority, retain capacity
and keep the root barrier closed. Neither this arithmetic nor a newer database
generation can terminate a stale filesystem writer.

The ignored `lifecycle_values.rb` run passes 34 arithmetic/state checks for
strict slack, burst/cooldown/reset boundaries, earliest-deadline refusal and
takeover preconditions. It creates no service or media. Quiescence is an input
to that model, not something the model proves. Database outage/load, late-commit,
fencing, cancellation, cleaner-death, fairness and actual packaged settlement
tests are mandatory implementation gates. These tests and the complete
conditional D3 proof remain prerequisites to cutover, not newly waived holds.

The alternative one-hour stale heuristic delays recovery without proving
quiescence; endless rapid retries increase database pressure; permanent manual
recovery after an ordinary short outage unnecessarily harms availability. The
chosen cooldown permits automatic dependency recovery while uncertain physical
ownership still requires the separate recovery authority. Rollback preserves
closed barriers, snapshots and evidence; it never restores lease-only takeover
or deletes work to compensate for a failed new implementation.

## AUDIO-1: Full Versioned Audio Contract

The [exact audio proposal and evidence appendix](support/588-audio-proposal.md)
is part of this consolidated approval package, not a separate accepted ADR.
Its `/2` versions are new proposed contracts; the retained `/1` experiments
must never be relabeled as executions of those later contracts.

| Proposed Preset | Integrated LUFS, Non-Mono / Mono | Range / Transform |
| --- | --- | --- |
| `music-linear/2` | -18 / -21 | Measured scalar gain only; preserve LRA within 0.5 LU PCM / 1.0 LU lossy; reject peak-infeasible gain, no dynamic fallback. |
| `dialog-balanced/2` | -18 / -21 | Anchored, linked RMS compression 2:1, attack 20 ms/release 250 ms; measured two-pass normalization, LRA control 11 LU / final ceiling 12 LU. |
| `speech-focused/2` | -16 / -19 | Anchored, linked RMS compression 3:1, attack 10 ms/release 150 ms; measured two-pass normalization, LRA control 7 LU / final ceiling 8 LU. |

The appendix selects BS.1770-5/Tech 3342, every compressor/resampler control,
exact measurement order, inclusive PCM/lossy tolerances, -3 dBTP processing
budget with -2.8 PCM/-1 decoded-lossy ceilings, short-form outcomes, independent
meter agreement and human listening acceptance. It preserves configurable
sample rates and all included codec/channel/layout controls, rather than
turning the three measured encoders and 48 kHz into a release allowlist.

Bitrate means explicitly selected encoder-default or exact rational nominal
bps per realized output channel, with a separately optional packet-time mean
cap. Include LFE in channel count; enforce codec-native legal totals, exact
overflow-checked compilation and packet coverage. No nominal lower acceptance
bound, symmetric +/-5% window, hidden rate rounding or cap slack is selected.
Existing ambiguous stored rows require explicit mapping/replanning, never
reinterpretation. The exact input/default/domain and acceptance changes are
part of this new approval request, not G1 refinements.

Scalar A explicitly changes ADR 515's two-pass-transform requirement while
retaining required analysis, PCM and decoded-final checkpoints. B/C retain
identical anchored/compressed two-pass inputs. Failed PCM, measurement, source
identity or final-byte verification always prevents replacement. Immutable
preset versions, source hashes and package/measurement evidence remain coupled.

The local macOS arm64 FFmpeg 9.0.1 experiment produced 42 main encoded outputs:
25 passed encoded-only checks and 19 also passed their PCM checks. Six bounded
scalar checks passed the measured subset. These are not full `/2` passes or a
reliability percentage. Fixed recipes failed high-range inputs; `linear=true`
performed dynamic processing on zero-range material; 13 PCM/decoded records
exceeded the later 0.5 LU inter-meter LRA proposal. Independent readback
verified 173 measurement records and 147,632 packets across 48 packet logs,
not meter conformance. The one real excerpt is not a representative listening
corpus. Raw failures, source/tool hashes, previous proposal and command errors
remain in `decision-evidence-b/audio/`; its 725 checksummed files contain no
test media and all 1,861 tracked source hashes remained unchanged.

Objective native-package qualification and explicit human listening acceptance
are both required, including the exact listener/corpus criteria in the appendix.
The listening helper was syntax-checked only and is not a complete acceptance
harness. No listening result, short-form conversion, Linux package equivalence,
full fixture gate, CI/UI or Sonar pass is claimed. Approval would select the
contract for implementation/qualification, not activate unverified transforms.

Alternatives are the hidden legacy recipes, arbitrary raw filters, or omitting
normalization/compression. The recommendation keeps the full feature with typed
controls and visible infeasible-input failures. Rejection or rollback keeps
affected transforms held, preserves originals/checkpoints and never substitutes
preserve-only behavior for an explicit transformation request. Parameter or
algorithm changes require another immutable version and explicit revalidation.

## DISC-1 Through DISC-7: Discovery And Fingerprint Contract

The [discovery proposal](support/588-discovery-proposal.md) and its
[complete numeric table](support/588-discovery-values.md) are normative
appendices to this one approval package. DISC labels do not reuse the approved
database D1-D3 names. These new choices remain Proposed.

- **DISC-1:** Normalized run, directory-epoch and individual-name frontier rows;
  bounded staged enumeration and atomic, generation-fenced identity/intent/job
  publication with the consumption checkpoint. Persist no native directory
  cookie, serialized cursor or mutable policy head. A hash wait retains one
  pending candidate and its distinct reservation, outside the metadata quantum.
- **DISC-2:** Exact `aggregate_v1` bytes, SHA-256 and member/rule provenance in the
  appendix. Include the primary and every selected subtitle sidecar, including
  both VobSub files even if the target would remove them. Exact UTF-8 names remain
  distinct; aliases, ambiguous owners and incomplete membership reject the whole
  aggregate. Observations are separate from cryptographic content identity.
- **DISC-3:** Register immutable versions, add tested readers first, re-observe
  and hash before writer activation. Retain old snapshots; unsupported versions
  require explicit replan. Never transform old digest bytes into new identities.
- **DISC-4:** Durable coalesced rescans on uncertainty; two complete clean absence
  passes before a non-mutating tombstone; rename as old absence/new observation;
  explicit configuration activation triggers reevaluation without mutating
  already queued snapshots. Incomplete scans never certify disappearance.
- **DISC-5:** Durable instance/deployment/principal/root admission, idempotent
  reservation/settlement and fair bounded queues. Every discovery, claim and
  pre-mutation full read debits the rolling window. Cancellation never refunds
  the debit or prematurely releases uncertain physical capacity.
- **DISC-6:** Earliest-deadline composition with LIFE-1 and S2. Read-only owners
  use proposed 5/20 s renewal/expiry; shared mutators use 5/40 s. Affirmative
  old-root quiescence remains mandatory; elapsed time and database fencing alone
  cannot authorize a new filesystem writer.
- **DISC-7:** Exact initial/hard values, overload outcomes, observed limitations,
  implementation tests and rollback below and in the appendices. No smaller
  fixture subset substitutes for the full discovery feature.

Select a read-only same-binary helper mode before application/Tokio bootstrap,
under the existing 535 executor boundary, with at most two active helpers per
instance and four per deployment.
For each sequential member it receives only the retained read-only regular-file
descriptor, request/generation identity, exact expected observations/length,
earliest deadline and bounded cancellation/result channels. It has no path-open,
write, native-program selection, database or grandchild-spawn command. Use the
same selected E1 sanitized environment and close unrelated descriptors before
exec. Read only the reserved bytes in 64 KiB chunks; return monotonically bounded
progress (32-byte record maximum, at most once every 5 s) and one typed terminal
result (1 KiB maximum). Retain only the latest progress counter, not a growing
message history; the terminal result carries the exact final count and digest.
The parent revalidates complete membership/observations and owns wait/pipe
settlement before accepting the digest. This is a proposed helper transport
boundary, **not** a third RVB1 lane or permission to widen B1-B3 descriptor
inheritance. Internal codec implementation must preserve these exact semantics
and bounds; no subprocess sandbox or kernel-I/O interruption proof is claimed
from Ruby's local fork experiment.

| Selected Envelope | Initial | Hard |
| --- | ---: | ---: |
| Primary / complete aggregate | 256 / 260 GiB | 1,024 / 1,040 GiB |
| Per-sidecar / total sidecar bytes | 1 / 4 GiB | 4 / 16 GiB |
| Batch primary bytes | 1 TiB | 4 TiB |
| Member / active hash / whole request | 18,000 / 19,800 / 19,860 s | 72,000 / 79,200 / 79,260 s |
| Principal / deployment admitted bytes per rolling hour | 8 / 64 TiB | 32 / 256 TiB |
| Principal / deployment debits per hour | 4,096 / 32,768 | Same |
| Retained debit rows, principal / deployment | 8,195 / 65,556 | Same; expiry plus settlement before bounded hourly pruning |
| Run wall / cumulative metadata | 21 days / 6 h | 84 days / 24 h |

These limits are ceilings, not artificial waits. The named initial workload is
four hours of 100 Mbps video plus four 6 Mbps audio tracks and a 10% primary
allowance; the hard workload is eight hours of two 120 Mbps videos plus four
8 Mbps audio tracks and that allowance. The initial and hard size, sidecar,
batch, rolling-window and all-phase read arithmetic is reproducible in the
appendix. The 16 MiB/s qualification floor and illustrative 600 MiB/s rate
are **assumptions**, not measured storage guarantees. Faster completed work
does not wait out long maximum deadlines. Early drain/lease deadlines always win.

The retained real lab covers shallow/wide/deep traversal, restart-row models,
mutation/cancellation, inherited descriptor hashing, six concurrent hash
children and eight independently checked aggregate vectors. Its largest regular
hash fixture was 64 MiB and cache-rich; no multi-GiB sustained-read result is
claimed. Revision 2 adds arithmetic, not another media/throughput experiment.
Its 39 preserved files include the unchanged v1 package, failures, commands and
source/cleanup checks. No PostgreSQL transaction, production Rust helper,
multi-replica fairness, descriptor-relative scan or native-package proof follows.

The appendices require full grammar/member, database fence/late-commit,
concurrency, watcher uncertainty, cache invalidation, admission/byte/row-limit,
real sustained read, failure/restart and original-preservation tests. The
physical row cap fails closed when pruning falls behind; it never erases
unsettled evidence. Preserve all agreed scope and exact values during these
tests. Alternatives are unsafe timestamp/cookie identity, unbounded traversal
or process-local quotas; rejection does not authorize them. Rollback closes
automation/admission and preserves roots, reservations and immutable histories.

## Acceptance Work After Decisions

The following are implementation/qualification gates with defined expected
behavior, not unanswered choices or automatic follow-up approval requests.
Approval does not waive them. A failed result must be retained and fixed; if
the exact approved contract cannot be met, present only the materially changed
decision and its dependency consequences. Do not silently reduce the envelope,
weaken criteria, hide a failure or reapprove unrelated ADRs.

| Workstream | Required Evidence Before Claiming It Complete |
| --- | --- |
| Single-init / D3 | Full exact-reference/final transition certificate, caller/helper/branch matrix, constrained runtime authority, cold/warm sessions, rollback/late-commit tests and real fresh-install/profile/root workflow. Preserve reproduced reference defects as defects, not parity successes. |
| Environment / compliance / broker | Both current-source native images; exact environment bytes/ownership, complete native closure, real signature and post-build predicate binding, nine-file immutable installation, C1 before side effects, all B1-B3 timing/containment/descriptor cases and operational install/recovery. |
| Shutdown / ownership | Actual Rust lifecycle/control implementation, complete contributor accounting, native and whole-unit deadlines, forced versus clean outcomes, blocked-I/O and saturation matrix, lease-loss/late-commit/fenced takeover, clean restart handoff and uncertain-root recovery. Retain both failed and corrected isolated trials. |
| Discovery / admission | Real PostgreSQL atomicity and multi-replica accounting, complete byte-vector compatibility, restart/rescan/tombstone cases, directory and file races, no partial enqueue, helper cancellation and physical capacity settlement, exact quotas/fairness, declared slow/fast storage qualification. |
| Audio | Executed `/2` pipeline on every included codec/layout/rate and short-form boundary, independent calibrated meter agreement, exact packet/rate/cap accounting, real licensed listening corpus and explicit operator listening acceptance. No claim that `/1` fixture counts validate `/2`. |
| Full operator service | API/UI journeys from clean install through dry-run, real execution, verification, replacement and recovery; complete specification-to-evidence rows including all included operations. Crash/failure tests assert unchanged originals or the exact recoverable durable state. |
| Stack / release | Fresh actual-base/head ancestry and canonical sizes, conventional titles, assignment and Copilot review, all actionable feedback resolved, all applicable checks run and pass, full clean local CI/UI, strict published Sonar coverage, package/backup/restore/upgrade rehearsal. Do not use an earlier revision's result. |

The existing completion ledger and release verification matrix own the detailed
per-requirement results. This package does not introduce a second completion
ledger or declare any L1-L10 row complete. The known retained profile-creation
and UI coverage-teardown failures still require fixes and exact-revision reruns;
no research result here changes those application outcomes.

## Execution Prerequisites

Read-only preflight on 2026-09-11 establishes the following, without a source
upload, remote mutation, workflow dispatch, or package publication:

- GitHub CLI authentication and repository read/push permissions are available.
  Revaer is public, not archived/disabled, and Actions is enabled. The repository
  self-hosted runner endpoint returns zero runners. This is not evidence that
  GitHub-hosted capacity is absent.
- The unchanged image matrix selects native `ubuntu-latest` amd64 and
  `ubuntu-24.04-arm` arm64, without QEMU. Both labels are supported for public
  repositories; GitHub documents 4 CPUs, 16 GB RAM and 14 GB SSD for each.
  This verifies an available execution route, **not** job dispatch, actual free
  disk space, queue admission, completion within the existing timeout or package
  acceptance. [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
  Run the existing exact native matrix after approved source is published;
  retain actual runner/image IDs, capacity, duration and both native results.
- Local Docker supports isolated PostgreSQL experiments; unrelated MCP and
  build containers are not owned by this task and remain untouched. The current
  Docker daemon is Linux aarch64, with 5 CPUs and 33,599,119,360 memory bytes.
  The macOS filesystem has free capacity, but that is not the nearly-full Docker
  backing filesystem's capacity. No local amd64-native host was demonstrated;
  neither emulation nor the old arm64 image replaces both native package gates.
- `scripts/with-node.sh` selects and verifies NVM-managed Node **24.19.0**.
  No global Node installation, version change or package installation was made.
- Sonar CLI 0.10.0 `auth status` validates the keychain credential. Its displayed
  default organization is `tsoa-next`, while the explicit component metadata
  request identifies `VannaDii_Revaer` in organization `vannadii`. Project read
  access works; agentic-analysis entitlement and write scope are not proven.
- `SONAR_TOKEN` is absent from this task's process environment. GitHub's
  repository-secret metadata confirms a secret with that name, last updated
  2026-03-22. Its value was neither retrieved nor tested. A functioning CLI
  keychain connection and a secret's name are not proof of scanner credential
  injection, analysis permission, entitlement or token validity. No credential
  value was printed or copied into an evidence file.
- Additional source uploads still need their separately enumerated consent.
  No analysis command was run, no analysis result is claimed, and no settings,
  exclusions, issue dispositions, or required checks were changed.

The Sonar preflight commands were `just --command sonar auth status` and
`just --command sonar api get '/api/components/show?component=VannaDii_Revaer'`.
Resolve the actual analysis connection and scanner injection after the scoped
upload consent, rather than treating read access as full analysis readiness.
The fresh follow-up commands and outputs are retained in
`decision-evidence-b/prerequisites/run-01/`; all eleven read-only commands exit
zero. That count is not a passing analysis or release gate.

### Separately Scoped Upload Request

Architecture approval must not silently authorize these external transfers.
Request a separate affirmative decision for the approved first-release
implementation iterations through merge (or earlier operator revocation):

- Upload reviewed source commits and PR metadata to **VannaDii/revaer on
  GitHub**, and run that repository's applicable Actions/Copilot checks. Artifact
  uploads are the existing required nonempty test/coverage/native/scanner
  evidence. Do not upload private environments, secrets, acquired/generated test
  media, unpublished experimental harnesses or unrelated worktree contents.
- Submit the complete canonical scanner scope and required real coverage/native
  inputs to **SonarQube Cloud, organization `vannadii`, project
  `VannaDii_Revaer`**, and use the same exact project for necessary per-file CLI
  analysis. This request replaces neither the immutable criteria nor the
  requirement to inspect each scan for warnings, entitlement and published
  positive metrics. No account settings, rule activation, issue dispositions,
  billing plan, new service or credential exposure is authorized.
- Publish the approved release's digest-bound amd64/arm64 images, signatures,
  compliance predicates, corresponding-source evidence and Helm artifacts only
  to the existing **`ghcr.io/vannadii` Revaer image/chart destinations** through
  the reviewed release flow, after its verification/signing gates. This does
  not turn ordinary PR verification into publication or authorize a new registry,
  premature tag, unrelated package, deployment or failing-artifact release.

The earlier consent for `sonar verify --file
scripts/database_rebaseline/final_sql.rb --project VannaDii_Revaer` remains
scoped to that command. This proposed broader request is **not granted** by its
presence in an ADR. Connection failures after actual scoped consent are
execution issues to diagnose, not permission to use an unrelated project,
reduce analysis or ask the operator to repaste secrets.

## Task Record

- Motivation: Replace repeated approval-blocker reports with a finite,
  evidence-backed consolidated decision package while retaining full scope.
- Design notes: Candidate behavior executes only in isolated disposable labs,
  never production sources or ordinary tests. One parent integrates the exact
  database, package, ownership, audio, discovery and delivery proposals. The
  six normative appendices retain source/evidence limitations; the discovery
  copy explicitly attributes its final arithmetic revision to the parent
  agent, not operator direction or consent.
- Test coverage summary: The database, compliance, lifecycle, audio and
  discovery experiments and their failures are reported above. Independent
  review of the non-S2 consolidated approval boundaries found no material
  contradiction; S2 received separate integration review of raw waits, request
  versus broker lifetime, lost proof and clean-restart recovery. These are
  bounded reviews, not an exhaustive new code/security audit. Final
  documentation validation is retained in `decision-evidence-b/documentation/`.
  Full `just ci`, `just ui-e2e`, authoritative Sonar and current-source native
  package acceptance were not run for this proposal-only checkpoint and are
  not claimed. All remain mandatory implementation/release gates.
- Documentation checks: Index generation, instruction drift and whitespace
  validation pass. `just docs-build` exits zero but reports a large search-index
  WARN, so it is not a warning-free gate. The first link-check attempt failed
  on sandbox-denied external connections; the permitted retry and complete
  outputs are retained separately without changing link-check criteria. No
  index-size suppression or other criteria change was made.
- Observability updates: Only local experiment evidence and explicit decision
  provenance; production telemetry, levels, readiness and health are unchanged.
- Status-doc validation: Reviewed the specification's first-release scope/open
  questions, existing approval register, verification matrix and completion
  ledger. No operator support claim or accepted behavior is changed.
- Risk & rollback plan: Misreading experimental success as approval is the main
  risk. Keep this package Proposed until an actual operator decision, retain
  original failures, and remove only owned temporary resources. Preserve the
  primary checkout's existing staged changes and conflicts.
- Dependency rationale: No production dependency. Experiments reuse installed
  Ruby/Rust/Perl/native tools and immutable local PostgreSQL/runtime images;
  no new service or package is adopted by the proposed control protocol.
- Stale-policy check: Reviewed AGENTS.md, data/DevOps/Sonar scoped instructions,
  ADR template, 559's G1 boundary and 587. Research permission does not rewrite
  existing approvals or weaken scope. Earlier D5 evidence remains historical;
  the newly reproduced integer-ID failures are an explicit proposed extension.
  Reconciled known retry/lease/discovery contradictions in this proposal and
  exposed the new C1 delivery, S2 handoff/API and audio two-pass deltas. Root,
  scoped, workflow, scanner and historical approval records remain unchanged.
