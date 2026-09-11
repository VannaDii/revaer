# Media Release Verification Matrix

## Purpose And Evidence Boundary

This is an acceptance-work map, not a release certificate or a second status
ledger. Source review is pinned to `f38e6a791f7a28c98fd7dfdc87155271f69bec9a`
(2026-09-11). No media, package, recovery, or operator measurement was run for
this document. Record results, blockers, owners, and owning PRs in the existing
[completion ledger, ADR 564](adr/564-media-completion-ledger.md).

The [first-release scope](../MEDIA_TRANSCODING.md#first-release-scope) and
[approved resolution, ADR 559](adr/559-media-approval-delta.md#approval-resolution)
govern. Grouping below retains every included feature, required fixture case,
and packaged-component entry; it does not authorize a dry-run-only release.
The spec links retained approval holds; this matrix does not resolve them.
Implementation, test presence, a parsed digest, a host conversion, and a green
check are distinct evidence classes, none interchangeable with package proof.

## Recording And Sequencing

Use the following evidence fields within ADR 564, with links to retained reports:

- Matrix row and individual spec requirement/subcase; governing ADR and its
  exact operator-approval evidence, including unresolved decisions. The
  [approval register](media-approval-register.md) is navigation only.
- Source revision and reviewed PR; package/chart identity and digest; Linux
  architecture, host/kernel, supervisor, storage, database, tool/closure and
  fixture identities; effective configuration with secrets omitted.
- Exact `just` command, run date, expected/observed result, report/log location,
  warnings, failed/skipped/unrun cases, preservation comparison and cleanup.
- Evidence class, reviewer, remaining gap, and the exact scope of any claim.

Follow L1-L3 configuration through manual discovery and dry-run, then explicit
execution and recovery, then expand to all cases. Keep the next runnable journey
as the primary work item; independent approved package/evidence work can proceed
without creating competing unfinished journeys. This ordering defers no feature
out of scope. Routine results belong in the task record/ledger; new architecture
or exact-value choices require explicit operator approval recorded in the owning
ADR, with the approval register linking to that evidence.

## Operator Journey

Every row describes required evidence, not an observed pass. Run through the
installed authenticated API/UI and real database; preserve OpenAPI agreement,
problem-detail pointers, authorization checks, and actual SSE observations.

| Row / Ledger | Operator Action And Coverage | Evidence To Retain |
| --- | --- | --- |
| J1 / L1, L10 | Install, initialize, verify, authenticate, and reopen the service. | Clean/persistent installation and wrong/partial-baseline rejection under [541](adr/541-packaged-init-bootstrap-lifecycle.md) and [551](adr/551-packaged-database-baseline-contract.md); least-privilege runtime, real served assets, and package evidence P1-P4. Assembly is not init cutover. |
| J2 / L2-L3 | Create, read, edit, disable/delete profiles and associated targets, policies, compatibility and output settings; select trusted logical roots. | [550](adr/550-packaged-root-catalog-source-contract.md)/[557](adr/557-root-persistence-contract.md) catalog authority, whole-root selection, non-overlapping 1:1 associations, immutable versions, conditional mutations and rebind/re-plan behavior; reject aliases, overlaps, stale generations and remote path authority. |
| J3 / L3 | Configure every [match/filter](../MEDIA_TRANSCODING.md#match-and-discovery), [target](../MEDIA_TRANSCODING.md#target-profiles), [policy](../MEDIA_TRANSCODING.md#policy-profiles), classification, compatibility, discovery, retention, workspace, backup and verification control. | Round-trip each setting through normalized storage, API/OpenAPI and UI; invalid/conflicting or impossible settings fail explicitly. Ordered video/audio/subtitle targets, stream-family retention and every detailed control remain covered by M1-M6, not just simplified forms. |
| J4 / L3 | Validate/import/export versioned Revaer YAML and map local roots. | Portable export omits paths; explicit local-backup export includes them. Unresolved mappings stay disabled, imports force dry-run, invalid/foreign formats fail, and import/export/preview has no media filesystem effects. See [YAML contract](../MEDIA_TRANSCODING.md#revaer-yaml-exchange-format). |
| J5 / L4 | Preview/manual discovery; explicitly enable watchers and minute/hour schedules per association. | Default-off automation, no invented cadence, inherited dry-run, immediate enqueueing when enabled, include/exclude/filter behavior, restart/overflow/offline-storage reconciliation, deduplication and bounded fingerprint admission. Manual discovery cannot accept `replace`; [516](adr/516-durable-discovery-scheduling-and-versioned-aggregate-identity.md)/[535](adr/535-bounded-cancellation-aware-fingerprint-admission.md) holds remain. |
| J6 / L5-L6 | Refresh capabilities at startup/on demand; inspect, normalize, classify, compare and preview a plan. | Immutable source/target/policy/capability inputs, actual/desired graphs, compliance states/scores, selected/rejected stream and plan reasons, cost/risk ranking, DAG dependencies and disk estimates. Repeat identical inputs and invalidate all applicable [cache layers](../MEDIA_TRANSCODING.md#fingerprinting-and-caching) on identity/version changes; prove full native closure under [519](adr/519-immutable-media-capability-execution-identity.md). |
| J7 / L7-L8 | Execute manually, verify, back up when configured, replace, and inspect final disposition. | Dry-run remains visible and default. Only an explicit manual override of a saved dry-run profile requires exact `replace`, without changing that profile; a saved non-dry-run profile still requires authorization. Retain planned operations, independent output/final inspection, original preservation and F1-F5/P3 recovery evidence. |
| J8 / L8-L9 | Diagnose, cancel, retry/re-plan, resume safely, configure history/diagnostic retention, and inspect health/timelines. | Real job phases, plan/rejection reasons, verification, final disposition, SSE, metrics/logs and compact audit agree. Distinguish retry, explicit capability re-plan and successor jobs under [520](adr/520-explicit-media-job-replan-contract.md). Protect active/nonterminal/retryable jobs; completed retention stays off until configured, failed diagnostics follow the existing separate policy. Keep retired worker-write routes retired under [548](adr/548-retired-media-write-route-contract.md). |

## Real Media And Failure Matrix

Expand grouped rows into individual cases in the ledger. Keep every fixture and
acceptance item in the [spec suite](../MEDIA_TRANSCODING.md#media-conversion-integration-fixture-suite)
and [fixture manifest/lock workflow](../test-fixtures/README.md). Record actual
planned operations and independently inspected outputs, including explicit
unsupported outcomes where the spec permits them. Such an outcome does not
certify a required packaged capability. Preparation alone is not conversion.

| Row | Cases Kept In Scope | Expected Safety / Acceptance Evidence |
| --- | --- | --- |
| M1 | All locked/derived fixtures: MP4/MKV/WebM/TS/MOV/AVI; H.264/HEVC/AV1/VP8/VP9/MPEG-4 Part 2/Theora/DivX; AAC/Opus/AC3/MP3/Vorbis/HE-AAC; SRT/WebVTT; fragmented/profile/timecode/header-stripping/audio-gap cases; multi-stream, silent, video-only and audio-only shapes. | Per-fixture identity, probe, metadata, explicit selected outcome and operation; real audio, video and combined transcodes plus remux/copy, not only probeability. Preserve [578 F1](adr/578-locked-fixture-diagnostic-disposition.md#approval-resolution) exact preparation-only bounds and raw evidence; drift needs renewed approval. |
| M2 | No-op, copy/remux, stream order, metadata/disposition/label rewrites, container/title/faststart settings, languages/roles, chapters, fonts/attachments/data, provider/edition/version/quality and configured origin metadata. | No unnecessary rewrite; every change is planned and independently compared with the desired graph. Retained content and metadata survive; unsupported rewrites fail explicitly, never silently discard. |
| M3 | Multi-video preserve/remove/reorder/copy/remux/transform, including single-video reduction; codec/encoder, resolution/FPS/bitrate, pixel format/bit depth, HDR10/Dolby Vision/HDR-to-SDR, color/range, deinterlace/crop/scale and quality controls. | Verify each selected stream and configured transform, preservation and compatibility constraints, not just container tags or successful exit. Exercise capability fallback and configured hardware/CPU and intent exceptions. |
| M4 | Ordered audio and fanout; preferred/fallback codecs, channels/layout/minimums, bitrate interpretation, passthrough, language/commentary/descriptive roles, LFE, loudness/DRC, downmix, stereo compatibility and surround preservation. | Verify output codec/order/roles/layout, actual analysis and applicable quality criteria. Silent/undeclared tracks and missing required audio have explicit outcomes. [515](adr/515-versioned-audio-transformation-and-acceptance-contract.md) preset/filter-order/bitrate holds remain; test literals are not production values. |
| M5 | Existing subtitle embed/extract/copy, supported conversion without OCR; embedded/sidecar/both/none placement, patterns/precedence, SRT/ASS/VTT/SUP/SUB/IDX, language/role/requiredness/default/forced flags and styling. | Inspect retained/extracted content, sidecar identity and final placement. Image-based subtitles preserve/remove/fail per policy; required impossible placement fails. No acquisition, generation or OCR; intended final sidecars are distinct from forbidden source-adjacent transient files. |
| M6 | Every unmatched video/audio/subtitle/attachment/data preserve/remove/fail rule; general/anime/audiobook/archival intent; compatibility targets and all quality/runtime safeguards. | Exercise target-policy separation, optional/missing streams, conflict refusal, configured quality/loudness/size guards, upscale/upmix/lossy-reencode protection, reconversion/loop prevention and explicit force. Intents preserve configured grain, styling, chapters and metadata without silently mutating target semantics. |
| F1 | Inspection, import/export, discovery preview, plan and dry-run; unauthorized/incorrect manual override. | Compare source aggregate bytes/metadata and adjacent-directory inventory before/after; no source mutation, backups, output sidecars or unmanaged artifacts. Only authorized plan/audit state changes occur. |
| F2 | Inspection/normalization/planning/capability failures; unavailable or changed tools; disk/reserve refusal; execution timeout/cancel/output overflow; invalid/corrupt/mismatched output and failed backup. | Explicit bounded failure state, audit and events; no unverified replacement. Reinspect desired structure, duration, metadata, mux and configured decode/playback/seek checks under [verification policy](../MEDIA_TRANSCODING.md#verification). Prove process/descendant cleanup under [554](adr/554-preemptible-native-process-broker-contract.md)/[558](adr/558-rvb1-native-process-broker-wire-contract.md), not a mock-only exit. |
| F3 | Source/member changes; symlink, root/mount replacement, alias and concurrent-writer races; changed configuration or capability identity. | Fail closed before unauthorized read/write/replacement; retain original or verified recovery material. Prove generation/descriptor binding and root barriers under [557](adr/557-root-persistence-contract.md) and immutable execution identity under 519. |
| F4 | Crash/restart at DAG checkpoint, backup publication, replacement, final verification, database/outbox acknowledgement and rollback; competing stale owners; shutdown during each phase. | Resume only verified intermediates under current fenced authority; classify uncertain state before new destructive work. Prove complete aggregate preservation/recovery and exactly attributable outcomes under [512](adr/512-fenced-resumable-worker-ownership-and-recovery.md) and [525](adr/525-attempt-scoped-backup-and-rollback-layout.md). No inferred shutdown bound or S2 approval. |
| F5 | Cleanup failure and restart; retention during active, retryable, terminal and unresolved recovery states; backup/quarantine capacity and expiry. | Filesystem cleanup precedes evidence pruning; preserve rollback material and compact destructive audits, never delete another attempt's tree. Exercise bounded diagnostics, stale janitor and protected bundles under [513](adr/513-attempt-scoped-workspace-retention-transaction.md)/525; unreconciled failure stays visible. |

## Exact Package And Maintenance Evidence

Repeat P1-P4 separately for Linux `amd64` and `arm64`, on clean deployments of
the exact release candidate. Record native versus emulated execution explicitly;
a historical arm64 probe or host test cannot stand in for either package run.

| Row | Rehearsal | Evidence Boundary |
| --- | --- | --- |
| P1 | Resolve/install the exact image and chart; inspect final runtime, capabilities, About API/UI and release metadata. | Source/build manifest, architecture image digest, chart hash/version, full tool/dependency-closure inventory, licenses/notices/attribution, corresponding-source/source-offer, patent-review, SBOM, scans and required signatures/attestations all bind to that artifact. Account for every [required packaged component](../MEDIA_TRANSCODING.md#docker-runtime-image); prove no forbidden build/source material, nonfree FFmpeg, direct GPL linkage or unapproved interpreted utility. ExifTool needs its exact bounded-adapter/security/compliance evidence. |
| P2 | Fresh install and repeated baseline verification; empty/persistent volumes; wrong database/package identity, permission failures and missing/corrupt package evidence. | Pin [build inputs](../.github/build-inputs.env), database baseline/init digest, runtime identity, volume ownership/durability and readiness/admission observations. Prove accepted init lifecycle and least privilege; do not activate inert SQL. C1's manifest startup outcome remains undecided, so characterize failures without selecting its proposal. |
| P3 | Exercise media backup and complete aggregate restore; separately back up and restore the service database and required persistent state into an isolated deployment. | For media, verify 525 ownership/manifest/member hashes and authorized metadata, interrupted restore and protected rollback source. For service restore, record package/init identity, baseline verification, profiles/bindings/jobs/audits and persistent-media continuity before/after. Reconcile changed root identity explicitly. [551](adr/551-packaged-database-baseline-contract.md) permits verified full-database restore or disposable same-digest recreation, not an in-place downgrade. A created backup is not restore proof. |
| P4 | Rehearse documented maintenance, stop/restart, loss of a required mount/tool, repair, and rollback using retained evidence. | Record the exact runbook/actions, stop/admission boundaries, recovery classification, orphan-process/workspace checks, operator intervention, elapsed recovery and any lost/unreconciled state. Re-run a representative journey and preservation checks afterward. Image/tool/root changes obey 519/557 re-plan/rebind rules; this is no permission for a new upgrade mechanism, maintenance cadence or deadline. |

## Operating Support Envelope

These are approved contract boundaries and evidence dimensions, not assertions
that this revision implements or has validated them. Publish exercised tuples
only; record untested combinations and held included capabilities in ADR 564.

| Dimension | Contract / Boundary | Evidence Needed For A Support Claim |
| --- | --- | --- |
| Platform and containment | [554](adr/554-preemptible-native-process-broker-contract.md#process-and-containment-model), amended by [559 B1-B3](adr/559-media-approval-delta.md#decision-delta): Linux amd64/arm64 in validated container or equivalent service control-group containment. Docker Desktop, macOS, Windows, uncontained shells and unsupervised Docker are not production media environments. | Exact kernel/runtime/supervisor/security identity and settings; startup/recovery, restart, both broker lanes and session-escaping descendant termination on each architecture. A successful Docker build is insufficient. |
| Filesystems and roots | [523](adr/523-managed-root-catalog-and-job-binding-contract.md), 514 and 557 require attested managed roots, explicit durable workspace and writer authority. Network filesystems are outside 554; default `emptyDir`/unattested writable storage is evaluation/dry-run only. | Actual filesystem/mount/device, permissions, durability, rename/fsync/capacity/reflink/hardlink behavior where used, sole-writer evidence and root recovery. [525](adr/525-attempt-scoped-backup-and-rollback-layout.md) does not support required ACL/xattr backup restoration. |
| Database and service state | [551](adr/551-packaged-database-baseline-contract.md) exact database identity, settings, roles and baseline; coordinated init cutover, not candidate equivalence alone. | Exact deployed inputs and cold/warm/existing-data results, retained failed counterexamples, pristine/partial/wrong baseline tests and complete restore/recovery. Conditional D3 is not certification; D4/D5 remain held. |
| Media and native environment | [519](adr/519-immutable-media-capability-execution-identity.md), 558 and the spec require immutable execution closure and capability-bound configured compatibility. E1 exact environment/HOME values are held. | Tool/library/driver/device identity, actual codec/container/profile/filter/subtitle/utility cases, unsupported outcomes and CPU/hardware fallback. No general Plex/client playback certification or support inferred from an encoder listing. |
| Workload and resources | [policy controls](../MEDIA_TRANSCODING.md#policy-profiles), 512-516/535; retain existing approved limits without inventing replacements. | Tested input sizes/durations/stream counts, association/library shapes, concurrency, queue/IO/CPU/GPU/memory/disk conditions, reserve behavior, maintenance/pause/active-stream rules and recovery load. Document observed ranges, refusals and limitations; no capacity, latency, quality or recovery promise from code constants. |

## Measurement Candidates

The [spec observability contract](../MEDIA_TRANSCODING.md#observability) remains
authoritative. Candidates below add no metrics policy, labels, collection
cadence, telemetry implementation, alert rule, numerical threshold or SLO.
For each observation retain workload, units, start/end definition, sample count,
excluded/unrun cases and report identity; distinguish measurement from estimate.

| Question | Candidate Evidence |
| --- | --- |
| Can the operator complete and explain the journey? | Observed install-to-first-plan and first-verified-replacement time; actions, remediation attempts and unassisted completion for a defined scenario; ability to locate the selected/rejected reasons, dry-run state, verification and original/recovery evidence. Record failures as well as completions. |
| Does execution deliver useful results without avoidable work? | Discovery candidates/exclusions, inspection failures, compliance states, selected operations, codec/encoder usage, fallback/reconversion counts, estimated versus actual operation cost and disk amplification; source/output byte delta only alongside preservation/quality results. |
| Can the operator detect and recover failures? | Phase/queue/recovery elapsed time, reserve refusals, capability/verification/replacement/cleanup failures, intervention count, restore success/failure and unreconciled state. Correlate job/profile/root-association/attempt/phase through existing bounded events, logs and SSE. |
| Does maintenance preserve operability and evidence? | Before/after health/admission state, process/workspace cleanup, completed/failed diagnostic pruning by configured age/count, retained compact audit and rollback material; time and actions to reproduce a diagnosis and resume the journey. |

Review measurements before proposing held values; never backfill them from
constants. Preserve origin-only error logging and existing secret/path/metadata
redaction; correlation identifiers are not permission for unbounded metric labels.

## Holds And Completion

This matrix does not approve [C1/586](adr/586-compliance-manifest-failure-boundary.md),
[D4/579](adr/579-ingestion-policy-temporary-table-lifetime.md),
[D5/583](adr/583-ingestion-imdb-conflict-inference.md),
[E1/558](adr/558-rvb1-native-process-broker-wire-contract.md),
[S2/577](adr/577-runtime-shutdown-event-classification.md),
[binary exception 585](adr/585-fixed-binary-deletion-review.md), or the numeric
and semantic holds in [559's remaining-holds table](adr/559-media-approval-delta.md#remaining-holds)
(512-516/535). Existing exact approvals, conditional D3 and narrow F1 retain
their original scope and expiry. Explicit operator evidence in each owning ADR
is authoritative for decisions; the approval register is navigation only.
No new dependency, runtime behavior or quality criterion is set.

Existing [API/UI tests](../tests/README.md), [media test source](../crates/revaer-media-runtime/tests/media_fixtures.rs)
and [Just recipes](../justfile) are starting points, not executed evidence here.
Completion still requires exact-revision integrated `just ci`, `just ui-e2e`,
dedicated media and package validation, strict Sonar, applicable remote
checks/reviews and recorded ledger evidence. Documentation checks alone do not
close any release row.
