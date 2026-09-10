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
| L4: Discover manually and enable automation explicitly | [Discovery service](../../crates/revaer-app/src/media_discovery_runtime.rs) has native watcher events, scan cursors, and an in-memory `last_scheduled` map. [Frozen procedure](../../crates/revaer-data/migrations/0182_media_normalized_configuration_identity.sql) rejects legacy automation writes with `media_profile_filesystem_identity_required`. | Finish L2, durable discovery/aggregate state under [516](516-durable-discovery-scheduling-and-versioned-aggregate-identity.md), and bounded fingerprint admission under [535](535-bounded-cancellation-aware-fingerprint-admission.md). Obtain held measured values/semantics before activation. Prove default-off behavior, manual plan/audit-only discovery, enabled watcher/schedule enqueueing, restart/overflow/missed-event recovery, deduplication, and cancellation/quota accounting. Enabled automation is included, not expendable to make UI tests pass. |
| L5: Inspect media and refresh trustworthy capabilities | [Capability probes](../../crates/revaer-media-runtime/src/capabilities/detect.rs) use an injected supervisor; [inspection adapter](../../crates/revaer-media-runtime/src/inspect/adapter.rs) composes probing and inspection. [Bootstrap](../../crates/revaer-app/src/bootstrap.rs) constructs the current system supervisor. | Complete [519](519-immutable-media-capability-execution-identity.md) full native dependency closure and immutable job binding, plus [554](554-preemptible-native-process-broker-contract.md)/[558](558-rvb1-native-process-broker-wire-contract.md) broker routing and containment. No RVB1 app/data/API implementation was found. Prove stale/mutated tool rejection, both-lane readiness, bounded probing, secret-free environment, and all advertised tool/filesystem capabilities on both Linux architectures. E1 blocks production environment selection. |
| L6: Inspect explanations, compare state, preview a safe plan | [Core pipeline](../../crates/revaer-media-core/src/pipeline.rs) compiles targets, diffs graphs, prunes candidates, and returns selected/rejected explanations. [Job runtime](../../crates/revaer-app/src/media_job_runtime.rs) persists planning evidence and has an explicit dry-run completion branch. | Verify immutable source/target/policy/capability inputs, deterministic selection, compliance/scoring, cost/disk estimates, DAG dependencies, and cache invalidation across the complete configured graph. Prove every declared operation is executable and verifiable, rejected plans remain explainable, and dry-run produces no source-adjacent artifacts. Core enum coverage is not operator workflow coverage. |
| L7: Execute the requested media transformations | [Execution builders](../../crates/revaer-media-runtime/src/execute/mod.rs) construct desired-graph, subtitle and transcode arguments. They explicitly reject arbitrary `MetadataRewrite` without a verified desired-metadata contract. The same file still has direct `Command::new` execution; audio analysis and verification have separate direct runners. | Finish broker/closure routing, then validate copy/remux, labels/dispositions/order, subtitle copy/embed/extract/conversion without OCR, audio/video transforms, multi-video, metadata, HDR/color, chapters, attachments, unmatched-family policies, and configured quality safeguards. Hold audio preset/bitrate interpretation under [515](515-versioned-audio-transformation-and-acceptance-contract.md). Real fixtures must assert planned operations and independently inspected output; existing literals or hand-built fixture capabilities are not approved production behavior. |
| L8: Verify output, replace safely, recover and retain evidence | [Job orchestration](../../crates/revaer-app/src/media_job_runtime.rs) performs candidate/final verification, replacement commit, rollback/finalization, and startup recovery. [Replacement](../../crates/revaer-media-runtime/src/replacement.rs), [verification](../../crates/revaer-media-runtime/src/verification/mod.rs), and [workspace retention](../../crates/revaer-app/src/media_workspace_retention.rs) contain implementations, not a packaged safety proof. | Complete [512](512-fenced-resumable-worker-ownership-and-recovery.md)/[513](513-attempt-scoped-workspace-retention-transaction.md)/[514](514-packaged-media-subsystem-lifecycle-and-health-contract.md) ownership, resumability, per-root barriers, and cleanup transactions with held values resolved. Prove source/root races, disk refusal, verification failure, backup/quarantine, concurrent stale owners, crashes at filesystem/database/outbox boundaries, rollback, cancellation/shutdown, retention and compact audits. The replay's one-hour stale threshold is not approved resumable recovery. |
| L9: Operate through API/UI and diagnose failures | [Handlers](../../crates/revaer-api/src/http/handlers/media.rs), [UI](../../crates/revaer-ui/src/features/media/view.rs), and [API](../../tests/specs/api/media.spec.ts)/[UI tests](../../tests/specs/ui/media.spec.ts) expose configuration, capabilities, previews and job diagnostics. The UI test includes a surface-rendering check, not full service acceptance. | Deliver L2-L8 through the real authenticated workflow, including explicit manual `replace` override without changing saved dry-run, cancel/retry/re-plan, timelines, SSE, health and bounded metrics/logs. Reconcile OpenAPI/client/teardown routes and retention controls. Run the full UI suite against the same installed service and database, not route mocks or softened assertions. Parent checkpoint below is currently failing. |
| L10: Ship supported packages and merge reviewable deliverables | [Dockerfile](../../Dockerfile), [media recipes](../../just/media.just), [PR workflow](../../.github/workflows/pr.yml), and [Sonar criteria](../../sonar-project.properties) provide build/validation surfaces. [Fixture source](../../crates/revaer-media-runtime/tests/media_fixtures.rs) includes real generated audio fanout with a hand-built capability snapshot; this does not exercise the full packaged operator lifecycle. | Validate clean Linux amd64/arm64 installs, complete spec fixture matrix, tool inventories/redistribution evidence, ExifTool boundary, immutable closure, containment and E1 values. Retain per-image digests/reports. Reconcile one linear stack and recheck every exact diff against the canonical 9,999-line limit; require all applicable checks, positive strict Sonar coverage/evidence, Vanna assignment, requested Copilot review and addressed actionable feedback. No package or remote pass is established here. |

## Critical Path And Decision Queue

1. **First runnable operator slice: L1-L3, then L4 manual discovery/L6 dry-run.**
   Finish the approved normalized root/profile workflow and coordinated init
   lifecycle. Use the reproduced API/UI failure as an acceptance boundary, not
   a reason to disable an included control. Keep root generation, whole-root
   prefix, overlap locking and the approved 128-association bound unchanged.
2. **In parallel: L5/L10 package and native identity evidence.** Implement only
   approved broker/closure mechanics. Present E1's exact environment and HOME
   ownership proof for amd64/arm64 for separate approval; do not install the
   candidate table as policy. B1's proven-clean degradation is not permission
   for unsafe startup or failed recovery to continue.
3. **Batch measured held choices for approval, then enable affected behavior.**
   L4 needs ADR 516 scheduler/traversal/hash budgets, aggregate encoding and
   limit semantics, and ADR 535 admission values. L8 needs ADR 512 lease,
   heartbeat/stale recovery values, ADR 513 cleaner takeover/expiry, and ADR 514
   retry/backoff values. L7 needs ADR 515 preset/order/bitrate acceptance
   evidence. Preserve already fixed values; approval of 557-559 supplies none
   of these missing choices. Gather evidence independently where safe.
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
