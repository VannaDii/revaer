# Media completion evidence ledger

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context: The full first-release goal is active; implementation, accepted
  contracts, local checks, and remote PRs must not be conflated.
- Decision: Record a bounded outside-in work breakdown with evidence and
  dependencies. This document selects no architecture, values, or exceptions.
- Consequences: No row below establishes release readiness. Missing evidence
  remains work, not an exclusion from the release.
- Follow-up: The integrating agent reviews this record, attaches exact-revision
  validation, and assigns reviewed PRs after stack reconciliation.

## Evidence Boundary

### Current Delivery Entry Point (2026-09-11)

The operator approved the evidence-led delivery recommendations in
[ADR 587](587-evidence-led-delivery-governance.md). This remains the single
capability-to-evidence ledger; earlier checkpoints below are dated history,
not current completion claims. All L1-L10 capabilities remain open.

The operator subsequently approved ADR 588 choices 1-6 and separately scoped
transfers in 7 at reviewed commit `9575c077`; see the
[approval resolution](588-first-release-decision-package.md#approval-resolution).
Those named design holds are resolved. The qualification conditions are not:
older held/pending descriptions below remain historical checkpoints, not new
approval requests or evidence of implementation.

- [Approval register](../media-approval-register.md): decision scope and retained holds.
- [Release verification matrix](../media-release-verification.md): journey,
  preservation, package, maintenance, and operating-envelope evidence required.
- [Execution records](../tasks/index.md): bounded ongoing work without new ADRs
  for every implementation iteration.
- [Root catalog mode portability](../tasks/2026-09-11-root-catalog-mode-portability.md):
  bounded follow-through on Linux CI diagnostics; 59 focused tests pass on each
  of Linux/macOS arm64. Source `c8d83bf1` completes `just ci` with eight retained
  watcher WARN lines, while full E2E still fails profile creation (201 expected,
  400 received): 46 passed, one failed, 61 not run, plus teardown coverage error.
  This is not root binding, cutover, published Sonar or package acceptance.
- [Root readiness response contract](../tasks/2026-09-11-root-readiness-contract.md):
  source `ea4fa87a` passes 61 focused tests on Linux/macOS; independent review
  caught and verified fixes for noncanonical Serde representations. Full CI
  exits 0 with eight retained watcher warnings; E2E remains 46 passed, one
  failed, 61 not run with the profile-creation and teardown failures. No route,
  database, filesystem, published Sonar or package readiness follows from this
  transport model.
- Retained integration before this deliverable: `bb302c03`.
  [ADR 581](581-stack-boundary-reconciliation.md) records newer local boundaries
  and the 2026-09-11 remote audit. Older GitHub diff counts below used different
  comparison surfaces and must not replace canonical per-base/head checks.

### Active Delivery Order

| Priority / Scope | Next Concrete Result | Exit Evidence / Current Constraint |
| --- | --- | --- |
| First: L1-L3, then L4 manual discovery and L6 dry-run | Complete approved init/root/profile integration through the actual authenticated API/UI. | Pristine init, stored-procedure binding, profile creation, portable configuration and a real dry-run with unchanged originals. Implement approved D4/D5 and complete D3 evidence. Do not soften the reproduced profile-creation failure. |
| Independent prerequisite: L5/L10 | Establish exact native/package identity and approved broker evidence. | Linux amd64 and arm64 image digests, native closure, containment and environment evidence. Implement approved C1/C1-D/E1; qualify both native packages before activation. |
| Next: L7-L9 | Run one configured transformation through verification, replacement and crash recovery, then close the full included operation matrix. | Real output inspection plus filesystem/database/audit assertions. Implement and qualify the approved ownership, discovery, audio and shutdown contracts before activation. |
| Merge closure: L10 | Publish verified deliverables in dependency order, retiring redundant unpublished candidates only after preservation is proved. | Exact diff/ancestry/size, full local gates, applicable remote checks, positive published Sonar coverage, actual review state, and resolved feedback. No current release or merge acceptance is established. |

For each L row, attach requirement-level evidence progressively using the
following fields; do not mark a whole row complete from a representative test:

| Requirement / Spec Anchor | Owner / PR | Implemented At | Locally Verified At | Package Verified At | Merged At | Remaining Limitation |
| --- | --- | --- | --- | --- | --- | --- |
| L2/L9: ADR 557 path-free readiness JSON representation | Local, unpublished | `ea4fa87a`, shared API models | 61 focused tests on Linux/macOS; [full CI/UI evidence](../tasks/2026-09-11-root-readiness-contract.md#final-executable-evidence) | Not run | Unmerged | No route, consistent SQL read, filesystem proof or usable root workflow; full handoff gates remain blocked |
| L1: Approved D4/D5 ingestion corrections | Local, unpublished | Corrections `9786ef13`, replayed into `b5c860eb`; proof through `f5981931` | [Committed sampling increment](569-init-privilege-and-timeout-resolution.md#committed-sampling-evidence-2026-09-13): 200 focused assertions, 12 live variants and canonical proof 6,062/6,063 checks; only incomplete D3 fails. All 38 sampling source hashes match `fe0948a4` | Not run | Unmerged | Complete branch, mutating-helper/warm and native/trigger qualification remains. Frozen authority unchanged; UI and canonical Sonar acceptance remain absent |
| L5/L10: C1 early compliance-metadata failure | Local, unpublished | `81947a65` | [701 focused tests, four lint passes, real-file failures and injected-success serving](586-compliance-manifest-failure-boundary.md#c1-implementation-checkpoint-2026-09-11); parent reviewed and integrated locally | Not run | Unmerged | Combined CI/UI, canonical Sonar, C1-D delivery and conditional E1 qualification remain; parsed metadata is not authenticity proof |
| L2/L5/L10: Approved explicit C1 host-E2E bootstrap | Local, unpublished | `bee07560`, replayed as `1c470560` | [Parent E2E result](586-compliance-manifest-failure-boundary.md#parent-integration-result-2026-09-11): 17 enforced bootstrap guards, eight focused Rust tests; full UI reaches 57 passed, one failed, 72 not run | Not run | Unmerged | Schedule enablement lacks verified filesystem identity; required 200 assertion and missing-UI-coverage failure remain. Test-only compliance injection is not package evidence |
| L5/L10: C1-D chart binding | Local, unpublished | `fd1dc9b4` and `cad195b6`, replayed as `e940e873` and `e48364da`; parent ownership/coverage integration | [Parent chart checkpoint](../tasks/2026-09-11-compliance-chart-binding.md#parent-integration-checkpoint-2026-09-11): 100 chart cases, seven package cases, strict lint, real unsigned archive, clean changed-file secrets scan and 301 drift assertions | Not run | Unmerged | Installer/authenticity, actual PVC/startup, both native packages, clean final CI/UI and canonical Sonar remain; renderer fixtures are not release evidence |
| L5/L10: Completed engine-profile publication | Local, unpublished | Worker `396c87fc`, integrated as `749ba0b4` | [Publication boundary](586-compliance-manifest-failure-boundary.md#completed-engine-profile-publication-boundary-2026-09-12): 17 app tests and 22 libtorrent tests in both feature configurations; strict scoped lint; actual worker acknowledgements precede profile publication | Not run | Unmerged | Existing serial refresh callers only; revision-wide atomicity, native rollback/settlement, shutdown admission, owner/PID1 and watcher warnings remain |
| L10: Inclusive coverage, native pipe closure and owned watcher shutdown | Local, unpublished | Full gates on clean `fe0948a4` | [Current checkpoint](586-compliance-manifest-failure-boundary.md#sampling-and-owned-watcher-integration-2026-09-13): CI passes with no WARN/compiler-warning lines, all 18 package coverage gates pass; prior eight watcher warnings absent. Full UI: 57 passed, one failed, 72 not run; Rust/script and partial JS coverage retained | Not run | Unmerged | Broader S2/native containment, scheduling/root binding and UI coverage; unavailable canonical Sonar token and positive published coverage; native package/conversion qualification. No complete handoff or remote pass |
| L1: Native FK counter limitation | Local, unpublished | Worker `bbf50836`, integrated as `0ef147c0` | 208 unit assertions and 306 bounded live checks; real FK failures retain NULL counters despite working tracked C-function controls; combined CI passes at `fe0948a4` | Not run | Unmerged | This mechanism cannot prove native callback counts. D3 stays incomplete; preserve the negative result without substituting catalog reachability for execution |
| L1: Magnet normalization branches | Local, unpublished | `e3fb95c6` | [Bounded wrapper evidence](569-init-privilege-and-timeout-resolution.md#magnet-normalization-branch-evidence-2026-09-13): 738 focused live checks; canonical 6,575/6,576, failing only incomplete D3; full CI passes without warnings, UI remains 57 passed/one failed/72 not run | Not run | Unmerged | Concurrent GUID conflicts, remaining helper/native scope, scheduling/root identity and published Sonar coverage remain; no D3 or cutover acceptance |
| L1: Completed native RI insert observation | Local experiment only | Worker at `36abef44`; no production adoption | [Bounded mechanism evidence](569-init-privilege-and-timeout-resolution.md#bounded-native-callback-observation-2026-09-13): eight variants pass; 64 completed callback records independently matched to raw logs | Not run | Unmerged | First-visible path only; update/delete/error/skip paths and full closure remain. No canonical profiler or gate change |
| L1: Concurrent GUID conflicts | Local proof, unpublished | `7e9ab9cf` | [Controlled conflict evidence](569-init-privilege-and-timeout-resolution.md#concurrent-guid-conflict-proof-2026-09-13): eight variants/four comparisons, 18 write/19 read tables and 356 unit assertions; canonical proof 6,588/6,589, failing only incomplete D3. Full CI passes without warnings | Not run | Unmerged | In-call settings and native/helper closure remain. UI is 57 passed/one failed/72 not run; Sonar input gate rejects absent JS LCOV, despite successful native-input generation. No scan, push or cutover |
| L1: GUID logger context and warm reuse | Local proof, unpublished | `ce971164` | [In-call and logger-first proof](569-init-privilege-and-timeout-resolution.md#guid-in-call-and-logger-first-evidence-2026-09-13): 24 live variants, 267 focused checks and 1,048 unit assertions; canonical 6,618/6,619, failing only incomplete D3. Full CI rerun passes with all 18 package coverage gates | Not run | Unmerged | Initial CI process-group EPERM is unresolved despite 20 isolated passes and the full rerun. UI remains 57 passed/one failed/72 not run; other helper/native scope, JS coverage, Sonar publication and PR acceptance remain |
| L1: Native sample-retention observation | Local experiment only | Clean `ef3b17ef`; no production adoption | [Bounded trim evidence](569-init-privilege-and-timeout-resolution.md#guid-in-call-and-logger-first-evidence-2026-09-13): eight variants/200 fixtures, 600 checks in each plain/profiled run and 160 offline checks; actual UPDATE/DELETE plans retained | Not run | Unmerged | Observed callback counts are not independent oracles; unobserved native/error/skip paths remain. No canonical profiler or D3 completion |
| L2/L4/L9: Profile fixture and manual-admission API regressions | Local, unpublished | `6fbf4ab0`, `f63c1849`; replayed onto `4a67468f` | [Bounded API proof](../tasks/2026-09-11-media-profile-create-rejection.md#continued-profile-and-scenario-proof): 13 passed, one failure at actual schedule enablement; isolated watcher-only refinement passes | Not run | Unmerged | Pre-C1 injected host proves HTTP/persistence and manual job admission only. Trusted-root binding and enabled automation remain unimplemented through this legacy API; positive assertions and full CI/UI/route gates remain required |
| L10: Independent coverage retention and deterministic setup-deadline fixture | Local, unpublished | `d08598ea`, exact guardrail at `9b988225` | [Integrated refinement](586-compliance-manifest-failure-boundary.md#coverage-retention-and-deadline-fixture-2026-09-13): full CI passes without warnings, all 18 package coverage gates pass; 494 real independent entries survive regeneration; local Sonar input verification passes | Not run | Unmerged | Full UI still fails: 57 passed, one failed, 72 not run, plus absent UI coverage. Positive local Rust/JS inputs are not published Sonar coverage; no production errno suppression, D3/cutover or remote acceptance |
| Pending expansion for every included requirement | Unassigned until exact stack mapping | Source commit plus files, or not implemented | Source commit, dirty delta, Just command, full result and evidence path, or not run | Same source, platform, immutable image digest and report, or not run | Merge commit and remote gate/review readback, or unmerged | Explicit missing proof or decision |

The field row is a required format, not fabricated evidence. The detailed
acceptance mapping remains work; no percentage-complete claim follows from
this documentation. The first operator slice does not exclude later features.

### Historical Source Inspection

Source inspection is pinned to local integration
`78cec0c0d8b7759847a29cb11402924a24bf432c`, the base of this documentation branch.
Links below refer to that tree unless explicitly marked as a parent checkpoint.
Names, modeled operations, test presence, and accepted ADRs are not execution
proof. This is a bounded completion ledger, not an exhaustive code audit or the
final per-requirement acceptance report required by ADR 559.

The [specification](../../MEDIA_TRANSCODING.md#first-release-scope) remains the
scope authority, as amended by approved ADRs. Its historical "no unresolved
operator decisions" sentence does not cancel later explicit holds. The
[accepted resolution](559-media-approval-delta.md#approval-resolution) approves
R1-R4, B1-B3, and narrow G1 genuinely; E1 and the enumerated remaining holds stay
open. No older proposal, replayed constant, or green check supplies consent.

### Local Is Not Remote

Read-only GitHub CLI queries on 2026-09-10 establish:

- [PR 98](https://github.com/VannaDii/revaer/pull/98),
  `fix(media): bound verifier diagnostics`, has base `54157ef8` and head
  `7ed723d5`: 110,059 additions plus 20,998 deletions, **131,057 changed lines**.
  It needs stack/diff repair before review or merge; its title cannot establish
  ownership of everything in that diff.
- [PR 194](https://github.com/VannaDii/revaer/pull/194),
  `fix(runtime): classify requested shutdown cancellation`, has head
  `cfed91f82a07a909ac1b96d9129e9eabeaddbf1c`, not this integration revision.
  Local `git rev-list --left-right --count 78cec0c0...cfed91f8` reports
  116 local-only and 107 remote-only commits. Preserve and reconcile both sides;
  replacing the remote tip with the local integration is not a proven restack.

No feature row is assigned a remote owning PR: this task did not prove an exact
reviewed diff carrying each complete deliverable. PR 98 is directly identified
only as a delivery blocker, and PR 194 only as a divergent remote checkpoint.
No GitHub check, review, stack metadata, or package result is inferred here.

### Verified First Repair Boundary

The parent checked the existing clean chain
`54157ef8 -> 4207a08a -> f87bea0b`. Its total diff is 519 changed lines: a
364-line documentation prerequisite followed by the 155-line verifier change.
The context-free stable patch ID of `f87bea0b` matches remote `7ed723d5` exactly:
`09366ee35d93a058cace1358c3ddf85c1da3d753`. Verifier source, both regression
tests, and ADR 341 are byte-identical; this candidate does not omit the verifier
deliverable. The documentation prerequisite is separate from that equivalence
proof and does not independently authenticate historical approval quotations.

PR 99 currently advances `7ed723d5` to `8c985b4b`; local `ccf8b3ef` advances
`f87bea0b` with the corresponding target-stream change. Its range-diff has
base-dependent import, prior API-boundary, and documentation-index differences,
so it is not a claim of identical full patch IDs. Reconcile and validate this
transition and the remaining descendants before publishing the repaired
boundary. No branch was pushed, no formal-stack metadata was changed, and no
exact-restacked-revision CI/UI result is established by these comparisons.

## Outside-In Ledger

Every row remains open. "Source" means inspected implementation, not a passing
test. "Required" names acceptance work, not a new architectural decision. Owning
PR for each row is **unassigned pending verified stack mapping**.

| ID / Operator Capability | Current Source Evidence | Remaining Work And Required Verification |
| --- | --- | --- |
| L1: Install and initialize | [Rebaseline configuration](../../config/database-rebaseline.env) is `assembly`, with 167 frozen migrations and a 1,624-statement candidate. [Data bootstrap](../../crates/revaer-data/src/config.rs) still invokes `sqlx::migrate!`; the assembled [init script](../../crates/revaer-data/init.sql) is inert. | Finish accepted [522](522-pre-v1-single-init-script-transition.md), [541](541-packaged-init-bootstrap-lifecycle.md), and [551](551-packaged-database-baseline-contract.md) finalization, grants, baseline seal/verification, packaged init/verify modes, and coordinated authority cutover. Prove pristine install, repeat verification, partial/wrong baseline rejection, least-privilege runtime, and migration retirement. Do not add a migration or claim assembly is cutover. |
| L2: Bind trusted roots and configure profiles | [Root source](../../crates/revaer-media-runtime/src/root_catalog/mod.rs) parses/loads a trusted deployment catalog; its contract explicitly grants no root attestation or write authority. Searches found no catalog caller in app/data/API. [Profile UI](../../crates/revaer-ui/src/features/media/view.rs) still holds raw source/output strings. | Implement accepted [550](550-packaged-root-catalog-source-contract.md) attestation/writer evidence and [557](557-root-persistence-contract.md) normalized persistence, bootstrap reconciliation, logical binding, API/UI, and immutable five-root job snapshot in the coordinated init flow. Prove empty whole-root prefix, overlap races, stale generations/rebinding, authorization, conditional updates, and rejected remote path authority. Parser tests alone cannot close this row. |
| L3: Configure targets/policies and exchange portable YAML | [Media facade](../../crates/revaer-app/src/media.rs) implements profile/catalog operations and YAML validation/apply/export; [target compilation](../../crates/revaer-media-core/src/target.rs) resolves ordered streams. | Complete the spec's video/audio/subtitle, compatibility, matching, retention, runtime/output, and verification controls across normalized storage, API/OpenAPI, UI, and YAML. Reconcile with L2 and immutable version contracts. Exercise round-trip portability, unresolved mappings kept disabled, invalid/conflicting policy rejection, imports forced to dry-run, and no import/preview filesystem effects. Existing simplified forms are not full configuration coverage. |
| L4: Discover manually and enable automation explicitly | [Discovery service](../../crates/revaer-app/src/media_discovery_runtime.rs) has native watcher events, scan cursors, and an in-memory `last_scheduled` map. [Frozen procedure](../../crates/revaer-data/migrations/0182_media_normalized_configuration_identity.sql) rejects legacy automation writes with `media_profile_filesystem_identity_required`. | Finish L2 and implement the durable discovery/aggregate and bounded fingerprint contracts under [516](516-durable-discovery-scheduling-and-versioned-aggregate-identity.md)/[535](535-bounded-cancellation-aware-fingerprint-admission.md), using ADR 588's approved DISC values and semantics. Qualification remains required before activation. Prove default-off behavior, manual plan/audit-only discovery, enabled watcher/schedule enqueueing, restart/overflow/missed-event recovery, deduplication, and cancellation/quota accounting. Enabled automation is included, not expendable to make UI tests pass. |
| L5: Inspect media and refresh trustworthy capabilities | [Capability probes](../../crates/revaer-media-runtime/src/capabilities/detect.rs) use an injected supervisor; [inspection adapter](../../crates/revaer-media-runtime/src/inspect/adapter.rs) composes probing and inspection. [Bootstrap](../../crates/revaer-app/src/bootstrap.rs) constructs the current system supervisor. | Complete [519](519-immutable-media-capability-execution-identity.md) full native dependency closure and immutable job binding, plus [554](554-preemptible-native-process-broker-contract.md)/[558](558-rvb1-native-process-broker-wire-contract.md) broker routing and containment. No RVB1 app/data/API implementation was found. Prove stale/mutated tool rejection, both-lane readiness, bounded probing, secret-free environment, and all advertised tool/filesystem capabilities on both Linux architectures. ADR 588 conditionally approved E1's environment; native qualification still gates activation. |
| L6: Inspect explanations, compare state, preview a safe plan | [Core pipeline](../../crates/revaer-media-core/src/pipeline.rs) compiles targets, diffs graphs, prunes candidates, and returns selected/rejected explanations. [Job runtime](../../crates/revaer-app/src/media_job_runtime.rs) persists planning evidence and has an explicit dry-run completion branch. | Verify immutable source/target/policy/capability inputs, deterministic selection, compliance/scoring, cost/disk estimates, DAG dependencies, and cache invalidation across the complete configured graph. Prove every declared operation is executable and verifiable, rejected plans remain explainable, and dry-run produces no source-adjacent artifacts. Core enum coverage is not operator workflow coverage. |
| L7: Execute the requested media transformations | [Execution builders](../../crates/revaer-media-runtime/src/execute/mod.rs) construct desired-graph, subtitle and transcode arguments. They explicitly reject arbitrary `MetadataRewrite` without a verified desired-metadata contract. The same file still has direct `Command::new` execution; audio analysis and verification have separate direct runners. | Finish broker/closure routing, then validate copy/remux, labels/dispositions/order, subtitle copy/embed/extract/conversion without OCR, audio/video transforms, multi-video, metadata, HDR/color, chapters, attachments, unmatched-family policies, and configured quality safeguards. Implement ADR 588's approved AUDIO-1 resolution of [515](515-versioned-audio-transformation-and-acceptance-contract.md), retaining its measurement and listening acceptance. Real fixtures must assert planned operations and independently inspected output; existing literals or hand-built fixture capabilities are not approved production behavior. |
| L8: Verify output, replace safely, recover and retain evidence | [Job orchestration](../../crates/revaer-app/src/media_job_runtime.rs) performs candidate/final verification, replacement commit, rollback/finalization, and startup recovery. [Replacement](../../crates/revaer-media-runtime/src/replacement.rs), [verification](../../crates/revaer-media-runtime/src/verification/mod.rs), and [workspace retention](../../crates/revaer-app/src/media_workspace_retention.rs) contain implementations, not a packaged safety proof. | Complete [512](512-fenced-resumable-worker-ownership-and-recovery.md)/[513](513-attempt-scoped-workspace-retention-transaction.md)/[514](514-packaged-media-subsystem-lifecycle-and-health-contract.md) ownership, resumability, per-root barriers, and cleanup transactions with held values resolved. Prove source/root races, disk refusal, verification failure, backup/quarantine, concurrent stale owners, crashes at filesystem/database/outbox boundaries, rollback, cancellation/shutdown, retention and compact audits. The replay's one-hour stale threshold is not approved resumable recovery. |
| L9: Operate through API/UI and diagnose failures | [Handlers](../../crates/revaer-api/src/http/handlers/media.rs), [UI](../../crates/revaer-ui/src/features/media/view.rs), and [API](../../tests/specs/api/media.spec.ts)/[UI tests](../../tests/specs/ui/media.spec.ts) expose configuration, capabilities, previews and job diagnostics. The UI test includes a surface-rendering check, not full service acceptance. | Deliver L2-L8 through the real authenticated workflow, including explicit manual `replace` override without changing saved dry-run, cancel/retry/re-plan, timelines, SSE, health and bounded metrics/logs. Reconcile OpenAPI/client/teardown routes and retention controls. Run the full UI suite against the same installed service and database, not route mocks or softened assertions. Parent checkpoint below is currently failing. |
| L10: Ship supported packages and merge reviewable deliverables | [Dockerfile](../../Dockerfile), [media recipes](../../just/media.just), [PR workflow](../../.github/workflows/pr.yml), and [Sonar criteria](../../sonar-project.properties) provide build/validation surfaces. [Fixture source](../../crates/revaer-media-runtime/tests/media_fixtures.rs) includes real generated audio fanout with a hand-built capability snapshot; this does not exercise the full packaged operator lifecycle. | Validate clean Linux amd64/arm64 installs, complete spec fixture matrix, tool inventories/redistribution evidence, ExifTool boundary, immutable closure, containment and E1 values. Retain per-image digests/reports. Reconcile one linear stack and recheck every exact diff against the canonical 9,999-line limit; require all applicable checks, positive strict Sonar coverage/evidence, Vanna assignment, requested Copilot review and addressed actionable feedback. No package or remote pass is established here. |

## Critical Path And Decision Queue

1. **First runnable operator slice: L1-L3, then L4 manual discovery/L6 dry-run.**
   Finish the approved normalized root/profile workflow and coordinated init
   lifecycle. Use the reproduced API/UI failure as an acceptance boundary, not
   a reason to disable an included control. Keep root generation, whole-root
   prefix, overlap locking and the approved 128-association bound unchanged.
2. **In parallel: L5/L10 package and native identity evidence.** Implement the
   approved broker/closure and ADR 588 C1/C1-D/environment contracts. Conditional
   E1 requires actual amd64/arm64 qualification before activation, not another
   approval of the same values. C1 permits no degraded startup when required
   compliance metadata fails; B1 does not authorize unsafe continuation.
3. **Implement and qualify the approved discovery, lifecycle and audio choices.**
   ADR 588 resolved the named DISC-1 through DISC-7, S2/LIFE-1 and AUDIO-1 design
   holds. Preserve their exact limits, ownership and recovery contracts; supply
   native timing, durable-state and listening evidence before activation.
   Historical pending wording below is not a renewed decision request. Any
   additional substantive deviation still requires its own operator approval.
4. **Complete explicit execution and failure recovery: L7-L9.** Once prerequisites
   are proved, exercise one real configured job through verification and safe
   replacement/recovery, then expand to the complete included operation matrix.
   A successful remux or dry-run alone does not complete the release.
5. **Close delivery evidence: L10.** Parent reconciles the stack and maps exact
   reviewed diffs to these deliverables. Rerun applicable gates after code or
   base changes; merge in dependency order only with actual checks/review proof.

These are execution priorities, not new architecture or permission to omit
anything in the specification. Other accepted exact contracts remain binding;
newly discovered decision gaps return to operator review, not invented defaults.

## Parent Validation Checkpoint

The integrating agent supplied these results in the task conversation on
2026-09-10 for local checkpoint `b4645d7b`; they were **not rerun in this ledger
worktree** and do not change the source-inspection baseline above:

- `just ui-e2e`: 46 passed, one failed, 61 not run. Profile creation at
  `tests/specs/api/media.spec.ts:130` expected 201 and received 400. Parent source
  inspection ties the line-96 fixture's raw roots plus `schedule_enabled: true`
  to the legacy `media_profile_upsert_v1` identity gate. API logs did not expose
  the error code; this is not a claim about an observed response body.
- Teardown also reports missing executed coverage for the job `phases` and
  profile `readiness` GET routes. This does not establish that the routes are
  absent from the service. Assertions were not softened.
- An earlier `just ci` exited zero with all 18 package coverage gates at least
  90%, full tests, Clippy, audit, deny and release build passing. Nine new asset
  tests landed after the coverage binary was compiled, so it is not exact-final
  CI proof. The rerun started on `b4645d7b`, including all root/source/assets/
  candidate diagnostic changes, and **exited zero**. All 18 package coverage
  gates, script coverage, full/minimal-feature tests, Clippy, audits, and release
  build passed. This ledger's documentation-only integration happened during
  that run; no executable source changed. The retained log is
  `ci-b4645d7b.log`. `CARGO_BUILD_JOBS=8` changed parallelism only, not criteria.
  Final Rust LCOV has 226 source records and 100,374 line records, of which
  93,292 are covered. Those are local coverage records, not published Sonar
  metrics or proof for a future restacked revision.
- `just docs-build` succeeded with a **10,416,429-byte search-index WARN**; it
  was not warning-free. `just docs-link-check`: 996 OK, zero errors.
- After ledger integration, `just instruction-drift`, `git diff --check`, and
  `just docs-link-check` passed; the latter checked 1,044 links without errors.
- Full-file Sonar MCP analysis of `scripts/database_rebaseline/contract.rb` and
  `scripts/tests/database-rebaseline-test.rb`, both Ruby in MAIN scope for
  `VannaDii_Revaer`, returned zero issues. This is not a repository/PR scanner
  run, Rust analysis, a published coverage result, or a quality-gate result.
- No GitHub, package, Sonar server/coverage, or overall completion result follows
  from this local checkpoint. Parent owns integrated CI/UI follow-through.

## Approved-Work Integration Checkpoint

Local integration `fe227810` adds the following bounded prerequisites to the
earlier checkpoint. They are not pushed, packaged, or a completed operator flow:

- [ADR 565](565-packaged-baseline-reader.md): read-only baseline projection and
  verification, with 14 focused tests and real malformed-projection refusal.
  It does not initialize, adopt, or authorize a database. An isolated child test
  fixes the captured-event regression without changing production logging.
- [ADR 568](568-root-attestation-identity-encoding.md): pure root identity framing
  and validation within the accepted root contract. The worker's 55 root-catalog
  tests passed. No filesystem attestation, persisted root binding, or write
  authority follows from an encoded identity.
- [ADR 567](567-postgres-pristine-catalog-evidence.md): the exact PostgreSQL 16.14
  pristine snapshot is reproducible; integrated validation and all 131 catalog
  assertions passed. Its SHA-256 is
  `0ba173f3caa88da40a4391e9bd34ac88416a2f7c41f19be47043bfa54a2cbf05`.
  This is catalog evidence, not runtime pristine admission.
- [ADR 566](566-single-init-final-sql-proof.md): finalization is now assembled
  but remains inert. Real SQL proof passed 62 of 64 assertions. Extension
  execution policy and reset-timeout leakage are presented for explicit approval
  in [ADR 569](569-init-privilege-and-timeout-resolution.md), which is still
  Proposed. Final approval review also identified the existing local
  variable-conflict substitution as an unapproved parity exception (D3), not
  demonstrated semantic equivalence. Neither failed assertion has been removed;
  the prototype must not be published, embedded, or activated before approval
  and the independent proof required by D3.
- `just ci` exited zero on this executable revision, including full workspace
  and minimal-feature tests, all 18 package coverage gates, shell/Ruby coverage,
  and the release build. Rust LCOV contains 232 source records, 100,731 line
  records, and 93,644 covered lines. The retained log is
  `ci-capture-isolated.log`; documentation-only checkpoint edits followed.
  These local coverage records are not published Sonar metrics.

The native torrent singleton-directory regression is retained separately in
commit `b6db6667`, pending integration and fresh full-gate validation. Its native
tests passed twice (95 library, seven build-guardrail, and three integration
tests), with no production/C++ change. Its task-owned worktree was removed after
the commit and evidence were preserved. Earlier E2E runs used a fixture root
outside the seeded authoring allowlist. The corrected full `just ui-e2e` run
used `$PWD/.server_root/library` without changing code, permissions, or
assertions: torrent authoring and its dependent tests passed. Overall, 46
passed, one failed, and 61 did not run. Media profile creation still expected
201 and received 400, and teardown reported unexercised job phases/profile
readiness routes. The retained log is `ui-e2e-allowed-root.log`.

The full E2E gate, published Sonar, package validation, and remote stack/review
checks remain open. Task-owned PostgreSQL containers and completed worker
worktrees were removed; `just clean-test-fixtures` passed and retained E2E
artifact roots contained no acquired/generated media files.

## RVB1 And GitHub Checkpoint

Local executable `787b69be` integrates the native singleton-directory regression
as `cf0bf982`, the 25 independent [RVB1 vectors](573-rvb1-canonical-golden-vectors.md),
and the [bounded pure wire codec](571-rvb1-bounded-wire-codec.md). Twenty focused
codec tests pass, including the peer-reviewed allocation-order correction.
Neither lifecycle/containment, trusted-root persistence, single-init cutover nor
the full operator workflow is complete. Fresh `just ci` passed on this executable
checkpoint, including all 18 package coverage gates. Rust LCOV records 236
sources, 101,208 lines and 94,115 covered lines. `just ui-e2e` still failed with
46 passed, one failed and 61 not run: profile creation expected 201 and received
400, plus unexercised phases/readiness routes in teardown. ADR 571 records the
exact-revision evidence, limits and retained logs. No assertion was relaxed.

[ADR 572](572-broker-env-local-image-evidence.md) records twenty isolated probes
of two historical arm64 images. Both have service-owned HOME and a missing
candidate PATH directory. No relevant amd64 image was found and neither image
has immutable current-source provenance. These facts inform E1; they do not
approve values, authorize package changes or validate a current release.

Read-only GitHub CLI/API refresh on 2026-09-10 still finds PR 98 at `7ed723d5`
with 131,057 changed lines, and PR 194 at `cfed91f8`, behind its base. Both are
assigned to VannaDii; PR 194 has no review requests, reviews or review threads in
the complete fetched pages. That is not evidence of a completed Copilot review.
PR 195 is clean against `main`, with 9,964 changed lines and VannaDii assigned;
this bounded metadata read does not establish its merge readiness.

PR 194's applicable stack ruleset 12202805 requires 21 status contexts. The
reported check list, including `gh pr checks --required`, contains only 20 of
them, all passing. **Supply Chain Checks is missing**, so the PR does not meet
all current requirements. Its successful run 31782414466 is from 2026-08-14 on
that exact remote SHA; the workflow there has separate Audit/Deny jobs but no
Supply Chain Checks job. The local integration already includes the required
aggregator and dependency edges. Re-running the old workflow cannot create an
absent job; validated stack reconciliation must carry the current workflow.
Do not remove the required context or infer an all-checks pass from the CLI's
filtered list. No ruleset or GitHub state was mutated during this audit.

The remote checks/rules/state JSON is retained as `rvb1-pr194-*.json` and
`rvb1-pr98-state.json` under the external 2026-09-10 review directory. Sonar's
local secrets scan is distinct from the authoritative scanner, which fails
closed without a local token, and Agentic Analysis, which returned an
organization-entitlement 403. Neither is a published full-Sonar result.

## Root Input And Conversion Gate Checkpoint

The approved outside-in root-input helpers are implemented in
[ADR 574](574-root-input-contract.md), executable commit `32a49c4b`. Its 22
focused tests pass, including five independent cursor vectors. Full `just ci`
passed with all 18 package coverage gates; the helper has 103/104 covered lines.
`just ui-e2e` still failed at profile creation (expected 201, received 400):
46 passed, one failed, and 61 did not run. No new HTTP route, root persistence,
filesystem authority, or production provider is activated by these helpers.

[ADR 575](575-media-conversion-gate.md), integrated as `4b073661`, corrects an
additional validation gap: `test-media-conversion` previously validated only
fixture integrity/probes. It now also invokes the complete Rust fixture binary
with ignored tests included and requires fresh positive runtime evidence.
Preparation and execution reports are separated so stale or preparation-only
reports cannot establish a pass. Structural and controlled execution tests
exercise skipped/zero-test, error-propagation, and report-provenance failures.
These controlled tests are not real conversion proof.

The real fixture attempt downloaded all 22 locked sources and generated the
derived fixtures, but strict probe verification rejected an unapproved EBML
diagnostic for the locked upstream `mkv-theora-vorbis-live-style` source. The
Rust suite therefore did not run. The local probe tools were 9.0.1; the runtime
manifest pins Alpine FFmpeg 8.0.1-r1. No supported-package comparison or exact
compatibility cause has been established. No diagnostic allowance, lock,
snapshot, or assertion was changed. All acquired/generated test media and
completed worker worktrees were removed; ADR 575 retains exact provenance.

Fresh local Sonar inputs were verified for `32a49c4b`, but the authoritative
scanner still lacks local `SONAR_TOKEN` and Agentic Analysis returns the
organization-entitlement 403. The available Ruby snippet analyzer returned
zero issues for both changed guardrail files with full-file MAIN scope; it
does not support Rust. Remote PR 194's Sonar snapshot reports positive coverage
and an OK gate, but does not bind its response to this unpublished revision or
prove no ignored conditions. It is not the required new analysis.

Read-only GitHub refresh confirms the same three remote heads and VannaDii
assignments recorded above. PR 98 remains conflicting and 131,057 changed lines.
PR 194 still lacks `Supply Chain Checks`: 20 reported passing checks are not
the 21 required contexts. No Copilot request is present in its complete current
request list. These are unresolved stack/review obligations, not a remote pass.
The primary checkout and all unapproved init deltas remain untouched. ADR 569
and E1 remain held; approval of 557-559 does not authorize either delta.

The combined executable `79ae7ffa` passed full `just ci` with all 18 coverage
gates, but failed `just ui-e2e` with the same 46/1/61 passed/failed/not-run
counts. Rust coverage records 237 sources, 101,312 lines and 94,279 covered
lines. Final Sonar inputs passed verification; the authoritative scanner still
fails closed without its token. The restored conversion gate is separately
blocked at strict fixture verification, so no whole-feature or all-checks pass
is established. ADR 575 records the final evidence, documentation-only
follow-up, and cleanup. No local integration history was pushed wholesale.

## Pinned Fixture Diagnostic Checkpoint

[ADR 576](576-pinned-ffprobe-fixture-diagnostic.md) compares the same locked
source with host FFprobe 9.0.1 and the existing historical Linux arm64 image's
8.0.1. Both return zero and exactly match the committed Theora/Vorbis snapshot,
but both emit the same element-ID error at byte 35. Official source inspection
confirms this is `AV_LOG_ERROR` and that the implicated parser function is
identical in the two versions. The strict diagnostic gate remains failed;
neither a 9.0.1-only explanation nor a conversion pass is supported. All media,
the disposable container, and both completed probe worktrees were removed.
No fixture exception, replacement, or E1 environment choice was made.

The fresh CI rerun also exposes shutdown `WARN` lines. Recipe exit status must
not be confused with warning-free output. PR 194 still lacks the required
`Supply Chain Checks` context. A Copilot API request returned no substantive
acknowledgement and follow-up reads have not confirmed it; review remains open.
Final integrated gates and documentation validation are retained in ADR 576.
The unchanged executable checkpoint `02de8394` completed `just ci` with exit
zero but the noted shutdown warnings; `just ui-e2e` again failed with 46 passed,
one failed and 61 not run. Fresh local Sonar inputs verified positive coverage,
but the authoritative scanner still failed before analysis without its token.
No package certification, remote all-checks pass, or feature completion follows.

## Shutdown And Fixture Decision Queue Before Approval

[ADR 577](577-runtime-shutdown-event-classification.md) is Proposed, not a
completed runtime correction. The parent prematurely changed shutdown event
classification, then verified ADR 559 G1's explicit approval boundary, stopped
the worker and restored all active executable/recipe/instruction changes.
Commit `2ec8c8a1` has the same executable as `783e4f54`; the local prototype in
`27770554` is audit history only and must not be published or extended without
approval. S1 covers event severity/message shape, S2 the configuration watcher
join and its potentially unbounded cooperative-shutdown completion. Neither
technical review nor historical PR 194 supplies approval.

[ADR 578](578-locked-fixture-diagnostic-disposition.md) presents the exact F1
test-only diagnostic exception, identity/tool bounds, evidence retention,
expiry and alternatives. No fixture allowance or replacement is implemented.
These holds supplement, not replace or resolve, ADR 569 and E1.

A CLI Copilot request for PR 194 exited zero, but immediate REST/GraphQL reads
still showed no requested or completed review. Submission remains unconfirmed;
the review requirement is not complete. No browser or push was used.
Restored checkpoint `2ec8c8a1` completed `just ci` with exit zero and the retained
shutdown WARN events, then failed `just ui-e2e` with 46/1/61 passed/failed/not-run
counts. Fresh Sonar inputs verified positive coverage; the authoritative scan
still failed without local credentials. ADR 577 retains exact results and
cleanup. No runtime change or diagnostic exception is active from these tasks.

## 2026-09-10 Decision-Specific Approval

The operator explicitly approved "D1, D2, conditional D3, S1, and narrowly scoped
F1" and directed "Keep S2 held pending a defensible shutdown bound." This
accepts ADR 569 D1/D2 and conditional D3, ADR 577 S1 only, and ADR 578's exact
F1 exception as reviewed at `1d62d087`. Earlier pending statements in this
ledger are historical checkpoints, not the current approval state.

At approval, implementation and independent evidence were still outstanding.
The subsequent checkpoint below records progress. D3 equivalence is not
established, the final init is not activated, S2 remains
held, E1 exact package values remain held, and no CI/UI/Sonar/package or remote
pass is implied. Preserve every expiry and renewed-review requirement in the
accepted decisions. The primary conflicted user checkout remains untouched.

## Approved Implementation And Independent Counterexample

The local approved-decision integration contains separate conventional commits
for the exact approval record, D1/D2, S1, F1 and independent D3 evidence. These
commits have not been pushed or merged into a GitHub PR. No existing remote
check is evidence for these new source revisions.

- D1/D2: the inert final init has the exact scoped reset delta and digest
  `1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.
  All 105 live extension, authored privilege, baseline and timeout assertions
  pass, including eleven extension mutations, real cancellation and lock
  contention. Eighteen exact-delta harness assertions pass. Independent source
  review found no actionable D1/D2 defect; it is not a separate live rerun.
- S1: expected cancellations requested by the local shutdown operation are INFO;
  panic, unexpected cancellation and grace-expiry warnings remain. Twelve real
  join/event/cleanup tests pass in each all-feature and minimal-feature build,
  with focused Clippy and policy checks. S2's unfinished configuration watcher
  still aborts and drops without joining; no shutdown bound is claimed.
- F1: the exact approved source/snapshot/tool/diagnostic contract is implemented
  with retained raw evidence and seventy adversarial cases. Real host conversion
  passes all six unfiltered tests, including ignored tests: thirty pipeline
  actions, eight video transcodes, six audio transcodes and zero failures.
  Historical Linux arm64 tool preparation is not current package certification.
- D3: the integrated proof now records 122/123 assertions passing. Both the
  untouched frozen-reference function and final candidate fail the warm second
  ingestion with `42P07: relation "tmp_policy_rules" already exists`. Six cold
  scenarios match. The report remains failed and incomplete; full helper and
  compiler-setting closure is not established. Three live harness controls and
  44 unit assertions cover successful warm-write preservation and strict
  diagnostic validation after independent review. ADR 579 proposes exact D4
  transaction-lifetime correction; no such SQL delta is authorized or applied.

## Integrated Gate Checkpoint For The Approved Decisions

The executable/proof checkpoint is `4c9912b9`. Full `just ci` exited zero,
including all-feature and minimal-feature tests, Clippy, policy, audits, all
18 package coverage gates, script coverage and the release build. The corrected
44-assertion ingestion harness ran in the final script-coverage phase after
its review fixes; focused final/rebaseline tests also pass 18/45 assertions.
Eight configuration-watcher shutdown WARN events remain under held S2. This
is not warning-free output or permission to change its completion contract.

Full `just ui-e2e` exited one: 46 passed, one failed and 61 did not run. Media
profile creation still returned 400 instead of 201 at API spec line 130.
Teardown rejects missing job-phase and profile-readiness GET coverage. The API
log also retains a missing local final-image compliance bundle error, a slow
recovery-query warning and a missing library mount warning. No assertion,
dependency-project gate, route-coverage requirement or production criterion was
changed to conceal those outcomes; no full operator-path pass is established.

`just sonar-compile-db` passed after CI; after E2E,
`just js-release-coverage js-coverage-merge sonar-verify-inputs` passed.
Rust LCOV has 238 sources,
106,833 lines and 98,336 covered lines. JavaScript LCOV has 60 sources,
5,276 lines and 3,924 covered lines from executed paths, not the 61 unrun cases.
The final live database proof and 44/18/45-assertion harnesses were separately
instrumented with Ruby coverage and merged through the existing generic
converter. Extension and timeout proof modules report 28/28 and 70/70
executable Ruby lines; ingestion reports 165/171. These line counts do not
replace SQL semantic proof or establish complete branch coverage.

Sonar verification was blocked before execution by source-upload authorization:
`just --command sonar verify --file scripts/database_rebaseline/final_sql.rb
--project VannaDii_Revaer`. Explicit consent to upload changed source was
requested rather than bypassing the denial through another analyzer. No changed
file has a new server analysis, published coverage or quality-gate result from
this turn. Local input validity is not a substitute for that required result.

Documentation indexing and instruction checks pass with 521 entries; link
checking reports 1,114 OK and zero errors. The book build exits zero but retains
the large-search-index WARN. The complete local range from `1d62d087` through
`4c9912b9` has 2,649 changed lines under the 9,999-line guard; no current GitHub
PR-size or all-checks claim follows. Full package checks, remote stack/check
completeness, review resolution and the remaining approval holds are outstanding.
The user checkout is unchanged. Logs, failed database proof and non-media
coverage inputs are retained under the private temporary evidence directory;
the real F1 conversion evidence remains in its separate private directory.

## Helper Compilation And Sonar Follow-Through

[ADR 580](580-ingestion-helper-compilation-evidence.md) records six added
helper-first cases within conditional D3. Forty-two exact answers across nine
pure helpers are checked before ingestion in each case, under unchanged caller
settings. The full proof still fails the required successful warm second call;
no complete helper/trigger closure or certified final baseline follows.
Independent review also tightened malformed-record rejection and diagnostic
identity/ordering checks. Frozen migrations and final-init bytes are unchanged.

The operator subsequently granted the source-upload permission requested above:
"This command is approved and permission to upload to the service is granted"
(2026-09-10). The exact `sonar verify` command for
`scripts/database_rebaseline/final_sql.rb` then ran and returned HTTP 403:
Agentic Analysis is unavailable for the organization. A complete-file Ruby MAIN
analysis through the Sonar MCP tool for `VannaDii_Revaer` returned zero issues.
This is only that file's result, not coverage publication or a full quality-gate
pass. The authoritative local scanner also lacks its required `SONAR_TOKEN`.
No scanner criteria or architectural approval hold changed.

The helper-proof checkpoint `0eb38ecf` has a fresh zero-exit `just ci` with all
18 package coverage gates, but eight held-S2 shutdown WARNs remain. Full E2E
again reports 46 passed, one failed and 61 not run at the unchanged profile
creation/route-coverage blockers. Local Sonar inputs validate; no current full
scanner or published coverage pass is established. See ADR 580 for exact counts,
proof-review provenance and the unchanged 134/135 database proof outcome.

The same follow-through verified VannaDii assignment on all 104 open
`stack/media3-*` PRs and submitted one Copilot request per PR. None was visible
as pending or submitted review in the final read-back, so fulfillment remains
unconfirmed. The live stack ruleset still requires 21 checks, while PR 194's
unchanged head does not define Supply Chain Checks. No source, stack or criteria
mutation was used to conceal the missing result.

## Task Record

- Motivation: Provide a bounded completion ledger while the integrating agent
  runs integrated validation, without restarting architecture review or treating
  partial implementation as a finished service.
- Design notes: Keep the ledger in this one task ADR rather than introducing a
  second status document. Scope edits to ADR 564, its two indexes and generated
  catalog; preserve all other documents and checkouts. Remote queries are
  read-only CLI calls. No pushes, PR mutations, or implementation changes.
- Test coverage summary: No runtime tests or fixtures added. Documentation
  validation results are recorded below. Full `just ci`/`just ui-e2e` remain
  parent-owned integration requirements; their omission here is not a pass or
  a change to the repository completion rule.
- Observability updates: No runtime changes; the ledger distinguishes source,
  parent-reported validation, held decisions and unassigned PR ownership.
- Status-doc validation: Reviewed the specification and accepted resolutions;
  existing README/operator guides are not changed. This record makes no claim
  that those broader guides have been exhaustively reconciled.
- Risk & rollback plan: A status ledger can become stale or overstate proof.
  Keep revisions and evidence classes attached; re-audit on integration. Revert
  this docs-only commit and regenerate indexes/catalog if rejected. No runtime
  state or architecture requires rollback.
- Dependency rationale: No dependencies added or changed.
- Stale-policy check: Reviewed `AGENTS.md`, the ADR template, data/UI/devops
  scoped instructions, and relevant accepted root, broker, lifecycle, audio,
  discovery, fingerprint and baseline contracts. The spec's old open-question
  wording and historical migration-retirement prose must be read with the
  later accepted resolution and current assembly guard. These are recorded
  provenance caveats, not edits to excluded files. No contradiction is resolved
  by inventing approval, changing frozen migrations, relaxing criteria or
  removing an included feature.

## Ledger Validation

- `just docs-index`: passed; generated 505 catalog entries. Only this ADR's
  entry and generator metadata changed in the two generated catalog files.
- `just instruction-drift` and `git diff --check`: passed.
- `just docs-build`: exited zero with the existing large-search-index WARN;
  this is not a warning-free documentation result.
- `just docs-link-check`: 1,042 OK, zero errors.
- `just clean-test-fixtures`: passed; the four ignored binary fixture
  directories are absent. No media was acquired/generated or runtime process
  started by this documentation task. Committed probe metadata is preserved.
- No runtime tests, full CI/UI, package validation or Sonar scan ran here.
- This is a completed documentation subtask only, not feature completion.
  Parent will review/cherry-pick the commit and remove its completed worktree.

## 2026-09-10 Complete Boundary Audit And Existing-Data Proof

[ADR 581](581-stack-boundary-reconciliation.md) records all 104 current media
PR boundaries. Base names form one linear chain, but formal membership is split
into stacks 204 and 202. Only PR 98 fails ancestry. The canonical no-rename,
binary-rejecting size guard also rejects PR 130's binary deletions and PR 186's
97,202-line direct diff; GitHub's displayed totals are not sufficient. No remote
source or stack metadata changed. Current PR 98/99 read-back has no unresolved
review threads, which does not certify code or other PR feedback.

The isolated PR 99 reconstruction is corrected at `377a082e` with 1,368 changed
lines against `f87bea0b`. Focused schema tests and strict API lint pass, while
full CI/UI stop at the old dependency security baseline. A descendant read-only
phase schema reintroduction remains to reconcile. The original refs are intact;
no oversized integration history was pushed.

[ADR 582](582-ingestion-existing-data-evidence.md), commit `4c16d775`, extends
D3 with eight existing-data cold-backend cases and 165 harness assertions.
The pinned live proof has 149 passing checks out of 151. Both variants retain
the existing warm `42P07` failure and a new IMDb conflict-inference `42P10`
failure. [ADR 583](583-ingestion-imdb-conflict-inference.md) proposes narrow D5;
it is not approved or implemented. D4, S2 and the other exact holds remain open.
The final candidate digest and frozen migrations are unchanged, and init
remains inert. Complete integration gates and all release acceptance remain
required; none of these partial results certifies D3 or the service.

The later managed-workspace checkpoint in ADR 581 records exact-tree Linux
arm64 CI, native and 101-test E2E passes at `b5ab8eec` / rewritten `63d1e258`.
All 18 package coverage gates pass with positive local Rust and script inputs.
The 40 held shutdown WARN records and missing published Sonar evidence remain;
these results neither close the integration E2E blocker above nor certify a
supported release package. Reconstruction continues through `011da317`, with
the recipe-test fix folded into its owner and every runtime patch preserved.
No source push or merge is claimed, and all architectural holds are unchanged.

The next execution/API boundary passes full CI and native tests at `011da317`,
but is not E2E-passing: 101 tests pass while teardown rejects 39 uncovered media
API routes. A discovered recipe-status bug had masked that failure. The bounded
fix and 28 regression cases now propagate the real failure; `13da9c59` exits
one on a complete rerun. ADR 581 records this correction, generated-schema
drift, and the rebuilt stack through `d540f67c`. Later dependency audit/deny
checks pass without new exemptions, but neither those checks nor exact-tree
replay proves a complete operator workflow, passing E2E or release acceptance.
