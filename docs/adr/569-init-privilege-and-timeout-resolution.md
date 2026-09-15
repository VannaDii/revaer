# Init privilege and timeout resolution

- Status: Accepted
- Date: 2026-09-10
- Operator approval: 2026-09-10: "Approve **D1, D2, conditional D3, S1, and narrowly scoped F1**. Keep **S2 held** pending a defensible shutdown bound."
- Context: ADR 566's real PostgreSQL 16.14 proof found an extension-privilege
  ambiguity in ADR 551 and a confirmed timeout-scope incompatibility. Final
  approval review also found an unapproved compiler-setting parity exception.
- Decision: D1 and D2 accepted; D3 accepted conditional on the independent
  semantic proof specified below. Runtime cutover remains disabled pending all
  required conformance and application evidence.
- Consequences: D1 explicitly chooses the authored-versus-extension privilege
  boundary; D2 and D3 change exact legacy parity and setting-scope constraints. These
  are operator decisions, not G1 internal refinements.
- Follow-up: Implement only the accepted
  delta, regenerate its exact init digest, and repeat all conformance gates.

## Approval Resolution

The operator accepted D1, D2 and conditional D3 exactly as reviewed at commit
`1d62d087`. D1's pinned extension boundary and expiry remain mandatory; D2
changes only reset timeout scope; D3 requires independent cold/warm ingestion
evidence and a renewed decision if any further semantic delta is necessary.
The proposal wording below preserves the reviewed recommendations and prior
hold history; it does not leave these decisions awaiting approval. Approval
does not establish behavioral equivalence, certify a final digest, activate
the single-init cutover, or release any unrelated hold.

The current [D3 closure map](support/569-d3-closure-map.md) reconciles the original
eleven audit families into existing witnesses and concrete remaining work.
It supersedes generic next-step descriptions, not approval conditions or prior
evidence limitations. Detailed historical execution records below remain intact.

### Closure-map implementation checkpoint (2026-09-13, In Progress)

- Motivation/design: the bounded eleven-family reconciliation identified actual
  normalization/wrapper gaps and an older compilation producer without per-run
  source evidence. New private run directories preserve historical reports;
  source drift, missing/changed source/image pins and duplicate JSON fail closed.
  Dependency consumers still require the producing process's validated bytes.
- Implementation: six empty/space-only explicit-hash inputs exercise new, reused
  and GUID-promoted identities. The exact whitespace-only derivation runs before
  and after commit. Both fresh drop actions traverse the current public wrapper,
  retaining observations/decisions, complete 18-table and policy-input images,
  rollback sequence gaps and an unrelated visible result without new page or
  best-context records. No frozen or final SQL, runtime behavior or D3 guard changes.
- Verification: checkpoint `df24ea2d` plus the archived source deltas passes
  4,836 provenance-path live checks, then 1,509 focused normalization/drop checks
  under the pinned PostgreSQL image. Combined unit suites pass, including 50,837
  ingestion assertions and 228 dependency assertions. The earlier ad hoc unit
  attempt exposed the expected inventory-count mismatch while the worker's
  companion change was not yet integrated; that failed log is retained.
- Sonar: the available MAIN-scope Ruby snippet analyzer reports zero issues
  for the eight changed Ruby files. Exact per-file hashes/results are retained.
  `just --command sonar verify --file scripts/database_rebaseline/ingestion_compilation.rb --project VannaDii_Revaer`
  returns the existing organization-entitlement HTTP 403. This is not a full scan
  or positive published coverage; full CI/UI results are recorded below.
- Full gates on that source delta: CI passed, including all 18 package coverage
  gates and release build. UI passed 49 tests, failed media scheduling and
  torrent authoring, left 79 unrun and failed API coverage. The new private
  fixture root exposed missing test-only allowlist wiring; the subsequent
  repair is recorded in ADR 586. Scheduling still requires real root binding.
- Observability/risk/rollback: only disposable proof evidence changes. No new
  dependency or architectural decision. Revert the proof cases and companion
  source/JSON guards together; no database state or bootstrap changes to undo.
  Scoped passes cannot replace the remaining temporal/warm/native D3 evidence.
- Stale-policy check: root, data, DevOps and Sonar instructions reviewed.
  DevOps now specifies the producer/consumer boundary and new fixture obligations.
  No approval condition, D4/D5 outcome, frozen authority or quality criterion is
  relaxed. Completed agent worktree removed after its changes/evidence were retained.

### Committed Observation Qualification (2026-09-14, In Progress)

- Motivation/design: close the finite temporal and retention obligations in
  the closure map using existing wrapper transport and complete-state models.
  No production SQL, bootstrap authority, dependency or approval changes.
- Retention: both 26th-sample cases pass cold/helper-first qualification, with
  26 commits on each tested backend. Newest retention removes the oldest
  observation; a late older sample is itself removed. All 18 tables, rollup
  values, exact identity sequence and immutable policy inputs are checked.
  Five sampling cases produce 20 variants; the live driver passes 245 checks
  including its prerequisite D4/D5 matrix. Unit coverage passes 246 assertions.
- Temporal: six GUID-less/GUID-promotion and older/equal/newer cases pass in
  cold, helper-first and committed-warm modes: 36 variants and 261 live checks
  including prerequisites. The independent oracle checks all last-seen fields,
  incoming observation fields, identities, samples, full write/read images and
  transaction provenance. Unit coverage passes 464 assertions; coherent state
  mutations, reconnects, duplicate JSON and changed D4 diagnostics are rejected.
- Frozen committed reuse remains the exact approved D4 failure, not successful
  equivalence. Both live drivers report `d3_complete=false` and successful owned
  container/volume cleanup. Source hashes are retained per run. The combined
  ingestion unit driver passes. Full combined CI passes without WARN/compiler
  warning lines, including all 18 package coverage gates and the release build.
  Full UI retries after a retained port collision: torrent authoring now passes;
  57 tests pass, scheduling fails with `media_profile_filesystem_identity_required`,
  72 dependent tests remain unrun and UI route coverage is missing.
- Evidence: `target/d3-reconcile/committed-retention-live-20260914.log`,
  `temporal-live-20260914.log` and `committed-temporal-unit-20260914.log`, with
  private sampling run `run-20260913-10411-gif31r` and temporal run
  `run-20260913-19983-dh0y4v`. The retained source deltas remain unpublished.
- Sonar: all six newly touched Ruby files and three TypeScript files receive
  MAIN-scope file analysis. Three setup-file style findings are fixed and its
  exact recheck reports zero; other files report zero. Earlier eight Ruby-file
  receipts remain content-matching. The full changed-file secrets scan and
  final local input verification pass. CLI agentic verification still returns
  organization-entitlement HTTP 403; `SONAR_TOKEN` is absent. No canonical
  repository analysis or positive published coverage is established. Evidence
  is retained in `artifacts/media-verification/2026-09-14-committed-ingestion-and-fixture-roots/`.
- Observability/risk/rollback: only disposable proof records change. Remove the
  added cases and their canonical/unit registration together to roll back; no
  live application data changes. Root, data, DevOps and Sonar policy reviewed;
  DevOps now records these exact proof obligations without relaxing criteria.
  Post-commit hash-fill/scoring/paging and canonical native evidence remain.

### Committed Hash Fill And Paging (2026-09-14, In Progress)

- Motivation/design: close the specific post-commit identity and page-boundary
  gaps using the existing public-wrapper transport and independent table models.
  Source is `45ffb436` plus retained integration deltas; runtime SQL, frozen
  migration authority, the final-init digest and the incomplete-D3 guard are
  unchanged. No architectural decision or new dependency is introduced.
- Hash fill: six explicit v1/v2/magnet and competing/uncontested cases each run
  cold/helper-first against frozen and final databases: 24 variants, 249 live
  checks including prerequisites, and 1,154 unit assertions. Warm, fill and
  reuse share one tested backend across three commits. Complete 18-table and
  19-input images, stable identities, conflict history and five sequence values
  are independently checked. The initial expected model omitted conflict IDs
  consumed before frozen D4 failures; the corrected model retains those
  allocations while requiring rolled-back rows to remain absent. The initial
  failed run and its successful cleanup are retained.
- Paging: two new/original-item reuse cases each run cold/helper-first on both
  databases: eight integrated variants and 233 live checks including
  prerequisites; 289 unit assertions pass. The tenth item fills page one; a
  committed eleventh item seals it and starts page two. Reuse changes neither
  page nor item membership. All 18 sequences, full state, policy inputs and
  fixture/tested backend separation are checked. Failed frozen calls remain
  exact D4 errors, including their nontransactional identity allocations.
- Scoring/title: eight integrated cold/helper-first and frozen/final variants
  pass, with 233 live checks including prerequisites and 1,032 focused unit
  assertions. Four tested commits cover warm-up, lower-score observation,
  higher-ranked title refresh and reuse. Distinct 10/100 scores, the 99/100
  seeder boundary, selected identities, full state/read inputs and nine
  sequences are independently checked. The public wrapper's final best-source
  write remains distinct from v1's promotion and score-ranked title choice.
- Evidence: integrated hash run `run-20260914-95940-3w781` and paging run
  `run-20260914-98326-5f1rul`, with live logs under `target/d3-reconcile/`.
  Both drivers report successful owned container/volume cleanup and
  `d3_complete=false`. The paging agent's sources and earlier evidence are
  archived; its completed worktree and temporary branch have been removed.
- Scoring evidence is `attributes-live-6332-c51028c7.json` and its referenced
  private matrix directory. Its agent source/evidence archive and failed
  sandbox launch are retained; the completed worktree/branch are removed.
  All three integrated drivers retain exact source hashes and successful
  cleanup. Their shared prerequisite check counts are not additive.
- Sonar: MAIN-scope file guidance reports zero issues for all eight changed Ruby
  files. CLI verification of `ingestion_committed_hash_fill.rb` still returns
  the organization-entitlement HTTP 403; no canonical scan or positive
  published coverage is established. Changed-file secrets checks pass. The
  documentation link gate passes all 1,407 links after a retained sandbox
  network failure. Canonical scanner credentials remain absent.
- Full gates: `just ci` passes with no WARN/compiler-warning lines, all 18
  package coverage gates and the release build. Its log executes all three
  new unit suites both ordinarily and with coverage. Each of the eight changed
  Ruby files has positive executed-line coverage. `just ui-e2e` retains the
  scheduling identity-required 400: 57 passed, one failed, 72 unrun, plus absent
  UI-route coverage. Both owned services, volumes, host data directories and
  the empty private E2E media root are removed; managed test-media cleanup
  passes. Local Sonar input validation passes, not a published Sonar gate.
  Gate directories are `revaer-host-backed-ci-ab8545c2955e` and
  `revaer-host-backed-ui-e2e-c6996f0004e7` under `target/`.
- Supplemental coverage: the unchanged three live matrices also pass under
  the existing Ruby coverage collector, with successful cleanup. Regenerating
  the standard generic report raises coverage across the eight changed Ruby
  files from 1,090 to 1,292 of 1,392 lines; the three new proof modules cover
  443 of 448 lines. Original CI coverage, raw collector records and the final
  merged report are retained. The first merge command incorrectly disabled
  Ruby's existing gem loader; the canonical loader succeeds. A read taken
  before that merge completed is retained separately, not used as final proof.
  Current-source post-merge input validation passes; no published metric follows.
  Evidence is sealed in
  `artifacts/media-verification/2026-09-14-committed-boundaries/`.
- Observability/risk/rollback: only disposable verification output changes.
  Remove these modules and their shared proof/unit registrations together to
  roll back; no live data changes. Root, data, DevOps and Sonar instructions
  were reviewed. DevOps now names these bounded obligations; no acceptance
  condition is relaxed. K1 and canonical native qualification remain on the
  existing closure map; this is not full D3 or cutover acceptance.

### Native Trust Rank Observation (2026-09-14, In Progress)

- Motivation/design: close K1's missing local-rank and branch evidence without
  changing SQL or substituting equal confidence for actual branch observation.
  Source remains `7c345bd5`; the ignored observer reuses the pinned images and
  canonical attribute fixtures. It reads native Boolean and assignment
  entry/return state, function/source coordinates and local datum identities.
- Capture: NULL-instance-key and missing-public-tier cases pass in cold and
  helper-first modes against reference/final databases: eight observed contexts,
  each paired with a plain run. All complete application oracles pass, including
  the exact third-call frozen D4 error. There are 42 events per NULL-key context
  and 54 per missing-row context. Independent source-derived readback qualifies
  all 24 calls and 384 events, including lookup skip versus NULL-rank fallback,
  exact local rank/bucket values, source coordinates and compilation settings.
  Raw/application bytes, complete-state oracles, helper-first and snapshot
  intervals, catalog identities and retained producer hashes are checked.
- Readback tests: 156 assertions pass, rejecting 129 mutations. Parent review
  found that unrecorded parent-FK/regex markers could be ignored; a coherent
  raw-line mutation reproduced acceptance before the explicit rejection was
  added. The failed regression and successful integrated suite are retained.
  All three successful captures also pass source/native/header binding checks.
- Calibration: the existing rank-40 fixture executes three operations per
  variant. Native reads retain rank 40, bucket 3, true non-NULL-key and
  rank-at-least-40 answers, and a false missing-rank answer. Both complete
  plain/observed application comparisons pass.
- Diagnostics: the first capture stopped at the strict JSON container's
  comparison conversion; its failure is retained. The successful K1 capture
  retains the missing musl source warning. Providing only the matching
  [musl 1.2.6 source](https://musl.libc.org/releases.html), mounted read-only
  in the debugger, removes that warning in the repeated small calibration pair.
  No target binary, warning switch or logging criterion changed. Earlier
  warned captures are not relabeled warning-free.
- Evidence/observability: captures `fk-compilation-run-20260914-70282-pnmifo`
  and warning-free calibration `fk-compilation-run-20260914-76159-urff4t` under
  ignored `target/d3-native-ri/` pin source/native identities and retain full
  application/debugger records. The calibration readback is
  `target/d3-reconcile/trust-rank-calibration-readback-20260914.json`; qualified
  K1 readback is `trust-rank-readback-qualified-20260914.json` in that directory.
  Every owned trial container and host data directory is independently absent;
  managed test-media cleanup passes. The completed validator worktree/branch
  and restored debugger image are removed after preserving their inputs.
  The source and evidence archive is
  `artifacts/media-verification/2026-09-14-native-trust-rank/`.
- Risk/rollback/dependencies: disposable observation only, with no new runtime
  dependency or canonical debugger adoption. Removing the ignored observer
  changes no application behavior. The existing debugger image and added
  source-listing input remain isolated from target binaries.
- Stale-policy check: root, data, DevOps and Sonar instructions reviewed. No
  application-source, criteria, approval, frozen-authority or bootstrap changes.
  Full CI/UI
  results remain those of `7c345bd5` above; no fresh full gate or published Sonar
  result is claimed for these experiments. Changed-document secrets scanning
  passes; per-file Sonar analysis returns the organization-entitlement 403 and
  the canonical token remains absent. The final documentation link gate passes
  all 1,410 links after two retained runs with existing GitHub FFmpeg source-link
  503s, without accepting error statuses or changing links. K1's readback is qualified;
  canonical adoption and remaining D3 obligations stay open.

## Observed Conflicts

The source for this review is the inert finalization in commit `b74d6b1a`
(worker commit `d451169c`). Its final-init SHA-256 is
`d27d2d99a0957b2502d0461b27c29ed1f2a53e486d1a140e7aff8e5429d7f069`.
The proof completed 64 checks: 62 passed and these two failed. This digest is
not a release certification and has not been embedded or deployed.

1. **Extension execution ambiguity.** The constrained owner installs trusted
   `pgcrypto` and `unaccent`, but 40 extension routines belong to PostgreSQL's
   bootstrap superuser. They retain PUBLIC EXECUTE ACLs in `public`, which the runtime must
   be able to use for application procedures. Omitting explicit runtime grants
   does not remove those ACLs. This catalog result does not prove every routine
   can be invoked directly: two have internal-only callback signatures. The constrained owner cannot
   revoke privileges on those superuser-owned routines. This is PostgreSQL's
   documented [trusted-extension ownership behavior](https://www.postgresql.org/docs/16/sql-createextension.html),
   not evidence that Revaer needs an unrestricted bootstrap account. ADR 551
   explicitly restricts authored routines; its extension-capability and broader
   default-privilege wording do not unambiguously decide inherited extension
   execution. The new proof selected the stricter interpretation. It must not
   be presented as an already-approved zero-extension-execution requirement.
2. **Timeout leakage.** The seed path calls
   `revaer_config.factory_reset_without_media_defaults_v1`, whose existing body
   executes `set_config('lock_timeout', '5s', true)`. It leaves the encompassing
   transaction at `statement_timeout=2min`, `lock_timeout=5s`, and
   `idle_in_transaction_session_timeout=30s`, instead of `2min/2min/30s`.
   Preserving that body unchanged conflicts with the caller-owned timeout rule.

The proof rejects both outcomes. Separately, its reference normalization already
contains D3's unapproved substitution. That is a local prototype mistake, not
consent or a semantic proof; it has not been pushed, embedded, or used by runtime.
No role privilege, extension ACL, timeout value, frozen migration, workflow, or
remote setting was changed to conceal the two failures. Independently passing
pristine-catalog and baseline-reader tests resolve none of these approval gaps.

### Native Warm Helper Integration (2026-09-14, In Progress)

- Motivation/design: complete the remaining magnet-URI and title-size native
  warm-cache observations using the existing identity inputs and correction
  transaction harness. Each variant pairs plain/native-observed execution,
  preserves complete application frames and immutable read inputs, validates
  exact identity/hash answers and retains frozen D4 diagnostics separately.
  Source/catalog binding and readback boundaries cover both real calls on one
  backend across commits; no production SQL or bootstrap change.
- Tests: the bounded worker's phase validator passes 1,790 mutation assertions;
  the integrated shared settings/warm capture suite passes 66, including exact
  plain-arm setting/lifetime checks and predecessor registration. Frozen-source
  live qualification passes all four pairs and 73 producer checks in
  `ingestion-native-warm-helpers/run-20260914-50037-s82vx2`, under the ignored
  database proof output. Source/target identities and independent cleanup pass;
  the report's source hashes match the current delta over `19d59c7e`. Full
  `just ci` passes (`target/revaer-host-backed-ci-c446d29e98cf`). The complete
  method replay has 9,851 passing checks and only the failing conditional-D3
  completeness guard; it is not a passing final proof. Aggregate tests
  rejected an entrypoint-only edit to
  the source-pinned ingestion module; registration now uses the existing native
  module override pattern, preserving the independently qualified source pin.
- Observability: retain private raw debugger/application output, source and
  target fingerprints, ABI headers and independent owned-resource cleanup.
  The first disposable run failed during provisioning with PostgreSQL disk-full
  SQLSTATE `53100`, before the proof ran. Its diagnostic is retained in
  `target/d3-reconcile/native-warm-first-startup-error-20260914.txt`. The retry
  uses the existing gate harness's private host-backed PostgreSQL storage
  pattern, retains startup logs and removes only owned resources; no Docker
  pruning or production storage change is authorized. Retain both the earlier
  passing `run-20260914-47013-x8l3qd` and rejected
  `run-20260914-48507-dp5ixe`: the latter correctly failed source identity after
  the integration owner edited its capture test during execution. All three
  completed observation runs have passing separate cleanup receipts. Only the
  final frozen-source run qualifies the current delta.
- Coverage/UI: retained current CI Rust/script coverage and source hashes in
  `target/d3-reconcile/native-warm-coverage-c446d29e98cf.json` before UI. All
  eight changed Ruby files have positive executed coverage; the new validator
  covers 90/91 executable lines, while the proof driver covers only 31/109.
  Uninstrumented live observations are not merged into those counts. Retained
  JS/native compile-database inputs keep their older mtimes; this is not a
  complete fresh Sonar submission or published coverage. `just ui-e2e` in
  `target/revaer-host-backed-ui-e2e-e1846e826672` has 57 passed, one failed and
  72 unrun: the scheduled-profile request at `media.spec.ts:484` still returns
  `media_profile_filesystem_identity_required` (400 instead of 200). Teardown
  also rejects absent UI coverage. Owned databases, host data and UI media root
  were removed, followed by `just clean-test-fixtures`. Documentation links
  pass 1,430/1,430. Neither test expectations nor criteria were relaxed.
- Full replay environment: the first unchanged-method attempt at
  `target/d3-reconcile/native-warm-full-20260914-70930-3hv3z7` failed before
  readiness because PostgreSQL could not open `/tmp/native-policy/observation`;
  retained diagnostics and storage cleanup confirm no proof acceptance. The
  retry keeps that exact log path/configuration on a disposable 64 MiB tmpfs,
  preserves its contents before cleanup and uses private host-backed PGDATA.
  This is test-only storage, not an adopted production resource bound. The
  full method and incomplete-D3 guard remain unchanged. Its terminal evidence
  is `target/d3-reconcile/native-warm-full-20260914-75494-rm0eb0`; the final
  report has 9,852 checks, `completed=false`, `passed=false`, and exactly one
  failure: `ingestion complete conditional D3 proof`. Within this run,
  `ingestion-native-settings/run-20260914-75494-rw9feh` passes twelve pairs and
  45 checks, and `ingestion-native-warm-helpers/run-20260914-75494-npk6u1`
  passes four pairs and 73 checks. Both have passing independent cleanup;
  the full replay also removed its container and host data. The scoped
  [callsite map](support/569-d3-closure-map.md#rows-1-and-5-source-bindings)
  identifies existing source-to-report bindings, not D3 acceptance. Finish the
  bounded callsite/skip-condition closure against these witnesses before
  replacing the completeness guard; add a witness only for a concrete gap.
- Risk/rollback: observer effects, insufficient coverage and incomplete
  reachable-callsite bindings remain explicit. Remove this observer integration
  if its evidence fails; retain failed runs and the incomplete-D3 guard.
  Warm callback counts are observations, not an invented completeness oracle.
- Dependencies/policy: existing standard-library Ruby, shared pinned observer
  and database only. Reviewed root and scoped DevOps native-observation rules;
  tightened the latter without relaxing criteria. One worker owned exactly the
  validator/tests, its integrated files were compared byte-for-byte, and its
  completed worktree was removed. No architecture decision or upload occurred.

### Native Settings Integration (2026-09-14, In Progress)

- Motivation/design: bind native compiler-scope observations to the existing
  success/rollback, late year-validation and regex-error cases. Keep the
  independent NOTICE matrix and exact application/read-image oracles; compare
  plain and native-observed clones in all twelve cold/helper-first variant
  contexts. No production SQL, frozen migration or bootstrap change.
- Tests: focused phase mutations pass 1,248 assertions; capture routing,
  protocol rejection and cleanup pass 21. Focused live qualification passes
  all twelve native pairs and 45 producer checks, alongside the unchanged
  NOTICE matrix, on the dirty delta over `d09daa34`. `just ci` passes for this
  delta; the complete final-proof replay has not yet qualified it.
- Retained evidence: `ingestion-native-settings/run-20260914-63676-bdt9wu`
  under the ignored final-proof output contains the successful run. The earlier
  `run-20260914-60030-24z121` failed because the new harness wrongly expected no
  callbacks before the year guard. Frozen source inserts the source row at
  line 512 before the year guard at line 946 and policy call at line 1278;
  correcting that expectation preserves, rather than suppresses, the observed
  callbacks. Both runs have successful independent cleanup receipts.
- CI follow-up: `target/revaer-host-backed-ci-1d436ce7bb1e/gate.log`
  stopped on the newly published `RUSTSEC-2026-0285`, not on this proof's
  assertions. The audit-loaded RustSec advisory requires Rustls >=0.23.45 for
  TLS 1.3 handshake encryption-level separation. Updated only Rustls
  0.23.35 -> 0.23.45 and required WebPKI 0.103.13 -> 0.103.15 in `Cargo.lock`;
  retained the existing TLS features, provider and unrelated resolved edges.
  Both patched crates require Rust 1.71, below workspace Rust 1.96. `just audit`
  and `just --command cargo check --locked -p revaer-app --all-features` pass.
  The full CI rerun passes; its log and owned-database diagnostics are retained
  in `target/revaer-host-backed-ci-09650752812b`. This dependency qualification is not an independent
  malicious-handshake reproduction or a claim of completed release validation.
- Coverage/UI: preserved the CI coverage reports and source hashes in
  `target/d3-reconcile/native-settings-coverage-09650752812b.json` before UI
  execution. All eight changed Ruby files have positive executed line coverage;
  the new phase validator covers 64/67 executable lines, but the integration
  driver covers only 17/87 in this coverage run. The separate focused live run
  is not merged into these coverage counts. Retained input mtimes distinguish
  fresh Rust/script reports from older JS/native compile-database inputs; this
  is not proof of a complete fresh Sonar submission or published coverage.
  The current `just ui-e2e` run in
  `target/revaer-host-backed-ui-e2e-ecae35e52e3d` has 57 passed, one failed and
  72 unrun: `tests/specs/api/media.spec.ts:484` receives HTTP 400 with
  `media_profile_filesystem_identity_required` instead of 200. Teardown also
  rejects missing UI coverage because UI tests did not execute. The initial
  sandbox-denied launch started no database; the authorized rerun removed its
  owned database, anonymous volumes, host data and private media root. No gate
  or API expectation was relaxed. UI, Sonar and release acceptance remain open.
- Observability: private raw native records, same-clone catalog/source binding,
  before/after source and target fingerprints, complete application evidence
  and a separate owned-resource cleanup receipt. No production telemetry.
- Risk/rollback: observer effects and incomplete scope remain explicit risks.
  Revert only this proof integration if necessary; retain failed evidence and
  the D3 guard. These observations do not prove every callsite or FK count.
- Dependencies/policy: existing Ruby standard library and pinned disposable
  observer only. Reviewed root, Rust, data and DevOps instructions; updated
  DevOps for this scope without relaxing any gate. The legacy scheduled-profile
  failure still requires ADR 557's coordinated root/profile replacement, not
  weakening the frozen filesystem-identity rejection.

### Canonical Native Cancellation Integration (2026-09-14, In Progress)

- Motivation/design: integrate the qualified cancellation snapshot and exact
  plain/observed comparison into the existing canonical Rust cancellation
  cases. The cold and committed-warm reference/final cases now each run as a
  pair; this is not an additional competing application matrix. Both arms
  retain owned warm-up/recovery locks and directly witnessed transaction
  clocks. Only the observed arm attaches the read-only debugger during the
  actual source-INSERT wait. Calls and memory/register writes are disabled;
  exact setting/backend/database readback, empty diagnostics, detach and the
  unchanged owned wait are mandatory before the original cancellation signal.
- Test coverage: the focused live integration passes all four pairs, including
  unchanged rollback/read inputs, same-pool recovery and the exact frozen D4
  warm-recovery failure. The comparison port passes 2,324 synthetic assertions
  across all four cases; snapshot tests retain 122 rejection cases. The capture
  suite passes 73 assertions covering transport, clock provenance, target
  identity, detach/wait drift and cleanup. The final-source focused repeat at
  `run-20260914-96248-jt6lga` passes all 16 producer checks and cleanup.
- Combined gates: `just ci` passes at `revaer-host-backed-ci-1f76550129ff`,
  including executed positive coverage for all fourteen changed Ruby files.
  Rust, JavaScript, generic and native coverage/compiler inputs were retained
  before UI execution. No new Sonar analysis was uploaded; scoped upload
  enforcement and scanner authentication remain unresolved. `just ui-e2e`
  remains red at `revaer-host-backed-ui-e2e-88a2d71b7aa8`: 57 pass, one fails,
  72 are unrun, and UI coverage is absent. Scheduled-profile setup retains the
  real `media_profile_filesystem_identity_required` rejection. Owned databases,
  host directories and UI media were removed; port 62241 was independently
  verified free, and `just clean-test-fixtures` passes.
- Complete-proof status: the first run passed 9,561 checks, including the four
  native cancellation pairs, before PostgreSQL closed its connection with
  `57P02` during temporal qualification. Cleanup succeeded. Its failed report,
  query diagnostic, full log and database-evidence archive are retained. The
  server log was not captured and the bounded Docker event query returned no
  events, so the cause is unproven. A serial replay of unchanged
  `FinalProof#run!`, through the Just command surface and injected runner,
  finishes with 9,733 passing checks and only the unchanged incomplete-D3
  guard failing. Its native cancellation group (`run-20260914-1132-l1cbl8`)
  passes all four pairs and 16 checks; cancellation, K1, policy and FK each
  retain successful cleanup. Server logs/inspection are retained before
  cleanup and show this replay's server running without a recorded OOM kill.
  This does not establish the first failure's cause. No gate, source
  comparison, role, timeout or server argument changed. The failing D3
  completeness guard remains enabled; no complete D3 acceptance follows.
- Documentation: all 1,415 links pass. The docs build exits zero with its
  retained large-search-index warning; it is not warning-free evidence.
- Provenance: tested source is `0a56763c` plus this integration delta. The
  comparison's semantics are unchanged; its tests no longer need an ignored
  live evidence archive. The FK expectation pin advances only for the reviewed
  `ingestion_proof.rb` report-pointer addition. No fixture, routine body,
  frozen/final SQL, predicted FK count or criterion changes with that pin.
- Observability: retain the original application reports, independently bound
  clock witnesses and complete pair comparisons; raw debugger stdout/stderr,
  exact commands, before/after backend identities, post-detach waits, source
  fingerprints, native target identity and separate owned-resource cleanup.
- Risk/rollback: remove cancellation observation and its comparison together,
  retaining the original Rust cancellation controller/oracles. The shared
  debugger creation extraction keeps existing trace transport unchanged and
  requires its regression suites. Incomplete D3 remains a required failure.
- Dependency rationale: Ruby standard library and existing pinned PostgreSQL/
  debugger tooling only; no added dependency or architectural decision.
- Stale-policy check: root, data and DevOps instructions reviewed. DevOps now
  records this exact paired observation boundary. Existing cancellation,
  scanner, bootstrap-authority and release requirements are not relaxed.

### Canonical Native FK Integration (2026-09-14, In Progress)

- Motivation/design: adopt the qualified cold/logger-first, external-ID and
  existing-v2 wrapper FK observations in the canonical proof. Reuse the four
  unchanged application scenarios, independent full-state oracles and owned
  native transport. Fixture setup stays outside capture. Source-derived counts
  retain exact caller ownership, compiler settings and frozen D5 failures.
- Test coverage: the reviewed phase port passes 120 synthetic assertions;
  the independent expectation provider passes 151, including target-body,
  signature, setting, missing/duplicate inventory and source-drift rejection.
  The first focused live run passes all eight plain/observed contexts and five
  targeted constraints. Follow-up capture hardening requires each raw trigger
  record immediately after its matching callback entry, as in the qualified
  readback. Capture tests now pass 35 assertions, including that mutation.
  The unchanged eight-context repeat at `run-20260914-3032-fg4m8k` passes all
  115 producer checks and its separate cleanup receipt. The current-source
  full proof repeats that result at `run-20260914-6479-kb6pl1`, alongside
  passing current-source K1 and policy groups; all three cleanup receipts pass.
  Its 9,722 passing checks and one failing incomplete-D3 guard are not complete
  D3 acceptance. Tested source is `04c3990ed9e89c5a0089e22dc68c32b9ca3d97f9`
  plus this integration delta.
- Combined gates: `just ci` passes at `revaer-volume-ci-4bbf4ff195b3`, including
  the final 35-assertion capture suite and positive executed line coverage for
  all fourteen changed Ruby files. Full Rust/JavaScript/generic coverage and
  native compiler input are retained before UI execution. This is local
  coverage, not published Sonar evidence; no new Sonar analysis was submitted.
  Scanner authentication and upload enforcement remain unresolved, and fresh
  confirmation of the configured project upload scope was requested.
  `just ui-e2e` still fails at `revaer-host-backed-ui-e2e-a92a5f6a12cc`: 57 pass,
  one fails and 72 are unrun. Scheduled-profile setup returns 400 with
  `media_profile_filesystem_identity_required`; UI coverage is absent.
  Documentation links pass 1,413/1,413; the build retains its large-search-index
  warning. Instruction drift and whitespace checks pass. No warning is waived.
- Test infrastructure: the initial host-mounted CI database failed startup
  because PostgreSQL rejected its data-directory ownership. That terminal
  failure and cleanup are retained. The unchanged volume-mode retry passes.
  An isolated child-PGDATA probe then succeeds with the same pinned image;
  the UI harness adds only that owned child-directory setting. Readiness,
  credentials, loopback confinement and test gates are unchanged. Owned
  databases, volumes, host directories and UI media were removed, and port
  63804 is free. `just clean-test-fixtures` passes. This does not alter
  production storage, package support or an architectural choice.
- Integration provenance: the expectation provider's `ingestion_proof.rb` pin
  advances only for the reviewed two-line FK registration/result-pointer delta.
  Frozen SQL, final SQL, application fixtures, target-body pins and all other
  source pins remain unchanged. Original worker bytes and qualified oracle
  projections are retained; no count was inferred from the new native run.
- Observability: retain paired full application/read images, raw ordered native
  records, exact live catalogs and routine sources, target ABI/native identity,
  source fingerprints and independent owned-resource cleanup receipts.
- Risk/rollback: remove the FK registration, capture and consumers together if
  integration fails. The shared strict collector still rejects cross-protocol
  observations; K1 and policy require current-source regression checks. Keep
  the incomplete-D3 guard and frozen bootstrap authority unchanged.
- Dependencies: standard Ruby JSON and digest support, plus the already pinned
  PostgreSQL/debugger tooling. No new package dependency or architecture.
- Stale-policy check: root, data and DevOps instructions reviewed. DevOps now
  records the bounded FK capture/source-binding duties. D3, cutover, operator
  workflow, UI, published Sonar, package and PR acceptance remain incomplete.

### Canonical Native Policy Integration (2026-09-14, In Progress)

- Motivation/design: adopt the already qualified twenty-context native policy
  matrix in the canonical proof. Retain the unchanged five policy fixtures and
  full plain/observed application oracles in cold/helper-first reference/final
  sessions. Eight frozen inlined-cast contexts pair actual completed query
  plans with native executor-entry compiler settings; the other twelve retain
  the qualified helper/regex phase validator. No application SQL is changed.
- Test coverage: the ported validators pass 766 synthetic assertions, including
  514 rejected mutations across all twenty contexts. Capture tests pass 33
  assertions; shared session tests pass 48. The focused live integration at
  `run-20260914-15239-sansxf` passes all twenty contexts and 304 checks, with
  unchanged source/target identities and successful owned-resource cleanup.
  The complete canonical run repeats the policy result at
  `run-20260914-35155-5dvrm7`, alongside successful K1 readback at
  `run-20260914-35155-agsxxg`. Both cleanup receipts pass. The aggregate records
  9,607 passing checks and one failing incomplete-D3 guard; it is not complete
  D3 acceptance. Tested source is `7f344b82402b99d5da3b846b62265b82eb3c2198`
  plus this native-policy integration delta. Synthetic contexts are not live
  evidence, and the supplemental run does not replace the canonical run.
- Combined gates: `just ci` passes in the owned host-backed environment
  `revaer-host-backed-ci-7d4fd12e95f8`. Its later script-coverage execution
  includes the final 33-assertion capture suite; all thirteen changed Ruby
  files have positive executed line coverage. This is retained local coverage,
  not published Sonar evidence. `just ui-e2e` remains red at
  `revaer-host-backed-ui-e2e-d0c9b7998301`: 57 passed, one failed and 72 unrun.
  Scheduled-profile setup returns `media_profile_filesystem_identity_required`
  (400 rather than 200); UI coverage is absent. The unfinished operator
  association/root workflow remains required. Owned databases, volumes and the
  private media root were removed, and `just clean-test-fixtures` passes.
  Documentation links pass 1,412/1,412 after the restricted-network attempt
  failed to reach external links; the unchanged network-enabled retry passes.
  The documentation build exits zero but retains its large-search-index
  warning. No gate or warning is waived.
- Integration corrections: the first capture enabled regex probes outside the
  qualified method's error cases and was rejected. The original per-case probe
  selection is restored; the broader failed trace remains retained, not filtered
  into a pass. A second capture passed twelve contexts before rejecting the
  legitimate absence of FK callbacks in early regex-error paths. Those paths
  now explicitly require absence; success paths and K1 still require callbacks.
  Tests reject both missing required callbacks and unexpected error-path
  callbacks. Snapshot source binding independently checks the canonical body
  rather than trusting self-consistent altered captured SQL and catalog hashes.
- Sonar: all thirteen changed Ruby files have non-skipped full-file MAIN
  analyses with zero reported issues; a transient profile-retrieval HTTP 500
  passed on the same-file retry. The canonical scan still cannot start without
  `SONAR_TOKEN`. CLI secrets analysis could not access its credential store in
  the sandbox, and execution policy rejected the escalated upload even after
  the existing ADR 588 transfer approval was cited. Fresh confirmation of the
  exact changed-file/project scope was requested; no upload workaround or
  criteria change is authorized by that failure. Secrets checks, positive
  published coverage and the full project gate remain unverified.
- Observability: retain private raw debugger and server streams, exact catalog
  and routine-source bindings, producer/target hashes, ABI headers, observer
  identity, complete application records and separate cleanup receipts. Plan
  logging is enabled only on the selected owned observed clones and reset
  afterward. Source warnings and missing evidence remain failures.
- Risk/rollback: remove the native policy registration and matching capture,
  validators and tests together if qualification fails. Shared transport keeps
  K1's strict protocol; policy records cannot be accepted as trust-rank records.
  Frozen migration authority, final SQL and the incomplete-D3 gate are unchanged.
- Dependencies: standard Ruby libraries, the pinned PostgreSQL auto_explain
  module and the already qualified debugger/tooling; no new package dependency.
- Stale-policy check: root, data, DevOps and Sonar instructions reviewed. DevOps
  now states canonical policy pairing, inlined execution and evidence duties.
  Existing approval applies to proof refinement, not a new architectural choice
  or completion claim. Remaining logger/settings/callsite obligations stay open.

### Canonical Native K1 Integration (2026-09-14, In Progress)

- Motivation/design: integrate the qualified K1 observer and independent readback
  into the canonical final proof instead of relying on archived experiments.
  One owned session layer attaches the pinned debugger to the actual disposable
  backend; the existing attribute fixtures and full application/input oracles
  remain unchanged. Two zero-rank cases retain cold/helper-first reference/final
  pairs, with separate nonzero rank-40 calibration.
- Test coverage: session/dispatch/catalog tests pass 46 assertions; independent
  readback tests pass 305 assertions including 271 rejected mutations. The live
  integration at `run-20260914-35549-8v7nu4` passes eight paired contexts, 24 calls,
  384 source-bound events, both nonzero calibrations and cleanup. The corrected
  full canonical run repeats that result at `run-20260914-46391-ezct1r`, with
  unchanged source/target identities and successful cleanup. Its 9,303 passing
  checks and one failing incomplete-D3 guard are not complete D3 acceptance.
  Tested source was `7ebe6a6302dfb2cb5a1ee8304eee0098e36dc49f` plus the recorded
  delta committed as `7f344b82402b99d5da3b846b62265b82eb3c2198`, with per-file
  producer hashes, on the pinned arm64 PostgreSQL
  observer environment; these are not amd64 or release-package results.
- Integration fixes: the first live run rejected GDB's source-newer-than-binary
  warning. Extraction now preserves the verified archive's file timestamps;
  tooling tests pass 332 assertions, including a historical-timestamp regression.
  The initial full proof stopped after 8,800 passing checks when registered
  `UniqueObject` evidence reached the successful-control normalizer. The retained
  records reproduce the duplicate-field exception. Copying the validated frame
  to an ordinary hash fixes normalization without mutating original evidence or
  changing the strict parser. Validation tests pass 1,434 assertions; the exact
  registered reference/final records now compare unchanged. A superseded rerun
  was interrupted and cleaned up before restarting with the correction.
- Handoff gates: full `just ci` passed, including executed coverage for all 14
  changed Ruby files and both corrected regressions. `just ui-e2e` remains failed
  (57 passed, one failed, 72 unrun): scheduled profiles require the unfinished
  filesystem-identity association, and UI coverage is incomplete. All 1,411
  documentation links pass; the HTML build retains its large-search-index
  warning. Fourteen full-file Ruby MAIN-scope analyses report no issues, but
  canonical `just sonar-scan` cannot start without `SONAR_TOKEN`; positive
  published coverage and the complete project gate remain unverified. No check
  was waived. Test media and owned database resources were removed.
- Observability: retain private raw process streams, debugger script and command,
  current producer hashes, catalog bindings, source-derived local reads, setup
  receipt, before/after source and target identity, and separate cleanup results.
- Risk/rollback: remove the K1 registration, session/readback integration and
  matching policy tests together if qualification fails. The frozen migration
  authority, final SQL bytes, incomplete-D3 guard and release criteria remain
  unchanged. K1 cannot certify other native callsites or either Linux package.
- Dependencies: reuse the already qualified debugger/image inputs, standard Ruby
  libraries and existing proof helpers; no application dependency is added.
- Stale-policy check: root, data and DevOps instructions reviewed. DevOps now
  records canonical K1 pairing, calibration, provenance and cleanup requirements.
  No operator approval is inferred and no criterion is relaxed.

### Shared Native Observer Tooling (2026-09-14, In Progress)

- Motivation/design: move process transport and observer preparation out of
  one-off capture setup into reusable, policy-tested components. `NativeProcesses`
  retains exclusive private stdout/stderr, drains both streams, bounds readiness
  and completion, and propagates stream failures while reaping later clients.
  Its caller still owns container termination; timeout alone is not termination.
- Dependencies: `NativeTooling` uses Ruby's existing standard libraries and
  RubyGems archive reader, not a new application dependency. The test-only GDB,
  debug-symbol and transitive package pins, full package-metadata digest, binary
  hashes and matching musl source are explicit in `.github/build-inputs.env`.
  Cached or locally built observers must extend the exact PostgreSQL base and
  pass every identity check. No registry publication or runtime package change.
- Verification: the registered component suites pass 76 process assertions and
  66 tooling tests with 331 assertions. Current local covered/executable lines
  are 120/122 and 229/237 for the components, and 148/151 and 310/313 for their
  authored tests. These are focused local metrics, not published project coverage.
  The combined ingestion suite and policy gate pass. Full `just ci` passes,
  including all 18 Rust package thresholds and the release build. Its canonical
  script-coverage XML includes all four new files with positive coverage:
  120/122, 229/237, 146/151 and 310/313 covered/executable lines respectively.
  `just ui-e2e` records 57 passes, one scheduling failure and 72 unrun tests;
  missing UI coverage files also fail teardown. The unchanged scheduling failure
  is `media_profile_filesystem_identity_required`, not a test waiver.
- Live setup: cached qualification and a real fresh build pass. The latter
  returns immutable image `sha256:618f6d3357404d809f999baf6f8615bc36ad848771be2b50b8b90b0f5157ba35`;
  its complete package inventory and all binary/source hashes match. Every
  recorded probe container, owned build tag and temporary source workspace is
  removed. The test-loader fix explicitly loads RubyGems under `--disable-gems`.
  Original worker source, the cleanup-diagnostic case mismatch and failed
  APK-fetch builds remain retained, not acceptance evidence. The corrected
  build uses APK's install solver with the complete explicit pins.
- Evidence/observability: the staged delta from `50d7a8ac`, component coverage,
  file-analysis results, setup receipts and failed attempts are retained under
  `target/d3-reconcile/`. Final transport run
  `fk-compilation-run-20260914-41430-zr1e0a` passes all eight paired K1 application
  comparisons, retaining empty debugger stderr, unchanged source/native hashes
  and confirmed client/container/host-data cleanup. Cleanup exceptions now mark
  its report failed. This does not adopt the complete
  native observer/readback into `FinalProof` or discharge its completeness guard.
- Sonar: the five changed Ruby files receive MAIN-scope file analysis with zero
  findings. The exact CLI command
  `just --command sonar verify --file scripts/database_rebaseline/native_processes.rb --project VannaDii_Revaer`
  still returns the organization-entitlement HTTP 403; no full analysis or
  positive published coverage is claimed. The later ADR 588 transfer approval,
  verified against reviewed commit `9575c077`, permits this implementation scan;
  it does not authorize ignored experimental harness uploads or criteria changes.
- Full-gate logs: `target/revaer-host-backed-ci-d53dd846c7b0/` and
  `target/revaer-host-backed-ui-e2e-e0838ab232cf/`. Both owned databases and the
  private UI media root are removed; managed fixture cleanup passes. All 1,411
  documentation links and the changed-file secrets scan pass. The documentation
  build exits successfully but warns about its 15,848,668-byte search index;
  this remains open and is not a clean-build claim. The restored legacy debugger
  image is removed after the last transport run.
- Risk/rollback: remove both helpers, their test registration and observer-only
  pins together. No SQL, bootstrap selection, production observability or approval
  boundary changes. The observer remains arm64-only qualification, not evidence
  for both supported release architectures. The agent's completed worktree and
  branch are removed after its files are integrated and separately archived.
- Stale-policy check: root, data, DevOps and Sonar instructions reviewed. DevOps
  now records strict stream ownership, preparation identity and cleanup duties.
  No criteria or source-scope restriction is relaxed. The scheduling test remains
  intact: the legacy profile procedure rejects automation until the approved
  root/profile/discovery binding path exists. Next integrate the already-qualified
  native observation and readback into `FinalProof`; do not repeat settled cases
  merely to recount them. Full D3, cutover, operator workflow and release stay open.

## D1: Clarify The Extension Privilege Boundary

**Recommendation, not authorization:** retain the pinned, stock `pgcrypto` and
`unaccent` objects in `public`, including their inherited PUBLIC EXECUTE ACLs,
as PostgreSQL extension primitives. Require owner-owned SECURITY DEFINER and
explicit-only runtime grants for every **authored Revaer routine granted to
runtime**. Keep trigger routines and the baseline seal ungranted. Runtime
still receives no application table/sequence access, DDL, extension management,
role administration, or baseline-seal permission. This does not promise that
runtime SQL cannot invoke hashing, cryptography, randomness, or text-search
primitives supplied by PostgreSQL and these two extensions.

The exact clarification and proof replacement requested are:

- Narrow ADR 551's "every runtime-executable routine" acceptance clause to
  authored, directly granted non-trigger routines, with the stock extension
  members independently constrained below. Its authored PUBLIC-revocation and
  explicit application-grant rules remain unchanged. This is the exact broader
  wording D1 supersedes, not an assertion that ADR 551 already made this choice.
- Replace only the new `no effective extension routine grants` assertion with
  an exact pinned-extension inventory/definition/ownership/ACL proof. The
  observed inventory has 40 routine identities, all SECURITY INVOKER; distinguish
  ordinary callable functions from internal callbacks. Derive and compare the
  complete inventory from the pinned clean fixture, not a permissive name glob.
- Preserve every authored-routine grant, search-path, relation-access, owner,
  role-substitution, baseline, and extension-DDL denial assertion. No additional
  PUBLIC grant or SECURITY DEFINER extension member is permitted.
- Approval covers only these stock extension objects under ADR 551's exact
  PostgreSQL image/version. It expires for any image, PostgreSQL minor version,
  extension version, member definition, owner, ACL, or extension-set change;
  renewed explicit review and proof are then mandatory. It authorizes no Sonar,
  advisory, coverage, GitHub-check, or other quality-criterion relaxation.

If the operator instead requires zero inherited extension EXECUTE ACLs or zero
extension-mediated computation, reject D1. A separately approved provisioning
or dependency design is then necessary; neither a private schema nor omission
from an explicit grant list establishes that stronger guarantee.

### Withdrawn Private-Schema Recommendation

The first local draft recommended `revaer_extensions` with no runtime schema
USAGE. Peer review found that this was not sufficient for its promised owner-only
dependency access. Numeric `regdictionary` input avoids name lookup, and
`ts_lexize` dispatches the dictionary callback by OID. This conclusion is derived
from the pinned 16.14 [OID conversion](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/adt/regproc.c),
[dictionary dispatch](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/tsearch/dict.c),
and [callback initialization](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/cache/ts_cache.c)
sources; it is not a live reproduction. Fastpath function calls themselves do
check both namespace USAGE and function EXECUTE in the pinned
[fastpath implementation](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/tcop/fastpath.c).
Existing prepared lookups are another limitation of post-hoc schema revocation,
as documented under [schema USAGE](https://www.postgresql.org/docs/16/ddl-priv.html).
No private schema was implemented. It is no longer the recommendation.

Alternatives considered:

- Retain stock extension primitives while strictly protecting authored Revaer
  state and procedures: recommended above, with its explicit capability limit.
- Give bootstrap superuser or ownership of extension member objects: materially
  widens authority and may violate extension safety assumptions; not recommended.
- Add a privileged pre-provisioning or ACL-management step: could enforce literal
  member ACL revocation, but changes pristine admission, installation topology,
  and transaction authority. It needs its own exact operator-approved design;
  no such step is included in this proposal.

## D2: Function-Scoped Reset Lock Bound

**Recommendation, not authorization:** attach `SET lock_timeout = '5s'` to
`revaer_config.factory_reset_without_media_defaults_v1` and remove its redundant
transaction-local `set_config` call in the final init only. This preserves the
existing five-second bound within reset while restoring the caller's exact
prior value on routine exit. PostgreSQL documents this
[function-local configuration scope](https://www.postgresql.org/docs/16/sql-createfunction.html).

The requested supersession is narrow: ADR 551's legacy-definition parity may
include this exact routine configuration/body delta, and its prohibition on
later timeout changes permits this tighter function-scoped lock bound. The
initializer still owns `120s/120s/30s`; no top-level reset, larger timeout,
automatic retry, deadline restart, or post-call repair statement is permitted.
Runtime reset calls also stop leaking their five-second setting into subsequent
statements. That observable scope change is explicitly part of D2.

Removing the five-second bound entirely would change contention behavior;
restoring 120 seconds with a later top-level statement would conceal leakage
and violate caller ownership. Neither alternative is included. The frozen
migration corpus remains unchanged under every option.

## D3: Function-Local Variable Resolution

**Recommendation, not authorization:** permit only the exact
`public.search_result_ingest_v1` substitution from function configuration
`SET "plpgsql.variable_conflict" TO 'use_column'` to an initial
`#variable_conflict use_column` compiler directive, conditional on proving the
complete search-ingestion call paths under the constrained roles. Grant no
superuser or parameter-setting privilege. Change no frozen migration, other
routine body, or ambient/session setting.

This expressly requests an additional exception to ADR 551's legacy-definition
parity constraint. It is not one of its specified SECURITY DEFINER, ownership,
or search-path changes, and G1 cannot supply consent. ADR 566 originally claimed
equivalence; that claim is withdrawn. `final_sql.rb` performs the substitution
and `final_proof.rb` applies it to the comparison reference. That equality check
cannot independently prove either authorization or behavioral equivalence.

PostgreSQL's [PL/pgSQL compilation documentation](https://www.postgresql.org/docs/16/plpgsql-implementation.html)
distinguishes a GUC affecting subsequent compilations from a directive affecting
only its containing function. A helper or trigger first compiled inside the
old function can therefore have a different ambient setting. This is a
source-supported risk, not a reproduced application regression. The current
focused application-path proof does not exercise search ingestion.

Approval must be followed by pinned 16.14 cold-session and warm-cache evidence
for the actual ingestion branches and every reachable helper/trigger, including
ambiguous names and dynamic calls. Compare results, mutations, errors, and
before/during/after setting scope against the frozen reference. If equivalence
cannot be established for application behavior, stop and present the exact
additional semantic delta; do not silently annotate other routines or broaden
privileges. Rejecting D3 leaves the local candidate uncertified and unpublishable.
The exception covers only this routine under the pinned PostgreSQL image and
validated helper/trigger closure; it expires if their definitions, compilation
context, or pinned PostgreSQL identity change. Renewed review is required then.
No further implementation or activation of this substitution is authorized now.

## Acceptance Evidence Required After Approval

- Fresh init under the pinned constrained owner; direct owner/runtime/outsider
  sessions; no role substitution; no added role or server capability.
- For D1, prove the exact stock extension inventory and permissions, no broader
  extension or authored capability, all existing application-state/DDL denials,
  and successful application paths before and after owner NOLOGIN. Retain
  schema/seed parity and complete transaction rollback proof.
- For D2, prove exact before/inside/after values for successful reset, lock
  contention, SQL failure, cancellation, transaction rollback, and nested calls.
  Preserve the fixed whole-operation deadline and final cancellation reason.
- For D3, independently establish the complete cold/warm ingestion call-path
  evidence above. A normalized-reference comparison alone is insufficient;
  restrict any parity exception to the single exact approved substitution.
- Mutation tests must reject changed extension membership/definitions/ACLs,
  authored PUBLIC execution, broadened search paths, timeout leakage, and removal
  or increase of the scoped bound. Existing failing proof assertions remain until
  the operator accepts the exact replacement contract and tests.
- Run full application parity, `just ci`, `just ui-e2e`, positive published
  Sonar coverage and applicable GitHub checks on the resulting stack revisions.
  Freeze/embed no final digest and enable no cutover before all required proof.

## Approved D1/D2 Implementation Checkpoint

The approved final SQL changes only D2's exact function-scoped reset timeout:
one `SET lock_timeout TO '5s'` function option replaces the transaction-local
body call. The regenerated local candidate digest is
`1a9f0f2e9b4ca04ee880baa1babc4f105d2726ee5b4a4fe627f04fcc1a06beb9`.
This is an evidence identity, not a certified or activated baseline. The
167-file frozen migration corpus and runtime bootstrap remain unchanged.

The finalizer now applies D2/D3 transformations only inside the exact named
SQL statement, and rejects missing, repeated or unexpected routine envelopes.
The privilege proof compares the pinned clean extension fixture with the
candidate, including all extension memberships, the 40 routine definitions,
owners, effective ACLs, the two internal callback signatures, and dictionary
and template definitions. Eleven deliberately changed extension states must
be detected, and each mutation must roll back to the exact prior inventory.
All previous authored-privilege and application-state denial checks remain.

`just db-init-final-proof` passed 105 live checks on PostgreSQL 16.14, including
real owner/runtime resets, exact before/inside/after timeout values, nested
reset scope, SQL failure, externally delivered cancellation, independently
held-lock contention, transaction rollback, and application calls after the
bootstrap owner is NOLOGIN. The first harness run exposed its SQLSTATE-only
notice formatter and an invalid self-cancellation assumption across the
SECURITY DEFINER boundary; corrected test observation uses full retained
notices and an exact-backend signal from the disposable test administrator.
No production role was elevated. A no-op dictionary mutation was replaced by
an actual dictionary-definition mutation before the successful rerun.

`just --command bash scripts/tests/database-final-test.sh` passed 18 exact-byte
assertions, including rejection of removed/increased reset bounds and restored
timeout leakage. `just instruction-drift` passed. The 105-check result does not
include or discharge D3's independent ingestion closure proof. Full integrated
CI/UI, published Sonar and GitHub checks are not established by this checkpoint.

## Independent D3 Counterexample

The integrated proof on the same `1a9f0f2e...6beb9` final digest now executes
123 assertions: 122 pass and `ingestion warm-committed required outcome` fails.
All 105 preceding assertions pass again. The original frozen function retains
its function GUC in the independent reference; it is not rewritten to D3's
directive for these behavioral comparisons. Six cold cases match observed
results, errors, eighteen-table state and caller settings. A second ingestion
after commit on the same backend fails in both reference and candidate with
`42P07: relation "tmp_policy_rules" already exists`.

This shared legacy failure neither demonstrates a D3-only regression nor
fulfills the required usable warm path. The proof retains raw observations and
explicitly reports `complete=false` and `passed=false`. Its 44 harness assertions
pass. Independent review found and corrected unconditional rollback of a
successful second call and insufficient control-error validation. Three live
transaction controls prove persistence of successful second writes, visibility
of differing writes and rollback of the expected division-by-zero failure.
Controls require exact SQLSTATE sequences and diagnostics, rejecting warnings.
The final source re-review found no further issues in that correction; it is
not a second independent live rerun.
Complete branch/helper closure and in-call compiler-setting observations
remain outstanding. [ADR 579](579-ingestion-policy-temporary-table-lifetime.md)
presents D4's exact additional temporary-table lifetime delta for approval;
conditional D3 does not authorize that change. No D4 SQL is implemented, no
frozen migration is edited, and the final init remains inert and unpublishable.

## Approved Correction Comparison Checkpoint (2026-09-11)

This continues D3 verification after the [ADR 588 approval](588-first-release-decision-package.md#approval-resolution).
It changes the disposable proof only, not final SQL, migrations, privileges,
runtime bootstrap or the conditional D3 acceptance boundary.

- Motivation: Raw parity rejects the two separately approved D4/D5 fixes. The
  proof must recognize their exact effects without rewriting the reference or
  treating its failed calls as successful application behavior.
- Design notes: Recognize only `warm-committed` and `existing-external-id`.
  Retain `equivalent=false` and record the named approval separately. Pin the
  original SQLSTATE, statement digest, routine, line and native location;
  require exact successful results and all 18 final table images, including
  unchanged rows, generated identity links and transaction provenance. The
  D4/D5 correction matrix remains mandatory before these comparisons.
- Test coverage summary: On parent `b5c860eb` plus the retained proof delta,
  `just db-init-final-proof` passes 371 of 372 checks using pinned PostgreSQL
  16.14. All 21 current ingestion comparisons meet their required outcomes;
  19 retain parity and two explicitly validate the approved correction. The
  remaining failure is the complete conditional D3 scope gate, not a pass.
  The 27-case, 220-check independent correction matrix passes again. Harness
  tests pass 316 assertions, including 82 new checks rejecting extra table
  writes, changed diagnostics/settings/results, lost counterexamples and
  unrelated case names. Exact finalizer guards pass 40 assertions.
  Full `just ci` exits zero, including all 18 package coverage gates, script
  coverage and release build, but retains eight config-watcher shutdown WARNs;
  this is not warning-free release acceptance. Rust LCOV contains 94,524
  covered records of 101,615. All five changed Ruby files have positive generic
  coverage. Full serial `just ui-e2e` still fails profile creation (201 expected,
  400 received): 46 passed, one failed, 61 not run, plus missing job-phase and
  profile-readiness route coverage. Earlier UI attempts stopped at an occupied
  port and concurrent asset generation; logs are retained. No assertion changed.
- Sonar: Whole-file Ruby MAIN analysis through `analyze_code_snippet` reports
  zero issues for all five changed Ruby files in `VannaDii_Revaer`.
  `just --command sonar analyze secrets <changed-files>` passes after retrying
  outside the keychain/cache sandbox. This supplements, not replaces, the
  canonical scanner and positive published coverage, which remain outstanding.
  Documentation links pass 1,333 checks; instruction drift and diff checks pass.
- Observability updates: Retain both raw variants and the explicit approval
  field. `complete=false` and `passed=false` remain until full D3 proof exists.
  No production telemetry changes.
- Risk and rollback plan: An overly broad exception could hide a regression.
  Exact fixture-specific predicates and mutation tests limit this risk; remove
  only this proof delta to restore strict raw equality. Never modify the frozen
  reference or enable cutover to make the proof pass.
- Dependency rationale: No new dependency; use existing Ruby standard library,
  SQL statement parser, pinned PostgreSQL and canonical Just commands.
- Stale-policy check: Reviewed root, data, DevOps and Sonar instructions and
  ADRs 569/579/583/588. DevOps now describes the precise approved comparison
  without weakening full D3 verification. Earlier counterexample records
  are historical; this checkpoint does not claim clean full CI/UI, canonical
  Sonar, package qualification, publication or merge.
- Remaining work: The bounded closure audit identifies mutating logger-first
  calls, populated policy matchers and casts, in-call setting observations,
  and discriminating warm wrapper/scoring/paging paths as the next executable
  batch. Its catalog inventory is not behavioral proof or D3 completion.
- Evidence and cleanup: Exact tested source hashes, patch, all proof JSON,
  gate logs, local coverage and the closure audit are retained under
  `artifacts/media-verification/2026-09-11-d3-adjudication/`. UI teardown removed
  its last task database and media; the canonical fixture cleanup passes.
  No production media, frozen migration, user checkout or remote PR changed.

## In-Call Setting Proof Checkpoint (2026-09-11)

- Scope: Proof-only continuation from `62c5d6a7`; no final SQL, frozen
  migration, privilege, runtime or acceptance-criteria change.
- Motivation and design: Exercise the real conflict logger before ingestion
  on a cold tested connection, including NULL fallback and 256-character
  truncation, then reuse that connection across commits. Also exercise cold
  canonical insertion, nested conflict logging and a persisted policy flag
  through the real decision cast. Disposable triggers observe session/current
  role, backend, transaction and ambient setting at three write boundaries.
- Verification: Four cases, two database variants and paired plain/observed
  runs pass 140 checks. All 18 application tables compare per operation;
  instrumentation must preserve behavior. Validate D4's exact temporary-table
  lifetime separately before normalizing that metadata. Generated identities
  and named transaction columns normalize only after validation; arbitrary
  UUID-shaped text and other timestamps remain visible. Harness tests pass
  515 assertions. Full `just db-init-final-proof` passes 511/512 checks; its
  sole failure remains the incomplete-D3 gate. These observations cover
  committed successful writes, not failed-call or complete trigger closure.
- Counterevidence retained: The first draft miscounted the two distinct hash
  conflict records and used `decision_type` instead of the persisted `decision`
  column. The harness was corrected against the unchanged routine/schema;
  neither failed assertion nor initial evidence was hidden.
- Observability: Raw queries, diagnostics, full images, observer events and
  real Ruby coverage are retained. No production telemetry changes.
- Risk and rollback: Observers could affect compilation or writes; paired
  uninstrumented runs and exact state comparisons guard that risk. Remove
  only this proof delta to roll back. Never use it to enable cutover early.
- Dependencies: Existing Ruby standard library and pinned PostgreSQL only.
- Stale-policy check: Reviewed root, data, DevOps and Sonar instructions and
  ADRs 569/588. DevOps records the bounded evidence contract. Historical
  checkpoints remain historical, not fresh release or approval claims.
- Remaining work: Populated policy matchers, discriminating warm wrapper and
  paging branches, remaining errors/mutations, and native/trigger closure.
- Full gates on the proof-only tree: `just ci` exits zero with all 18 package
  coverage gates and the release build, but retains eight config-watcher
  shutdown WARNs. Rust LCOV records 94,557 covered lines of 101,615 records.
  The merged real script coverage includes 173/174 lines in the new compilation
  module and 107/107 in its tests. `just ui-e2e` again reports 46 passed, one
  failed and 61 not run: profile creation returns 400 instead of 201, and
  teardown rejects missing job-phase/readiness GET coverage. The initial CI
  instruction-drift failure and UI database-ownership setup failure are retained;
  the reruns use the matching instruction update and documented caller-managed
  disposable database option, not reduced tests or relaxed assertions.
- Sonar and review: All five changed Ruby files return zero issues from
  whole-file MAIN analysis after a retained DNS-failure retry. Secrets analysis
  passes. `just sonar-scan` stops because `SONAR_TOKEN` is unavailable; no
  canonical scan or published-coverage pass is claimed. Independent review
  found no actionable issue and rejected 52 additional evidence mutations;
  it replayed retained results, not a second live PostgreSQL run. Finalizer
  guards pass 40 assertions and documentation links pass 1,333 checks.
- Evidence: Source hashes, exact dirty delta, initial failures, final raw proof,
  gate logs, local coverage and review are retained in
  `artifacts/media-verification/2026-09-11-d3-compilation/`. No runtime SQL or
  frozen migration changed. C1 was implemented independently at `81947a65`;
  this proof-only CI/UI run does not qualify the combined integration, packages,
  publication, merge or complete service.

## Wrapper And Warm-Cache Proof Checkpoint (2026-09-11)

- Scope: Continued D3 proof from integration `18357155`, with no final SQL,
  frozen migration, runtime, authority, dependency or criterion change.
- Motivation and design: Add the actual Rust-called `search_result_ingest`
  wrapper to the exact routine inventory. Compare its first visible-source
  tail, competing stored scores and seeder thresholds, a full ten-item page
  and its successor, canonical reuse without page mutation, the third size
  sample, and retention of the latest 25 samples after the 26th observation.
  Fixture rows come from successful ingestion on separate recorded backends;
  explicit base scores are pinned read inputs, not fabricated procedure results.
- Warm-cache boundary: Each of eight cases runs cold, after pure-helper first
  use, and after a real same-backend ingestion rollback followed by a committed
  retry. Rollback is observed through full table images; no DROP, DISCARD,
  reconnect, role substitution or compiler-setting change repairs the tested
  session. This is warm-after-rollback evidence, not successful frozen
  committed reuse. The separate exact D4/D5 correction matrix remains mandatory.
- Validation: All 48 database-variant runs and 24 paired comparisons pass
  456 checks. The actual canonical `just db-init-final-proof` passes 967/968;
  only complete conditional D3 remains unproven. All 18 write tables compare
  per operation. Six read-input relations are retained and immutable across
  tested calls; only explicitly named input timestamps equal to the recorded
  seed transaction may normalize. Other input values and unobserved timestamps
  remain visible. This does not claim a complete read/native dependency closure.
- Harness and analysis: 619 assertions pass, including deliberate mutations
  of result identities/flags, every write-table image, stored scores, title
  selection, page seal/position, retained samples and input clock provenance.
  Finalizer mutation guards pass 40 assertions; instruction drift and whitespace
  checks pass. All four changed Ruby files return zero issues from full-file
  MAIN Sonar analysis; both review-corrected files were rescanned with zero
  issues. `just script-coverage` passes. Merging its executed records with the
  instrumented canonical proof records gives wrapper coverage of 139/139 lines,
  wrapper-test coverage of 127/127, proof-owner coverage of 211/217 and
  proof-harness coverage of 195/196. This is local measured coverage, not
  canonical Sonar or positive published coverage proof.
- Independent review found that the initial highest-score fixture also won
  the lowest-ID tie-break, so wrong local-variable ordering could remain
  invisible. Reversed the real fixture insertion order and retained a negative
  control: the high-score source has the larger ID; wrong local ordering
  selects the already-100-seeder low-score source and incorrectly leaves the
  previous best unchanged. Initial reports and source are preserved under
  `artifacts/media-verification/2026-09-11-d3-wrapper-initial/`; their numerical
  pass counts alone do not establish this corrected binding proof. The full
  corrected canonical run again passes 967/968 checks, with only the incomplete
  D3 sentinel failing. Independent re-review replayed all 48 runs and 400 raw
  frames, verified all eight decisive calls reject the faulty ordering, and
  found no remaining issue in that scope.
- Scanner boundary: The exact-file Sonar secret scan passed. The authoritative
  `just sonar-scan` exited before analysis because `SONAR_TOKEN` is unavailable;
  no canonical quality-gate or published-coverage pass is claimed.
- Observability: Retain raw queries/results/diagnostics, all transaction and
  input images, source identity and check reports. No runtime telemetry changes.
- Risk and rollback: Test setup or normalization could hide a real difference;
  separate connections, unchanged inputs, exact result predicates and mutation
  tests constrain that risk. Remove only this proof delta to roll back; retain
  the conditional D3 gate and never cut over from this subset.
- Stale-policy check: Reviewed root, scoped data/DevOps/Sonar instructions,
  ADRs 569/588 and the existing proof owners. The DevOps update makes the
  warm-rollback and named-clock limitations explicit; no approval is inferred.
- Remaining: Complete populated policy/error/identity branches and native,
  trigger and read-dependency closure. Full integrated CI/UI, canonical Sonar,
  both native packages and merge acceptance remain required.

## Policy, Identity And Native Proof Checkpoint (2026-09-11)

- Scope: Continue the approved D3 proof without changing frozen migrations,
  final SQL, runtime authority or acceptance criteria. Policy commit `a3657b3b`
  was replayed byte-for-byte as `ff7a681e`; native inventory `dc004eb9` was
  replayed byte-for-byte as `6b335753`. Parent wiring invokes both through the
  existing canonical proof, alongside the new identity matrix.
- Policy evidence: 49 populated cases and 196 variant runs pass 1,472 checks.
  Retain all 18 write tables and 19 read inputs, exact decisions and actual
  `action::decision_type` execution. Exercise require families, precedence,
  matchers, actions, severities and prefer branches. Cold/helper-first and
  warm-rollback evidence remains distinct from approved committed D4 reuse.
  The frozen release-group regex requires a literal-backslash suffix in this
  fixture: ordinary suffix recognition and policy-creation API conformance
  are not proved by these cases. No regex repair is included.
- Identity evidence: Add new/reuse/GUID-promotion cases for six identity
  strategies, with distinct lower-ID decoys and competing v1/v2 identities.
  The full wrapper matrix now has 26 cases, 156 runs and 1,482 passing checks.
  Independent replay validates 724 frames, including 324 identity frames,
  with 3,024 additional assertions and 4,768 targeted mutation rejections.
  Canonical/source/observation identities, refresh data, separate hash vectors,
  unchanged unrelated rows and wrapper best-source selection are explicit.
- Review corrections: Initial validators omitted some observation/refresh
  assertions and skipped the common wrapper tail. Discriminating fixtures
  replaced single-row identity cases. The first corrected live run then caught
  a wrong test expectation: v2 reuse preserves canonical v1 absence but fills
  source v1 from the supplied hash. The SQL was not changed to satisfy it.
  A subsequent coherent wrong-selection mutation exposed result-anchored ID
  selection. The final predicate independently selects the prior fixture
  observation and binds all three expected IDs to it. Twelve modeled mutations
  and 16 raw mutated frames pass the old predicate and fail the new one.
  Independent re-review reports no remaining finding in that bounded scope.
- Native evidence: The worker's pinned PostgreSQL 16.14 run passes 823 checks
  with exact container/volume cleanup. It compares recursive catalog edges,
  definitions/settings, read-relation dispatch, FK equality and trigger
  bindings, actual policy cast, unaccent dictionary/template, and native file
  identities. Real known answers run cold and across commits. Installed
  callbacks and eligible row changes are explicitly not callback-entry traces;
  `pg_depend` is not complete PL/pgSQL late-binding analysis. This run predates
  the parent identity extension and is not combined-proof acceptance.
- Harness: The integrated proof harness passes 1,868 assertions; the new
  dependency harness passes 104. The last pre-native canonical run passes
  3,465/3,466 checks, failing only the explicit incomplete-D3 sentinel. Its SQL
  generation is unchanged by the final identity-predicate fix, which was
  independently replayed against the retained raw evidence. The newly wired
  canonical run on `6b335753` plus this staged proof delta passes 3,469/3,470:
  its only failure is incomplete D3. All four dependency checks pass, tied to
  166 current-process observation files. Finalizer guards pass 40 assertions.
- Native review status: The 3,469/3,470 run predates three required validator
  corrections. Coherent root/callback omissions could pass both catalogs;
  check labels did not bind later-consumed frame bytes; equal invalid backend
  IDs could satisfy warm provenance. The retained data itself replayed
  consistently. Correction `7f7b91c9`, replayed as `4981eef4`, plus the parent
  validation-time byte-hash producers closes those three findings. A fresh
  canonical run again passes 3,469/3,470, failing only incomplete D3. Independent
  replay passes 556 assertions, verifies all 166 file hashes and rejects the
  exact unchanged-707-root callback omission, empty/duplicate frames and 14
  invalid-backend mutations. Another 24 assertions exercise the actual producer
  methods with mocked transport and tampered writes; no observation registry
  was reconstructed from disk. The corrected native unit suite passes 197.
- Analysis and gates: Full-file MAIN Ruby Sonar returns zero issues for the
  final identity module/test, proof owner/harness and native module/test.
  The integrated CI attempt exits zero, including all 18 package coverage gates
  and the release build, but retains eight watcher shutdown warnings and the
  fixture suite's explicit ignored test. Rust LCOV contains 94,718 covered
  records of 101,711. Source changed during that run; it is not a clean final
  revision certification. UI stops at missing C1 startup metadata before any
  browser cases. ADR 586 already permits an explicit test-only injected loader;
  wiring that path is approved implementation, not a new architecture request.
  Canonical Sonar cannot start without `SONAR_TOKEN`; no published coverage,
  full UI, Linux package, remote-check or merge pass is claimed.
  Documentation generation succeeds; `just docs-build` exits zero but emits
  a large-search-index warning (14,668,403 bytes). This is not warning-free
  documentation qualification, and no search or analysis scope was reduced.
- Latest integrated gates: On `4981eef4` plus the retained parent delta, with
  executable source unchanged throughout the run, `just ci` exits zero through
  all 18 package coverage gates, script coverage and the release build. Eight
  watcher shutdown warnings remain, so this is not warning-free qualification.
  Its owned database and anonymous volumes were removed. Full UI now reaches
  57 passed, one failed and 72 not run through ADR 586's explicit test bootstrap;
  legacy schedule enablement and missing UI coverage remain blocking. Full-file
  MAIN Sonar reports zero issues for both corrected native files and both
  producer files; canonical `just sonar-scan` still fails before analysis for
  missing `SONAR_TOKEN`. The later child-coverage fix is tracked in ADR 586 and
  is not retroactively certified by this CI run.
- Observability and evidence: Preserve raw queries, diagnostics, images,
  source boundaries, local execution coverage and review under
  `artifacts/media-verification/2026-09-11-d3-identity-initial/`,
  `2026-09-11-d3-identity-review-intermediate/`,
  `2026-09-11-d3-integration-pre-native/` and `2026-09-11-d3-native/`.
  The native worker did not retain two superseded development failures; their
  final successful replacements do not reconstruct those missing logs.
  No production telemetry or original media changed.
- Risk/rollback and dependencies: Test oracles and normalization can hide real
  differences, so retain deliberate negative controls, separate role/session
  evidence and independent review. Revert only these proof changes to roll
  back; retain the D3 gate and inert init. Use existing Ruby standard libraries,
  PostgreSQL tools and pinned container only; no new dependency is introduced.
- Stale-policy check: Reviewed root, data, DevOps, UI and Sonar instructions,
  ADRs 569/586/588 and the closure audit. DevOps now requires fixture-bound
  expected identities and the exact limits of catalog/native evidence. No
  approval, condition, scanner scope or required check is weakened.
- Remaining: Validation/error, identity-fill/conflict, attribute/signal and
  reachability families plus complete helper/native dispatch qualifications.
  Full stable-source CI/UI, canonical Sonar, both packages and merge remain
  required. Every report retains `d3_complete=false`; no cutover is authorized.

## Task Record

- Motivation: Present the two live single-init conflicts and the subsequently
  identified unapproved parity exception without inventing architectural
  consent or treating a normalized-reference comparison as behavioral proof.
- Design notes: The original proposal identifies predecessor constraints,
  alternatives and retained boundaries. The approved D1/D2 implementation is
  recorded in the checkpoint above. D3 remains conditional on independent
  semantic proof; its local candidate must not be published or activated yet.
- Test coverage summary: ADR 566's original 62/64 result is historical. The
  approved D1/D2 checkpoint and subsequent 122/123 integrated result are
  recorded above; the new D3 warm failure is retained, not accepted as a pass.
- Observability updates: Retain the two named failures and bounded evidence;
  no runtime telemetry, error contract, or credential handling changes.
- Status-doc validation: Reviewed the completion goal and ADRs 522, 541, 551,
  and 559. Single-init remains inert; E1 and other retained holds remain held.
- Risk & rollback plan: D1 explicitly permits stock extension computation and
  requires exact inventory proof; D2 changes nested reset timing. Both are
  implemented in the inert local finalization only. D3 remains held pending
  proof, not a second approval of the same conditional substitution. Reverting
  this task's implementation restores the previous local candidate without
  touching deployed state or GitHub; never repair a sealed baseline in place.
- Dependency rationale: No new dependency proposed. Keep the existing pinned
  PostgreSQL tools, extensions, SQLx transport, and canonical recipe surface.
- Stale-policy check: Reviewed root, data, Rust, devops, and Sonar instructions.
  Tightened data instructions to prohibit publication or activation of the
  uncertified D3 candidate and to distinguish reference normalization from proof.
  No accepted ADR, required check, quality threshold, or frozen migration is
  changed by this proposal. Indexes and generated docs are updated.

## Validation Guard Checkpoint (2026-09-11)

- Motivation: Add the bounded validation/error family from closure-audit case 6
  without changing the frozen ingestion routine or discharging D3.
- Design notes: Worker `13ee74af`, replayed as `021656b4`, adds 108 cases at
  32 independently fixed RAISE coordinates. Cases cover request/indexer
  eligibility, title/hash/identity guards, companion arrays, duplicate keys,
  value channels/types and numeric/text domain failures. Four legitimate
  NULL/empty controls remain accepted. Cold and helper-first sessions repeat
  errors after savepoint rollback and commit on the same backend. Successful
  controls retain first-call rollback, second-call commit and the exact D4
  frozen third-call error versus final success.
- Evidence integrity: Retain exact diagnostics, roles/GUC/backend/clock frames,
  unchanged read inputs and all 18 table images. Register original serialized
  bytes only after validation; missing hashes, changed bytes and framing or
  state mutations remain errors. Sequence rollback and complete helper/native
  closure are not claimed.
- Test coverage summary: Worker units pass 1,426 assertions. Its fresh live
  wrapper passes 872 checks, including 649 validation-matrix checks across
  216 paired cases. All nine recorded source hashes match the committed files;
  exact container and named-volume cleanup succeeds. Earlier failed attempts
  remain in the archived worker evidence. Parent units pass 1,430 assertions
  after four added report-classification checks; the existing harness's 1,868
  and native inventory's 197 assertions also pass.
- Parent correction: The worker labelled successful D4 controls equivalent
  after comparing only their first two calls. Raw third-call differences were
  retained, but that label was misleading. The parent now records
  `equivalent=false`, explicit acceptance and the named approved delta. The
  archived worker result predates this reporting correction. Canonical proof
  and policy-harness wiring are present; fresh integrated evidence and
  independent review are still pending at this checkpoint.
- Observability updates: No production telemetry changes. Preserve source,
  initial/final evidence and review under
  `artifacts/media-verification/2026-09-11-d3-validation/`.
- Risk and rollback: A bounded matrix cannot certify all ingestion behavior.
  Keep `d3_complete=false`, the canonical incomplete-proof failure and frozen
  migration authority. Reverting this proof increment cannot permit cutover.
- Dependency rationale: Existing Ruby, PostgreSQL transport and proof helpers
  only; no runtime dependency, schema, permission or package change.
- Stale-policy check: Reviewed root, DevOps, Sonar and the ADR 569/588 approved
  boundary. DevOps now requires these exact guard/control distinctions and
  canonical wiring. No acceptance criterion or prior hold is relaxed.

### Integrated Validation Result

The fresh canonical proof on `021656b4` plus the parent wiring/report correction
passes 4,118 of 4,119 checks. Its only failure is the required incomplete-D3
sentinel. The new matrix completes all 649 checks: 216 accepted pairs, with
208 equivalent guard cases and eight explicitly non-equivalent approved D4
controls. Its nine declared source hashes still match. This is not a successful
final-init qualification or permission to replace the frozen migrations.

Independent review retains 4,637 assertions, 692 rejected coherent mutations,
eight mocked producer controls and byte-hash readback of 3,030 raw files. It
also checks the parent's report correction independently; no residual actionable
finding remains in this bounded review. The reviewer did not recreate a worker
registry from old files or certify the new canonical run. Reproducible review
code, results, raw inputs and exact limits are retained under
`artifacts/media-verification/2026-09-11-d3-validation-review/`.
The completed worker worktree and reviewer have been removed/closed. Full-file
MAIN Ruby analysis reports zero issues for the new module and its tests;
the two canonical wiring owners also report zero MAIN Ruby issues.

Final `just ci` on the same stable executable source exits zero on 2026-09-12,
including all 18 unchanged package coverage gates, script coverage and the
release build. Documentation alone changed during execution. Eight
config-watcher shutdown WARNs remain; this is not a clean handoff. The exact
owned database `revaer-approved-ci-13d9879c395b` and its anonymous volumes were
removed. Raw coverage and the complete log are retained with the proof evidence.
The fixed 52-file secrets scan ran successfully; canonical `just sonar-scan`
stopped before analysis because `SONAR_TOKEN` was unavailable. Full UI, positive
published Sonar coverage, both native packages and D3 qualification remain open.

## Settings-Path Proof (2026-09-12)

The next bounded increment observes successful rollback/retry, late validation
failure and nested policy-regex failure in cold and helper-first sessions.
Test-only NOTICE records survive rollback; paired uninstrumented runs check
that observers preserve application outcomes. Exact frozen insertion/stack
coordinates, original diagnostics, caller settings after transaction completion,
all 18 table images and the 19 read inputs remain required. The existing
protocol is unchanged unless its explicit finish-setting observation is enabled.

Focused units pass 281 assertions. The fresh isolated run passes 266 checks,
including 43 settings-path checks, and removes its exact container and named
volume. Its source is `af67cfce` plus the five proof/test changes, before later
canonical wiring. The initial missing-notice-context attempt remains retained;
the corrected client requests complete context rather than discarding it.
Independent review and canonical integrated validation remain in progress.

No production telemetry, dependency, schema or frozen-authority change is made.
Rollback removes only this proof increment; the incomplete-D3 failure remains.
Root, data, DevOps and Sonar instructions were reviewed; DevOps now specifies
the exact trace boundary. Neither these tests nor approval imply full D3,
clean CI/UI, package, published Sonar or merge qualification.

Independent review found three validator gaps: NOTICE/error ordering was lost,
matching empty input snapshots passed, and the regex error was not restricted
to the selected title callsite. The parent added combined-stream validation,
independent seed/clock checks and the exact title/helper coordinates, with
regressions. The initial 281-assertion/live result is not evidence that those
gaps were closed; corrected units pass 309 assertions. The superseded canonical
run was interrupted with
exit 130, its owned container was verified absent, and its log was retained.

The corrected disposable run on `52c7ef13` plus the owned settings/wiring delta
passes all 266 checks and removes its exact container and named volume. All 12
declared source hashes match the parent. Independent recheck passes 194 review
and 309 unit assertions, rejects the ten unchanged original witnesses and 16
fresh coherent mutations, and finds no residual issue in the three-finding
scope. Its isolated checkout differs only by six canonical wiring lines in
one declared file; this is retained replay, not a second live integrated run.
All 230 original review artifacts remain unchanged. Exact source selections,
raw output, failed attempts, commands and manifests are archived under
`artifacts/media-verification/2026-09-12-d3-setting-paths/`.

The exact ten-file secrets scan passes. Full-file Ruby MAIN analysis of the
two corrected files and both canonical wiring owners reports zero issues for
`VannaDii_Revaer`. These auxiliary results do not establish canonical scanner,
quality-gate or positive published-coverage acceptance. The combined canonical
result below exercises the integrated wiring on stable executable source.

## Attribute And Signal Proof (2026-09-12)

Worker `90a3c37b`, replayed as `52c7ef13`, adds 12 cases and 48 cold/helper-first
variant runs. It checks every valid non-D5 typed attribute key, normalized
language/subtitles, year/season/episode, release-group acceptance/rejection,
trust ranks 19/20/30/40, real first signals and repeated ingestion. Focused units
pass 206 assertions; the worker's pinned arm64 live driver passes 297 checks.
All 19 declared source hashes match its commit, and all three attempts' exact
containers and named volumes were verified absent. Raw attempts remain retained.

The frozen ordinary-suffix regex defect and duplicate signals caused by nullable
uniqueness remain explicit limitations, not accepted feature completion. No UUID
attribute key is invented; D5 ID cases retain their separate required proof.
Independent review replays all 48 variants and 192 frames, passes 245 review
and 206 unit assertions, rejects 26 mutations, and passes three producer
controls. No actionable finding remains within that scope. Repeated identical
attributes do not prove changed-value conflict updates or metadata conflicts.
Both full Ruby files report zero MAIN-scope Sonar issues. Exact raw attempts,
source, replay, commands and manifests are retained under
`artifacts/media-verification/2026-09-12-d3-attributes/`.

Canonical wiring is present and exercised by the combined result below. The two
completed review worktrees were removed after archival, and the reviewer was
closed. No new runtime, schema, telemetry or dependency change is made.
Rollback removes only the proof increment. Root, data, DevOps and Sonar policy
were reviewed; the DevOps proof contract was tightened without relaxing a gate.

### Combined Proof Result

On `52c7ef13` plus the owned settings and canonical-wiring delta,
`just db-init-final-proof` passes 4,234 of 4,235 checks and exits one. Its sole
failure is the explicit incomplete-D3 gate. All 43 settings-path and 73
attribute checks complete successfully; their 12 and 19 declared source hashes
still match the parent. No executable source changed during the run.

The exact PID-bound disposable container was removed and its absence verified;
the proof's cleanup completed without a recorded failure. Complete raw proof
evidence and the log are retained in the settings-path archive. This closes
neither unobserved identity-fill/conflict and reachability cases nor full
helper/native qualification. The frozen migrations remain authoritative.
Full clean CI/UI, canonical Sonar with positive published coverage, both native
packages and exact-revision PR acceptance remain separate required gates.

Full `just ci` on the same stable executable source exits zero with all 18
unchanged package gates, script coverage and the release build, but retains
eight config-watcher shutdown WARNs. Full `just ui-e2e` again reports 57 passed,
one failed and 72 not run: schedule enablement receives 400
`media_profile_filesystem_identity_required` instead of the required 200, and
teardown rejects missing UI coverage. Both exact gate databases and volumes
were removed. Canonical `just sonar-scan` stops before analysis for unavailable
`SONAR_TOKEN`; positive local coverage is not published Sonar acceptance.
The [coverage checkpoint](586-compliance-manifest-failure-boundary.md#settings-and-attribute-checkpoint-2026-09-12)
records the current inputs and limits. No gate, warning, approval or required
operator behavior was relaxed, and no PR was pushed or merged.

## Missing-Hash Proof (2026-09-12)

This continued D3 task adds six missing-source-hash cases: v1, v2 and explicit
magnet identities, each with an uncontested or competing GUID-less peer. The
target begins without hashes. Independent full-column models specify both
real fixture states and all 18 write tables after ingestion, including exact
identities, result flags, conflict/audit/health effects and sequence gaps after
whole-transaction rollback. A conflicting peer blocks the corresponding durable
hash fill while the incoming observation retains that identity. The derived
magnet can still fill in the v1/v2 cases; this is observed frozen behavior,
not permission to reinterpret source and observation state as equivalent.

The source is `a19e5856` plus the five-file proof/test delta. The fresh canonical
wrapper matrix passes all 1,945 checks, including 18 new paired cases in cold,
helper-first and same-backend rollback/retry modes. Those pairs contain 36
variant runs and 48 tested calls, separate from their 72 fixture calls. All
31 declared proof-source hashes remain unchanged. The earlier 12 cold probes
remain diagnostic history; replaying them with the new assertions is not
relabeled as a fresh database run or full D3 qualification.

The proof test owner passes 2,495 assertions, with the existing dependency,
validation, settings and attribute suites also passing. Full-file Ruby MAIN
analysis of all five changed files reports zero issues for `VannaDii_Revaer`;
the exact five-file CLI secrets analysis passes. These are auxiliary Sonar
results, not the authoritative repository scan or positive published coverage.
Independent review subsequently found two validator gaps: coherently empty or
changed read inputs could be accepted, and fixture results could contain extra
fields. The initial pass above does not close those findings. The correction
independently specifies all six seeded read tables, including column defaults
and empty base scores, requires exactly five fixture result fields, and adds
coherent mutations for every input column and result-schema drift. Corrected
focused suites pass 5,621 assertions, including 3,479 in the proof test owner.
Both corrected full-file Ruby MAIN analyses again report zero issues.

The full `just db-init-final-proof` invocation then exercises 4,698 checks:
4,697 pass and only `ingestion complete conditional D3 proof` fails. The report
retains `completed=false` and `passed=false`; this is not conformance acceptance.
The frozen candidate and final-init digests are unchanged. The owned
`revaer-final-proof-92212-ec72842e01928e87` container was removed with its
anonymous volume, and the exact-name absence check returned no container.
The complete invocation is retained in
`/private/tmp/revaer-approved-canonical-hash-fill.log`; its proof reports and raw
SQL/stdout/stderr remain under `target/database-rebaseline/` pending archival.

That original canonical evidence was archived before the corrected rerun, under
`artifacts/media-verification/2026-09-12-d3-hash-fill/canonical-proof-evidence.tar.gz`.
The independent review retains all 2,160 mutations, including the original 72
accepted invalid controls, and all seven simulated-transport producer controls.
Its proposed correction rejects every mutant; only the valid producer control
registers evidence. In-memory review is not fresh database execution: parent
revalidation of the applied correction and integrated gates remains in progress.
The first CI invocation was interrupted after the findings, exited 130, and
removed `revaer-approved-ci-279889b5ad8f` with its volume; exact-name absence was
verified. Its complete original log remains separate from the corrected gates.

The fresh corrected canonical invocation passes 4,733 of 4,734 checks and again
stops only at the explicit incomplete-D3 guard. Its wrapper matrix passes all
1,981 checks with unchanged declared source bytes. Independent recheck passes
480 checks across all 36 corrected variants and 18 pairs, rejects the original
malformed producer witnesses without registration, and retains the valid
registration control. All 348 earlier review artifacts remain unchanged; no
actionable finding remains in the bounded two-finding review scope. The corrected
`revaer-final-proof-41476-b2544c088a52a19a` container and its anonymous volume were
removed, and its exact-name absence check passed. The corrected full log is
`/private/tmp/revaer-approved-canonical-hash-fill-corrected.log`.

No frozen SQL, runtime behavior, telemetry or dependency changes. Rollback
removes only this proof increment. The root, Rust, data, DevOps and Sonar
instructions were reviewed; the DevOps proof contract was tightened. Stale Rust
guidance permitting credential defaults was replaced with a reference to the
already enforced explicit disposable-database input contract. Remaining metadata
conflict/interleaving and helper/native conditions keep D3 incomplete. Accepted
architecture still does not authorize premature init cutover or weakened gates.

The reviewed proof and instruction correction are committed as `9c861f2b`.
The [hash-fill gate checkpoint](586-compliance-manifest-failure-boundary.md#hash-fill-gate-checkpoint-2026-09-12)
records the fresh full CI/UI results on that clean source: CI exits zero with
eight watcher WARNs; UI fails at schedule enablement and rejects missing UI
coverage. Canonical Sonar cannot run without its token. The complete corrected
proof, independent recheck and gate evidence are archived under
`artifacts/media-verification/2026-09-12-d3-hash-fill/`; the earlier pending-run
statements above remain dated history, not the current result. No remote or
release acceptance is established, and remaining D3 work continues separately.

## Changed Metadata Proof (2026-09-12)

This continued D3 task covers three finite serial cases: replacing all 16
non-D5 typed observation values, adding eight previously absent attributes,
and appending conflicts beside existing conflicts with a stale observation
and a 512-character tracker. Independent models specify all 18 write tables,
19 read tables and defaults, and six affected sequence counters. Each case
runs cold and helper-first against reference and final schemas, with a real
fixture backend, whole rollback, same-backend retry and committed reuse.

The author commit `a6fb4bcc` passed 12,851 assertions and 12 live variants
with 243 checks, including 19 metadata checks. Independent review then found
two false acceptances: duplicate JSON fields discarded before validation and
coherently impossible clocks accepted through evidence registration. These
findings invalidate any claim that the original validator closed those cases.
The two-file correction `0caaf40c` rejects all 36 original actionable witnesses;
12,927 assertions and 37 producer controls pass using retained transports.
That replay is not fresh PostgreSQL execution or independent approval of the
correction. Parent review identified compatibility with older JSON runtimes;
follow-up `9d055ac7` combines the standard object hook and native rejection,
with a fail-closed behavioral check. It passes 12,931 focused assertions and
rejects all 36 original witnesses on retained transports. Decoder-only controls
pass on Ruby 2.6.10 / JSON 2.1.0 and Ruby 4.0.6 / JSON 2.18.0; they do not certify
the full suite on older Ruby. Parent inspected both patches. The integrated
metadata and wrapper source lists include the new guard; fresh gates remain
pending, and the earlier broad producer matrix was not repeated unnecessarily.

The metadata owner retains unique raw SQL/stdout/stderr directories and only
registers successfully validated bytes. The original findings and later
authored corrections remain separate evidence. The author's first failed
live run lost its shared correction-output bytes to a retry; surviving shared
bytes prove only that later run. This provenance gap cannot be reconstructed
or relabeled. Parent source fingerprints now also include the loaded ASSET-1
guard; historical counts and hashes remain historical.

Frozen ingestion only appends conflict, audit and health rows; conflict
resolution/reopen updates are not in this call path. A valid serial non-NULL
durable attribute cannot enter the guarded conflict-update branch. Neither
observation proves concurrent behavior or complete helper/native closure.
The exact frozen third-call D4 failure remains visible, as do the literal
backslash suffix and NULL-distinct duplicate signals. D3 stays incomplete.

No SQL, init authority, runtime behavior or telemetry changed. Rollback removes
only this proof increment. Dependencies remain existing tools and Ruby standard
libraries. Root, data, Rust, DevOps and Sonar instructions and ADR 588 were
reviewed; no approval, criterion or threshold was weakened. Full clean CI/UI,
canonical Sonar with positive published coverage, native packages and current
PR acceptance remain separate requirements.

Fresh integrated verification on executable `5027eba5` and documentation-only
successor `7ee7df07`: 12,932 focused assertions pass; canonical proof passes
4,752/4,753 checks and fails only the explicit incomplete-D3 guard. All 19
metadata checks pass across 12 live variants with 32 source hashes; the wrapper
retains 33 source hashes. Full CI exits zero with eight watcher WARNs. Full
UI/API reports 57 passed, one failed and 72 not run at legacy schedule
enablement; teardown rejects missing UI coverage. Local first-party Rust LCOV
records 122,231 covered of 131,114 lines in 310 files, not published Sonar
acceptance. The canonical token remains absent, and the rejected eight-file
upload was not retried. No native package, PR or release acceptance is claimed.
The 25-file, eight-archive checkpoint, including 53 raw Rust inputs, is retained
under `artifacts/media-verification/2026-09-12-d3-metadata/`. Completed worker
worktrees, disposable gate databases/volumes and managed test media were removed.

## Mutating Helper Order (2026-09-12)

The next D3 increment runs the real conflict-writing helper before ingestion
in the same cold backend, with separate committed and whole-rollback cases.
It covers both 256-character truncations and the default observation clock;
independent models retain all 18 table images and the three consumed conflict,
audit and health sequences. Existing pure-helper orders and D4 failures remain.
Initial focused suites pass 13,474 assertions, including 456 additional
mutations rejected. Fresh database and combined gates are pending. This is
proof-only work with existing dependencies and no SQL/runtime/init delta;
rollback removes this increment. Root, data, Rust, DevOps and Sonar instructions
were reviewed; DevOps now requires the actual same-backend mutating controls.
No broader D3, helper/native closure, cutover or new approval is claimed.

The bounded PostgreSQL run at clean `119ec599` passes 261 checks, including
24 metadata variants across three cases and four compilation modes. The
12 paired outcomes retain the exact D4 exception; `d3_complete=false` remains.
Its original report, raw evidence and real Ruby coverage are archived under
`artifacts/media-verification/2026-09-12-helper-order/`; the exact disposable
container and named volume were removed and absence verified. Independent
two-file review reports no actionable finding after replaying the 24 variants;
its temporary worktree was removed. This is not a full-service or Sonar pass.

A further wrapper regression distinguishes two actual size samples (100, 900)
from their median (500): the canonical display must remain the first sample
(100) until the third sample. The existing cold, helper-first and same-backend
rollback/retry modes exercise it without changing stored procedures. Separate
checks reject wrong rollup counts, extrema, median and premature promotion.
This is the same proof-only implementation boundary with no new dependency;
DevOps now records the two-sample requirement. Full canonical and combined
gate results remain pending and no existing acceptance condition is removed.

## Size And Domain Boundaries (2026-09-12)

This proof-only increment adds 19 independent sampling decisions: below, at and
above the exact 10 TiB cutoff; requested versus effective domain; no, single and
multiple instance domains; each ebooks/audiobooks/software exception; zero/NULL;
and normal/large title fallback without sampling. Each runs through the real
application wrapper cold, helper-first and with whole rollback/retry. Expected
values cover all 18 write tables and eight unchanged read tables, including
seed clocks, public identities and sequence gaps. Direct fixture seeding proves
these ingestion states, not their reachability through request-creation APIs.

Focused suites pass 14,378 assertions before the current-run consumer update.
Mutation controls reject altered full-column images, partial rollback, wrong
domains, duplicate JSON, impossible clocks and float-valued identities. Wrapper
evidence now uses unique directories; the native-dependency consumer follows
the producing run and still requires its validated byte registry. PostgreSQL
and combined gates for this increment remain pending; D3 stays incomplete.

No stored procedure, init authority, runtime policy, telemetry or approval
changes. This follows the existing proof-owner decomposition using Ruby standard
libraries, with no new dependency. Rollback removes this increment only. Root,
data, Rust, DevOps and Sonar instructions were reviewed; DevOps now records the
boundary and evidence-retention requirements. No criteria were relaxed.

The first live run at `5662cc68` rejected the model's integer zero scores:
the frozen schema declares all three scores `NUMERIC(12,4)`, serialized as
decimals. Its interrupted original run, shared correction outputs and cleanup
receipts are retained before retry. The model now expects exactly decimal
zero while still rejecting float-valued identities. This corrects the model,
not the database or comparison strictness. The disposable container and named
volume were removed; this failed run is not passing evidence.

At integrated `ccf6f7ff`, the bounded live run passes 1,422 checks across 114
variants; the canonical run passes 6,024/6,025 checks, failing only the existing
incomplete-D3 guard. Its 156 paired wrapper cases pass 3,235 checks. Containers
and the bounded named volumes were independently confirmed absent. Full CI
exits zero with the existing watcher WARNs; this is not a clean handoff.

Post-run inspection found a narrower validator defect: Ruby numeric equality
accepted a raw domain ID of `1.0` alongside a decoded ID of `1`. The current
comparison now requires the same exact table encoding for raw and declared
read inputs, and explicit regressions retain that original false acceptance.
Database results and earlier evidence remain unchanged; final-source focused
and combined verification is pending. No SQL, runtime or criterion changed.

Final executable checkpoint `be1a2a53` passes 812 focused size assertions and
all 114 retained size variants under the stricter validator. That replay is
not fresh database execution. Its subsequent canonical run independently
passes 6,024/6,025 checks and again fails only incomplete D3; all 156 paired
wrapper cases pass. Full CI exits zero with eight existing watcher WARNs.
Full UI/API has 57 passed, one schedule-PATCH failure and 72 not run; teardown
also rejects missing UI coverage. Neither required handoff gate is clean.

Local Rust LCOV has 122,900 covered DA records of 131,765 in 311 tracked
first-party files. Seven other records are retained, not counted as authored
Rust. LF/LH summary fields differ from DA totals in 251 records; the original
export and a separate comparison are retained without rewriting either. These
are actual DA counts, not a published Sonar metric or scanner acceptance.
The canonical token remains absent and the rejected upload was not retried.

Original failures, both canonical runs, real coverage and raw instrumentation
are retained at `artifacts/media-verification/2026-09-12-size-config/` with the
source bundle and exact gate driver. Containers and the UI listener were
confirmed absent; managed test-media cleanup passed. Frozen migration authority,
init cutover conditions, package qualification and PR acceptance are unchanged.

### Prevent-Merge Reachability Increment (In Progress)

The next bounded D3 family exercises all three hash strategies and both
orientations of a persisted `prevent_merge` rule through the application wrapper.
Each case first creates and independently verifies a successful no-rule fixture,
then executes two failed calls on one backend, in cold and helper-first modes.
The independently specified outcome is the frozen unique-key error, not a
successful canonical split. Every affected table, all 19 read tables, rule
identity and clock, canonical identity sequence, caller settings and exact
PostgreSQL diagnostic remain checked. PostgreSQL 16.14's independently retrieved
`nbtinsert.c` pins `_bt_check_unique` to line 666 and SHA-256
`3babaf9404d5f93dc20a5aea3dfdfffb0be636bf9ab1794c2fbb3b0bc92fe79e`.

This adds proof only. It does not repair the legacy behavior, establish rule API
reachability, cover concurrent identity races, observe every sequence or close
native/helper execution scope. The conditional-D3 guard remains incomplete.
There is no dependency, runtime SQL, observability or acceptance-criteria change;
rollback removes this proof family without changing the frozen authority.
Reviewed root instructions, DevOps, ADR 569 and the existing proof owners; the
DevOps specialization now names this narrow evidence boundary. No completion
or additional operator approval is asserted.

Focused validation passes 235 assertions. The first real run rejected integer
trust-weight expectations: `0012` declares `NUMERIC(12,4)`, so the independent
shared read model now retains decimal weights. The original failed source and
database evidence were archived before retry; no SQL or comparison was relaxed.
The subsequent pinned live run passes all 24 variants, each with one successful
fixture and two exact `23505` failures, plus the mandatory D4/D5 controls.
Both disposable runs report resource cleanup. Canonical database verification
at `c9a25f31` passes 6,049/6,050 checks; only the unchanged incomplete-D3 guard
fails. All 37 recorded proof inputs match integrated `749ba0b4`. Its full CI
fails the native deadline/pipe-closure assertion and retains four watcher WARNs;
full UI remains 57 passed, one schedule-PATCH failure and 72 not run, with
missing UI coverage rejected. Rust coverage was not reached by this CI run.
See [combined gates](586-compliance-manifest-failure-boundary.md#combined-configuration-and-disambiguation-gates-2026-09-13)
for exact limits and retained evidence. No native deadline or test criterion was
changed. Frozen authority and conditional D3 remain in force.

## Native Callback Counter Limitation (2026-09-12)

Parent delegated one bounded function-counter investigation on `ac094da9`
under the existing isolated-investigation goal. This allocation was not a new
decision-specific human approval and did not authorize another profiler or
runtime design. PostgreSQL 16.14's
[function manager](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/fmgr/fmgr.c)
sets built-in/internal functions to the never-track threshold before callback
dispatch. The pinned
[statistics implementation](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/activity/pgstat_function.c)
therefore creates no counter for them, even with `track_functions=all`.
The exact low-level
[transaction counter API](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/utils/adt/pgstatfuncs.c)
returns NULL for a missing entry. Bypassing the user-function view cannot
make built-in RI callbacks observable, and NULL must not become zero calls.
The [trigger caller](https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/commands/trigger.c)
also updates usage after normal return, not as an error-surviving entry trace.

The dependency owner now rejects shared reference/final substitutions of RI
language, native symbol, arguments, result type, library, settings and security
mode. Its installed/eligible callback records still explicitly deny execution
proof. Focused unit coverage adds eight substitution rejections and an absent-
counter limitation check; the 208-assertion suite passes. No new dependency,
observer trigger, runtime telemetry, production/frozen/init SQL, normalization,
timer, permanent privilege or approval change is introduced. Rollback removes
this guard and limitation record only; the incomplete-D3 sentinel remains.

The retained reproduction is under
`artifacts/media-verification/2026-09-12-d3-callback-counter/`, including pinned
upstream source bytes, source deltas and individually numbered attempts. It
uses the existing uniquely owned network-none bootstrap, required D4/D5 matrix
and the wrapper producer's independently validated bytes and source hashes.
The exact frozen candidate is recovered from `7fd5df30` and verified against
the unchanged candidate pin before use. Original import, sandbox and probe-
framing failures are retained separately, including shared correction outputs
before retry; none is relabeled as native evidence.

Attempt 04 passes 306 bounded checks. The first-visible application wrapper
runs cold/helper-first, reference/final and counters disabled/enabled: eight
validated runs with complete 18-table images, actual results and unchanged
caller compilation settings. Every profiled C `digest(bytea,text)` control
increments by exactly one; all three sampled internal RI counters remain
NULL before and after ingestion. Four separate privileged, isolated inserts
into the real `canonical_size_sample` table retain SQLSTATE `23503`, the named
FK and native `ri_ReportViolation` diagnostic, unchanged table images and NULL
RI counters. These actual native failures are counterexamples to observability,
not numeric callback counts or evidence for other ingestion branches. No
test-authored trigger substitutes for RI. Both created containers and named
volumes were removed; successful removal and exact-name absence receipts are
retained. This is a precise negative result for this mechanism only.

Stale-policy check: root AGENTS, data and DevOps guidance, ADR 569 and the
operator's ADR 588 approval boundary were reviewed. DevOps now records this
counter limitation; no contradictory authority was removed or relaxed. Parent
owns integration, ledgers/indexes and full CI/UI. No canonical full proof,
full gates, alternative profiler, native closure or complete D3 is claimed.

## Committed Sampling Evidence (2026-09-13)

In progress under conditional D3: add three-call application-wrapper cases for
median promotion, duplicate sample suppression and out-of-order observations.
Both cold and pure-helper-first modes retain the same tested backend across
commits. The frozen reference must keep its exact approved D4 failure on calls
two and three; the final implementation must succeed. Independent expectations
cover all 18 write tables, unchanged 19 read inputs and the sample identity
sequence, including duplicate allocation. Other sequences and complete native
or warm-path closure are not certified. The incomplete-D3 guard remains.

Mutation tests reject substituted state, raw/declared evidence divergence,
caller role/settings/backend changes, invalid clocks, numeric type changes and
altered native diagnostics. The focused suite passes 200 assertions and the
complete database unit harness passes. The pinned live run passes 237 checks,
including required D4/D5 controls and all 12 committed sampling variants, with
exact source hashes and successful disposable container/volume cleanup.
Two test-harness failures are retained before correction: the diagnostic hash
needed its already-approved `SQL statement` representation, and synthetic
readback snapshots needed independent copies. Neither fix changes a criterion
or database implementation. Integrated gates remain pending. Evidence
uses uniquely owned disposable databases and retained raw transport; no runtime
observability, dependency, SQL, privilege, timer or approval boundary changes.
Rollback removes only the added proof module and tests. Root, data and DevOps
instructions plus ADR 588 approval were reviewed; DevOps records the bounded
committed-call requirement without weakening or contradicting existing policy.

The canonical proof at `f5981931` passes 6,062/6,063 checks. Only the unchanged
incomplete-D3 gate fails. All 38 sampling-proof source hashes also match the
combined shutdown revision `fe0948a4`; its full CI passes without WARN or
compiler-warning lines. UI still fails schedule enablement and missing UI
coverage. See the [combined checkpoint](586-compliance-manifest-failure-boundary.md#sampling-and-owned-watcher-integration-2026-09-13).
Frozen migrations remain runtime authority; final init remains inert.

## Magnet Normalization Branch Evidence (2026-09-13)

In progress under the existing conditional D3 approval. Three actual-wrapper
identity inputs cover an empty magnet query, discarded empty parameter keys,
and a bare parameter without a value. Each exercises new identity, reuse and
GUID promotion in cold, helper-first and same-backend warm-after-rollback
sessions. Expected normalized URI hashes come from independently specified
text; all prior result, identity, read-input and 18-table assertions remain.
Direct helper-first known answers retain all earlier inputs and add these
three; mutation checks reject every substituted normalization output.

This is proof-only work: no database SQL, runtime, privilege, dependency or
approval change. Observability consists of retained local SQL, diagnostics,
source hashes and disposable resource cleanup receipts. Live and integrated
validation are recorded below. These cases do not close concurrent GUID conflicts,
native trigger execution or D3 overall. Rollback removes this proof increment
only, retaining frozen authority and the incomplete-D3 failure. Root, data and
DevOps instructions and ADRs 569/588 were reviewed; DevOps now records this
bounded requirement without weakening existing criteria.

At `36abef44` plus this six-file executable proof/test delta, the focused
ingestion suite passes 4,746 assertions and every other database proof family
passes. The rebaseline unit harness passes 45 assertions. The pinned live
run passes 738 checks, including 27 reference/final pairs (54 new variants)
and the separate mandatory D4/D5 controls. All recorded source hashes still
match after execution. The uniquely owned container and named volume were
removed. No first-attempt database failure occurred in this increment.
Canonical proof, full CI/UI and published Sonar evidence remain pending.

The frozen executable checkpoint `e3fb95c6` subsequently passes 6,575/6,576
canonical checks; only incomplete D3 fails. All 183 wrapper pairs pass 3,748
checks. Full CI exits zero without WARN/compiler-warning lines. UI exits one:
57 passed, one failed, 72 not run; schedule enablement still returns 400 rather
than the required 200, and teardown rejects missing UI coverage. Owned gate
databases and volumes were removed. Canonical Sonar authentication remains
unavailable; no analysis, upload, remote pass or release acceptance is claimed.

## Bounded Native Callback Observation (2026-09-13)

One isolated worker at `36abef44` established stock PostgreSQL 16.14
`auto_explain` viability for the first-visible wrapper, with eight plain/profiled,
reference/final and cold/helper-first variants passing. Four profiled variants
each record 16 completed native RI insert callbacks. Parent independently
matched all 364 plan records and 64 callback entries to raw structured logs.
These observed counts are not independent count oracles or complete D3 evidence.
The initial observer-only 42501 is retained; correction moved administrator-only
setting reads to separately labeled bootstrap receipts, adding no runtime
privilege or role substitution. Original and corrected archives are retained.

This completes only the bounded mechanism experiment. Native update/delete,
error/skip paths, concurrent GUID conflicts and full helper closure remain;
the independently reviewed score argument still requires its exact binding
premises. No production/canonical observer, SQL, dependency, criterion or new
approval was adopted. Existing root/data/DevOps and ADR 569/588 scope apply.
The worker was closed and its clean worktree removed. Evidence is retained at
`artifacts/media-verification/2026-09-13-d3-closure/`. Rollback discards the
experiment only; frozen authority and the incomplete-D3 gate remain unchanged.

## Concurrent GUID Conflict Proof (2026-09-13)

In progress under conditional D3. Two controlled interleavings reach the
frozen ingestion conflict callsites at lines 1029 and 1046: another real wrapper
caller either claims the wanted GUID on a competing source, or changes the
selected source's GUID before its next read. Each runs cold/helper-first against
reference/final, on separate direct-role backends in disposable clones. The
only instrumentation is one exact administrator-installed advisory barrier;
original and instrumented definitions are retained. No production SQL changes.

The canonical harness now includes these eight variants and four comparisons.
It verifies all 18 committed table transitions, independently specified conflict,
audit and health records, all 19 independently specified read-input tables,
unchanged caller settings through commit, exact role capabilities and the
approved D4 scratch lifetime. Raw transport is reparsed before comparison.
Only the verified conflicting source UUID text is additionally normalized.
The legacy competing-GUID observation reuse remains visible; this does not
repair or newly approve that behavior. Controlled scheduling does not establish
uninstrumented timing, in-call setting scope or native/helper closure.

The initial local prototype passed 349 checks on clean `94c691fe`, with source
hashes and resource cleanup verified. Two live promotion attempts caught harness
errors: OID-valued lock tags are JSON strings, and the initial reader returned
only six wrapper inputs rather than the required 19-table policy inventory.
Both failures and cleanup receipts are retained. The corrected reader uses the
existing complete snapshot and independent seed oracle; it does not narrow
the expected inventory. Focused live proof passes all eight variants, four
comparisons and mandatory D4/D5 controls. The expanded unit suite passes 356
assertions, including coherent mutations of every read-input table. Integrated
gates are recorded below.

Reusing the earlier native archive also binds both actual score upsert/selection
statements to the public score table in four first-visible contexts. All eight
plans match their raw log records and show one inserted/selected row. This
supports those observed binding premises only, not universal operator/OID
binding or complete native closure. No profiler run or adoption was needed.

Test-only observability adds retained SQL, raw results, plans, source hashes and
cleanup receipts. No dependency, permission, deadline or approval change is
introduced. Rollback removes this proof increment only; frozen migrations and
the incomplete-D3 failure remain. Root/data/DevOps instructions and ADRs 569/588
were reviewed; DevOps now records this bounded proof without weakening policy.

An independent, single-task UI investigation confirmed that frozen
`media_profile_update_v1` rejects scheduling regardless of fixture identity.
No fixture-only change can preserve the positive assertions. The approved
root-persistence/API integration after cutover remains required. No code changed
in that investigation; its agent was closed and clean worktree removed.

On clean executable revision `7e9ab9cf`, the canonical proof passes 6,588/6,589
checks; only the unchanged incomplete-D3 gate fails. All four GUID comparisons
pass with exact source hashes. Full `just ci` exits zero with no WARN/compiler
warning lines and positive local Rust/script coverage. Full `just ui-e2e` exits
one: 57 passed, one failed, 72 not run, with the same scheduling 400 and missing
UI coverage. Owned databases and volumes were removed and test-media cleanup
passed. This is progress, not a completed handoff or cutover.

`just sonar-compile-db` passes and produces the native compilation database and
both bridge headers without uploading anything. `just sonar-verify-inputs`
correctly fails on absent `coverage/js-lcov.info`; no partial report is promoted
to replace it. Canonical authentication is also absent. No scan, published
coverage, GitHub check, push or package acceptance is claimed. Local source,
raw/merged coverage, failed and passing proof attempts, native binding readback
and gate logs are retained at
`artifacts/media-verification/2026-09-13-guid-conflicts/`.

### GUID In-Call And Logger-First Evidence (2026-09-13)

The next bounded increment pairs plain and NOTICE-observed runs for both GUID
conflict callsites, adding logger-first to cold/helper-first compilation. All
24 live variants pass, including six reference/final comparisons and mandatory
D4/D5 controls: 267 focused checks and 1,048 unit assertions. The focused run
names base `ef3b17ef`, its exact four-file dirty delta and source hashes; it is
not evidence for a clean committed revision until the integrated gates below.

Every logger write is bound to its exact SQL, native caller stack, direct role,
backend and transaction clock. All three ordered writes retain the frozen
in-call `use_column` setting versus final `error`, while caller settings remain
unchanged. Logger-first really writes conflict/audit/health rows, rolls back
all table effects, and reuses that helper in the same backend. Its sequence
gaps remain visible and independently checked. Plain/observed results, complete
18-table transitions and 19-table read inputs match. Test-only NOTICE observers
and scheduling barriers remain confined to disposable administrator-owned
clones; this is neither an uninstrumented timing guarantee nor native closure.

A separate bounded `trim-size-samples` experiment on clean `ef3b17ef` passes
eight plain/profiled reference/final cold/helper-first variants with 200 real
fixture wrapper calls. Each tested call updates seven rows, inserts one and
deletes one; sample 100 disappears and the retained median becomes 1400.
Both runs pass 600 checks including D4/D5; offline analysis passes 160. The
retained 284 profiled plans contain four completed RI insert callback records;
UPDATE/DELETE plans report no callback entries. These are observations, not
independent callback-count oracles or proof of unobserved paths. No profiler
is adopted. Initial permission/harness/packaging failures and successful
cleanup are retained in the hash-verified archive. The completed agent is
closed and its clean owned worktree is removed after retaining the archive.

Motivation is to close the bounded GUID helper-context gap without changing
runtime behavior. No dependency, production SQL, grant, deadline, cutover or
approval changes. Observability is test-only raw SQL/NOTICE/plan evidence.
Rollback removes only this proof increment. Root/data/DevOps instructions and
ADRs 569/588 were reviewed; DevOps records the new exact trace and rollback
requirements. No stale constraint was removed or relaxed. D3 remains
incomplete, and UI/root integration, published Sonar coverage, native packages
and actual PR acceptance remain outstanding.

The complete unit harness also passes, including the 4,746-assertion core
suite and all nested proof modules. Clean executable revision `ce971164`
passes 6,618/6,619 canonical checks; only the existing incomplete-D3 guard
fails. All six GUID comparisons and 43 GUID checks pass, with all 40 source
hashes matching the committed files. The disposable proof database is gone.

The first full CI run fails `deadline_includes_pipe_setup_time`: its expected
20ms deadline error also contains a process-group force-kill `EPERM` message.
The unchanged isolated test passes once and then all 20 bounded repetitions.
One full CI rerun passes without WARN/compiler-warning lines, including all
18 package coverage gates and the release build. No code, assertion or timing
bound changed in response. The initial intermittent failure is unresolved and
retained; the rerun does not establish its cause or a fix. Rust instructions
were additionally reviewed for this diagnostic boundary.

Full UI still fails: 57 passed, one failed, 72 not run; scheduling returns
`media_profile_filesystem_identity_required` and teardown rejects missing UI
coverage. Sonar input verification rejects absent JS LCOV, and canonical
authentication is absent. Local Rust coverage has 124,088 positive line
records; it is not published Sonar coverage. All owned CI/UI databases and
volumes were removed, the UI endpoint is closed, and test-media cleanup passes.
Source and current proof, native experiment, failed/passing CI, UI, coverage
and cleanup evidence are retained in
`artifacts/media-verification/2026-09-13-guid-trace/`. User checkout changes
remain untouched. No scan, push, package qualification, complete handoff or
cutover is claimed.

### Rust Pool And Runtime Rank Qualification (2026-09-13)

- Motivation: choice 1's remaining acceptance requires actual Rust `PgPool`
  execution and changes to trust rank during ingestion-backend reuse. Earlier
  psql-only execution and initial fixture ranks did not prove those cases.
- Design: a transition-only, explicit-input Rust test calls the existing
  production `search_result_ingest` wrapper through a one-connection pool.
  It records direct role/backend/version identity, cold and restored compiler
  settings, typed outputs and exact SQLSTATE/message/detail across a first
  commit, committed reuse, a rejected request and the following valid call.
  The operational proof owns separate reference/final database clones and
  captures all 18 application table images before/after. It verifies returned
  identities and the last-seen timestamp implied by successful calls only.
  Ordinary Rust tests exercise the explicit-launch guard; they do not bootstrap
  init. The canonical proof requires the actual dedicated test and a fresh
  report, so the ordinary guard-only pass cannot qualify the pool case.
- Runtime rank: a separate committing writer changes the `public` tier while
  the ingestion backend remains open. Six sequences traverse both directions
  across 19/20, 29/30 and 39/40, with cold/helper-first modes and both database
  variants. Independent expected confidence values, transaction/backend
  identities and full read/write images distinguish runtime changes from
  fixture setup. First-call rollback retains compiled plans; the following
  two calls commit. Frozen third-call D4 failures remain exact counterevidence.
- Test coverage: the pool oracle currently has 59 assertions, including
  altered/missing table evidence, identity, privilege, compiler-setting,
  result/error fields and a negative control for the mutation harness itself.
  Its live pair passes the 19 pool checks plus the mandatory D4/D5 matrix.
  The rank worker's 24 live variants passed; review then tightened both writer
  JSON parsers, added duplicate-field mutations and removed a redundant tracked
  live launcher. Its final focused suite passes 78 assertions. At committed
  `818889e2`, the combined canonical run passes 6,662 of 6,663 checks; the only
  failure is the retained incomplete-D3 sentinel. Full CI passes with all 18
  package coverage gates and no warning lines. Full E2E fails with 57 passing,
  one failing and 72 unrun tests: legacy profile scheduling lacks verified
  filesystem identity; dependent UI tests do not produce coverage. This is not
  a fixture bypass opportunity: the approved deployment-owned root contract
  and coordinated init cutover remain prerequisites. No published Sonar or
  package pass follows.
- Retained failures: the first pool oracle confused the generic error message
  with `DETAIL`; frozen source independently confirms both fields, and the
  refined probe preserves and checks them separately. Review also corrected a
  mutation helper that could catch its own failed assertion; the unchanged-
  evidence negative control now rejects that behavior. Initial unit output is
  not independent proof of rejection. A test-struct naming lint failed and was
  fixed without suppression. Rank evidence retains its missing-candidate
  preflight, missing parsed-frame handoff and tied-rank ordering failures.
- Observability: new records are proof artifacts only. No production logging,
  SQL, schema, privilege, GUC or error-disposition change. Retain raw Rust/psql
  output, exact source and candidate hashes, failed attempts and cleanup.
- Risk and rollback: the proof uses the existing disposable PostgreSQL role
  model with one dynamically allocated literal-loopback transport for the
  actual host pool. It rejects non-loopback or mismatched database/role inputs,
  an existing/missing report, warnings and unsuccessful execution. Each clone
  and its container/volumes must be removed. Remove the probe, proof inclusion,
  recipe and matching instructions together if this test arrangement is wrong;
  never reinterpret missing evidence as D3 approval or activate init early.
- Dependency rationale: existing SQLx, Tokio, Serde, UUID, Chrono, anyhow and
  Ruby standard library only. No manifest or production dependency change.
- Stale-policy check: root AGENTS and scoped Rust, data and DevOps guidance were
  reviewed. Data/DevOps now describe the explicit proof entry, runtime-rank
  requirements and unchanged bootstrap authority. These are internal tests of
  approved D3/D4/D5 scope, not a new operator approval or criteria exception.
- Limits: one controlled pool size and bounded v1 rank cases do not qualify
  cancellation during ingestion, all cached/helper/native/dynamic paths,
  complete D3, the installed service, S2, either Linux package or release. The
  incomplete-D3 guard, frozen authority and all other acceptance gates remain.

### Rust Ingestion Cancellation Qualification (2026-09-13)

- Motivation: conditional D3 includes cancellation during actual application
  ingestion, not merely failed requests or idle-backend signalling.
- Design: in each owned reference/final clone, a separate controller holds a
  source-table lock. It observes the real Rust wrapper waiting to insert while
  holding its canonical-write lock, then cancels only that exact backend.
  An atomic no-clobber checkpoint must retain SQLSTATE `57014`, its exact
  message/detail, direct role and restored compiler settings before unlocking.
  All 18 rollback table images and 19 read-input images are retained; the same
  one-connection pool must subsequently create and persist a valid result.
  No function, trigger, grant, frozen migration or final-init bytes change.
- Test coverage: five Rust launch/input/publication tests and strict data-crate
  all-target/all-feature lint pass. The controller oracle has 223 assertions,
  including mutated identities, outcomes, table/input images and an unchanged
  negative control. The first live reference/final pair passes all three new
  checks and the mandatory D4/D5 matrix (`818889e2` plus its retained patch).
  On committed `1ff11a4a`, all three cancellation checks pass in the complete
  canonical run: 6,665 of 6,666 checks pass; only the incomplete-D3 sentinel
  fails. All 56 cancellation source hashes match the tested commit.
- Integration gates on `1ff11a4a`: full CI passes with all 18 package coverage
  gates and no warning lines. Full E2E fails with 57 passing, one failing and
  72 unrun tests, retaining the exact scheduling identity error and absent UI
  coverage. The API still uses legacy path-based profile writes; approved root
  binding and coordinated init cutover are product prerequisites, not grounds
  to fabricate identities or change success assertions into accepted failures.
- Coverage inputs: native compile input and executed release-script coverage
  were regenerated and preserved through CI. `just js-coverage-merge
  sonar-verify-inputs` passes. Rust LCOV has 322 source records and 133,758 line
  records, 124,467 covered; JavaScript/TypeScript has 61 source records and
  5,402 line records, 4,249 covered. The JavaScript result includes partial API
  execution, not complete UI coverage. The canonical scanner token is absent;
  no scanner analysis, source upload or published Sonar metrics are claimed.
- Observability: retain controller queries, lock wait, checkpoint, full raw
  stdout/stderr, owned-process settlement, source hashes and table images.
  These are qualification artifacts; no production logging changes.
- Risk and rollback: cancel only the observed backend in an explicitly owned
  disposable database. Bound and reap the owned test process, release its lock
  on failure, and remove the clone/container. Remove the probe, canonical hook,
  recipe and matching instructions together if incorrect; never weaken D3.
- Dependency rationale: existing Rust dependencies and Ruby standard library
  only. One worker owned the Rust probe; the integration owner implemented the
  disjoint controller and reviewed the combined result.
- Stale-policy check: reviewed root, Rust, data, UI and DevOps instructions.
  Updated data/DevOps for the explicit cancellation entry and evidence boundary.
  No approval, analyzer criterion or bootstrap authority was changed.
- Limits: one server-cancelled source-insert boundary is not client-future
  cancellation, all interruption sites, helper/native closure, complete D3,
  root workflow, supported-package proof or release completion. Rollback
  assertions cover the named tables, not restoration of sequence allocations.
  Next critical-path work is the finite remaining actual branch/helper/native
  closure, followed by the approved init/root cutover. Client-future
  cancellation remains unqualified, but is not a separately named mechanism
  in D3 or ADR 588's cancellation condition; any demonstrated reachable path
  still belongs to the approved ingestion scope.

### Unpublished Native Callback Feasibility (2026-09-13)

- Motivation: stock PostgreSQL function counters cannot establish built-in RI
  callback execution. Test whether actual entry and the ambient compiler
  setting can be observed without substituting callbacks or modifying SQL.
- Design: source `5b6257f8` plus an ignored, unpublished experiment provisions
  the exact frozen and final candidates in a network-disabled, owned PostgreSQL
  16.14 arm64 container. A separate network-disabled debugger shares only that
  container's PID namespace, with `SYS_PTRACE`; no host PID namespace, Docker
  socket, host mount, inferior function call, target package install, SQL/grant
  modification or production change is used. Debugger breakpoints instrument
  timing; they are not production timing or shutdown evidence.
- Evidence: `target/d3-native-ri/run-20260913-10108-vjev81/` retains ordinary
  and observed direct-role runs. Each variant executes 14 actual
  `RI_FKey_check_ins` entries. The observed native compiler-setting integers
  are 2 in the frozen variant and 0 in the final variant; both report caller
  setting `error` before/after ingestion and native readiness value 0. These
  are raw observations, not a claim that the ambient settings are identical.
  Within each variant, parsed application results and all 18 write-table
  images match its ordinary invocation after existing identity/clock
  normalization. All 14 declared read-input tables remain unchanged. The
  pinned target image and five recorded native-file hashes are unchanged.
- Test coverage: the paired cold/basic feasibility run passes. The first
  attempt reached one real callback but failed to capture its setting; a
  second attempt failed because GDB rejected a nonregular stdin command file.
  Both failures are retained. Staging the exact script into the separate
  debugger container produced the successful third attempt. Debugger stderr
  retains its unavailable musl source-line warning; this is not a clean
  complete native or package qualification. No new canonical acceptance,
  full CI/UI result, Sonar analysis or remote pass follows from this trial.
- Observability: retained GDB commands/stdout/stderr, backend and direct-role
  identity, raw SQL results/settings, table images, image/native fingerprints,
  process outcomes and cleanup records. All trial processes were reaped and
  owned database/debugger containers removed.
- Dependency rationale: development-only GDB 16.3-r4 in an isolated tool image
  provides native entry observations unavailable from PostgreSQL counters.
  Image `sha256:12fbe2ad89fa389c9a801050aa517b747089cc3b453242f1727bb685c3ff39c0`
  and its package-resolution log identify this experiment. This does not add
  a runtime dependency or adopt a permanent CI toolchain.
- Risk and rollback: active-goal authority permits this bounded unpublished
  experiment, not architectural adoption. Preserve its evidence, remove its
  owned resources and discard the experimental harness if unsuitable. Never
  turn an attach, catalog inventory, matching counts or this one basic case
  into a full callback/branch certificate.
- Stale-policy check: reviewed root, data and DevOps instructions and the
  approved D3/ADR 588 conditions. Corrected the previous next-step sentence
  that promoted a later client-future non-claim into a standalone prerequisite.
  No approval, criterion, frozen source or bootstrap authority changed.
- Limits: no update callback, helper-first, warm-cache, cancellation-site,
  complete native/helper closure, Linux amd64 or installed-service proof is
  claimed. The incomplete-D3 guard and init cutover boundary remain intact.

### URI Identity And Changed V2 Conflict Cases (2026-09-13)

- Motivation: close the actual-ingestion gaps for non-magnet/queryless-magnet
  normalization and the changed non-NULL v2 hash logger branch. Known-answer
  helper calls and missing-hash cases do not establish these application paths.
- Design: add both URI inputs without explicit hashes to the existing new,
  reuse and GUID-promotion identity cases in all three wrapper modes. Preserve
  exact stored URI values and independently derived identity expectations.
  Add a selected-GUID fixture with an existing non-NULL v2 hash, followed by a
  different non-NULL v2 hash through the actual wrapper. Require retained
  durable source identity, exact canonical/observation changes, both v2 and
  derived-magnet conflict/audit/health records, and unchanged read inputs.
  A real rolled-back first call and same-backend retry retain sequence gaps;
  this does not substitute for the separately retained frozen D4 counterexample.
- Test coverage: one isolated worker added the two bounded groups and reported
  2,361 focused assertions passing. Parent integration updates the shared
  wrapper inventory from 61 to 67 cases. The integrated ingestion unit suite
  passes, including 6,192 main assertions and the dependency, validation,
  setting-path, attribute, metadata, disambiguation, sampling and GUID suites.
  On executable commit `7648a51e`, all 36 new URI identity variants and all
  six v2-conflict checks pass live. The complete canonical run passes 7,013
  of 7,014 checks; only the explicit incomplete-D3 guard fails. All 11 retained
  source registries match the tested files, including the wrapper's 42 hashes.
  Full CI passes with no warning lines. Full UI fails with 57 passed, one
  failed and 72 unrun: scheduling still requires verified filesystem identity,
  and UI coverage is absent. No SQL, runtime, frozen source, final candidate
  or incomplete-D3 condition changes.
- Coverage and retention: `just js-coverage-merge sonar-verify-inputs` passes.
  Rust LCOV retains 322 source records, 133,758 lines and 124,467 covered lines;
  JavaScript retains 61 source records, 5,402 lines and 4,249 covered lines,
  including partial API execution rather than full UI coverage. The canonical
  scanner token is absent: no Sonar analysis or published result is claimed.
  Local archive `artifacts/media-verification/2026-09-13-native-ri-identity/`
  retains the executable bundle, 29 current canonical entries, all gate logs,
  coverage and 56 raw profiles, worker sources, and the separate native trial
  including its debugger image and failed attempts. The worker was closed
  after byte-for-byte integration verification and its worktree removed.
- Observability: reuse the existing full SQL stdout/stderr, clocks, direct-role
  identity, before/after/rollback table images and read-input evidence. No
  production telemetry changes.
- Risk and rollback: incorrect expectations must fail against the actual
  frozen/final databases, not be accepted from synthetic unit frames. Remove
  these test additions and their inventory/instruction updates together if
  unsuitable; do not change application behavior to satisfy the oracle.
- Dependency rationale: existing proof modules and Ruby standard library only.
- Stale-policy check: reviewed root, data and DevOps instructions. DevOps now
  records the exact URI/v2 evidence requirements; existing approvals and all
  cutover/release gates remain unchanged. Full D3, root workflow, package,
  published Sonar and merge acceptance remain unproven.

### Score And Seeder Boundaries (2026-09-13)

- Motivation: cover the four non-drop total-score clamp/equality controls and
  six nullable/edge seeder-promotion controls identified in the bounded D3 map.
- Design: vary a constraint-valid base score together with real policy/tag
  adjustments. Totals of -10001/-10000 and 10001/10000 distinguish saturation
  from equality; no drop action supplies the lower bound. For promotion, keep
  distinct low/high sources with persisted scores 10/100. Exercise incoming
  NULL, selected-best NULL and selected-best counts 19/20/99/100. Specify all
  fixture, result, write/read, rollback and retry images independently.
- Test coverage: the isolated worker reports 1,424 policy assertions and 40,494
  main ingestion assertions plus included suites passing. Parent review and
  integration retain only four proof/test files; live database qualification
  and combined exact-revision CI/UI results remain pending at this checkpoint.
  Counts change from 49 to 53 policy cases and 67 to 73 wrapper cases.
- Observability: existing retained database transcripts and complete state
  images; no production telemetry changes.
- Risk and rollback: new oracles must be tested against the real frozen/final
  databases. Reject additional differences; do not change SQL or weaken D3 to
  satisfy expectations. Remove these cases and inventory changes together if
  unsuitable. Out-of-range base inputs remain constraint-unreachable.
- Dependency rationale: existing Ruby proof modules and standard library only.
- Stale-policy check: reviewed root and DevOps instructions and the exact D3
  approval conditions. Scoped guidance now requires valid score inputs and a
  distinct selected source. No approval, candidate, runtime or gate changes.

### Unpublished Helper-Entry Trials (2026-09-13)

The isolated native-observation experiment now records actual PL/pgSQL and SQL
handler entry on the pinned PostgreSQL 16.14 arm64 image. Exact target headers
and the arm64 calling convention bind the observed function IDs to live catalog
identities, languages and source hashes. An initial catalog comparison failed
because PostgreSQL serializes OIDs as JSON strings; retaining the original type
and explicitly casting OIDs to bigint fixes this observer comparison only.

The helpers-first control records all nine known-answer helpers at caller
setting 0 before ingestion. Cold/basic and changed-v2 trials also pass paired
ordinary/observed application-state checks within each variant. The latter
records the real conflict logger twice, 18 native INSERT callbacks and one
UPDATE callback per variant. Ingestion helper entries observe reference setting
2 and final setting 0; these are retained differences, not normalized equality.
Target native file hashes remain unchanged. All owned trial containers are
removed. Raw failed attempts and the unavailable musl source-line warning remain
evidence, not a clean native qualification.

These are unpublished feasibility results in `target/d3-helper-entry/`, not a
canonical profiler, new runtime/debugger dependency, complete helper/trigger
closure, package acceptance or D3 discharge. Existing ordinary application
oracles remain authoritative; debugger timing is not production timing proof.

### Boundary Integration Refinements (2026-09-13)

The `e1925839` canonical run retained 7,457 checks and 36 failures, all in the
new promotion full-state oracle. All four score boundary cases passed in cold
and helper-first modes. The promotion oracle incorrectly cleared the magnet
hash despite a valid v1 input. The real frozen and final databases both derive
the same SHA-256 value from that input without a URI. Removing that clear and
adding an independent literal-hash regression makes all 36 retained variants
validate; a new exact-source live run is still required. No SQL change or new
application-semantic delta follows from the erroneous expected value.

The next integrated batch adds 16 policy NULL/error cases. It verifies NULL
v1/v2/uploader candidates, a genuinely hashless title/size fallback for NULL
magnet, a v1-derived non-NULL control, NULL regex/equality operands and exact
release-token/persisted-signal regex stacks. Three invalid NULL families remain
constraint-unreachable rather than being forced through altered constraints.
Parent focused policy verification passes 4,673 assertions; live fixture and
diagnostic qualification remain pending at this checkpoint. Scope is the
existing conditional D3 proof, with no dependency, runtime, approval or gate
change. Raw diagnostics and complete persisted-state evidence remain required.

Native trigger-entry trials now bind each observed function/trigger ID to its
live foreign-key constraint and full definitions. The changed-v2 trial has
19 entries per variant, across 14 constraints and nine tables. URI helper-first
and title/size cold/helper-first trials pass paired observation checks. Retained
entry sequences agree on constraint bindings; caller controls are at setting 0,
and in-ingestion reference/final entries remain 2/0 respectively. These are not
all native branches or skipped-trigger proof. One concurrent helper-first trial
failed initialization with Docker disk exhaustion; its diagnostic is retained,
its containers were removed, and the serial retry passed without capacity or
criteria changes. Future experimental errors are retained per run rather than
in inherited shared diagnostic paths.

The earlier failed canonical run, matching source registries, local source
bundle and CI failure log are retained in
`artifacts/media-verification/2026-09-13-score-helper-entry/`. Full handoff,
cutover, package, Sonar and remote acceptance remain unproven. Rollback removes
the new proof cases and corresponding guidance together, never alters SQL to
satisfy their expectations. The root and DevOps stale-policy check found no
approval change; scoped guidance now records the actual NULL derivation and
nested-error boundaries.

### Qualified Boundary Batch (2026-09-13)

Executable revision `e545d849` passes 8,011 of 8,012 canonical checks; only the
explicit incomplete-D3 guard fails. All 36 promotion variants, all four total
score cases and all 32 new policy cold/helper-first comparisons pass live,
including the exact nested regex diagnostics. All 11 retained source registries
match the tested files. The earlier erroneous oracle and its 36 failures remain
archived rather than being overwritten by the successful rerun.

Full `just ci` passes without WARN/compiler-warning lines, including all 18
package coverage gates and release compilation. Full `just ui-e2e` remains
57 passed, one failed and 72 unrun: schedule enablement still receives the
filesystem-identity-required 400 instead of 200, and UI coverage is absent.
`just js-coverage-merge sonar-verify-inputs` passes. Rust LCOV has 322 source
records, 133,799 lines and 124,508 positive records; JavaScript has 61 sources,
5,402 lines and 4,249 positive records from partial API execution, not full UI.

Canonical scanner credentials remain absent. The configured read-only Sonar
connection reports project `VannaDii_Revaer` with an OK gate but coverage 0.0
over 1,344 lines to cover. That existing project result is not tied to this
unpublished revision and does not meet positive-coverage acceptance. No new
analysis, code upload, issue disposition or server-criteria mutation occurred.

The archive `artifacts/media-verification/2026-09-13-score-helper-entry/` retains
both canonical attempts, matching source bundles, all gate logs, coverage,
58 raw/merged profiles, native trials and failures, and worker sources. A
reviewed external-ID conflict patch based on `e545d849` is preserved separately
for the next batch; its worker reports 10,595 focused assertions, but it is not
integrated or live-qualified here. Completed worker worktrees were removed after
preservation. Both integration worktrees ran test-media cleanup, and the exact
temporary UI port was confirmed closed. No source push, merge, runtime cutover
or release acceptance follows from this checkpoint.

### Differing External-ID Integration (2026-09-13)

The next conditional-D3 case covers differing IMDb, TMDB and TVDB values in
cold, helper-first and mutating-helper-first rollback/commit modes. A separately
observed administrator fixture follows real non-ID ingestion; it is not claimed
as successful frozen ID ingestion. Complete table/read images, exact D5 errors,
logger/audit/health payloads and seven sequence counters remain independently
checked. Frozen errors stay unequal to approved final success. Live validation
is pending at integration; synthetic assertions alone do not qualify this case.
No SQL, dependency, runtime, approval or gate change is made. Root and DevOps
guidance were reviewed; the scoped instruction now records this fixture and
oracle boundary without relaxing the incomplete-D3 guard. Rollback removes the
new proof case, its tests and scoped guidance together; existing evidence stays
retained. Observability changes are limited to disposable proof evidence.

### External-ID Qualification And Helper Errors (2026-09-13)

Revision `fa721b33` passes all eight differing-ID variant/mode executions and
their four paired fixture checks. Every frozen attempt retains its exact D5
error/rollback; final rollback and committed retries satisfy the independent
18-table, read-input, logger and sequence oracles. The canonical result is
8,023/8,024, with only the unchanged incomplete-D3 guard failing. All 11 archived
source registries match. Full `just ci` passes without warnings, including all
18 package coverage gates. Full `just ui-e2e` remains 57 passed, one failed and
72 unrun, with missing UI coverage. The legacy profile procedure rejects
scheduling; ADR 557's approved association/root workflow must be implemented,
not replaced by a fixture that accepts the current 400 response.

Four unpublished native trials now observe token and persisted-signal regex
failures in cold and helper-first sessions. Both variants preserve the exact
nested diagnostic and complete rollback, with equal ordinary/observed results
within each variant and unchanged target native hashes. The actual nested
helper and final regex operator have reference/final settings 2/0; caller and
helper-first controls remain 0. These existing-identity error cases require
zero RI entries. The initial inherited success-only RI expectation failed and
is retained alongside the corrected error-specific oracle. A separate populated
success trial still fails because its SQL cast helper was not observed; optimizer
inlining is only a hypothesis. No canonical profiler, native closure, package
acceptance or D3 completion is adopted from these experiments.

The bounded reachability sidecar distinguishes constraint-unreachable signal
updates from valid direct-writer schedules for eleven durable attribute updates
and a zero-sample path. Missing trust rank is also schema-reachable. Its source
reasoning and focused assertions are preserved, not live concurrency evidence.
The prepared two-case trust-rank patch passes its worker's focused tests but is
not integrated or live-qualified here. It retains explicit pending native branch
evidence; common confidence output cannot substitute for branch observation.

`just js-coverage-merge sonar-verify-inputs` passes. Retained Rust LCOV has
322 sources, 133,799 lines and 124,475 positive records; partial API JavaScript
has 61 sources, 5,402 lines and 4,249 positive records. No new Sonar scan or
positive published coverage is established. The archive
`artifacts/media-verification/2026-09-13-external-id-policy-entry/` retains the
canonical source bundle, logs, coverage, 58 profiles, all six native attempts
and the pending worker patches. Completed workers, owned databases/volumes and
the temporary debugger image were removed; both integration worktrees cleaned
test media, and UI port 61092 was confirmed closed. ADR 589 remains pending.
No SQL cutover, source push, merge or release acceptance is claimed.

### Reachable Concurrency Batch (2026-09-13, In Progress)

Source `09c24398` live-qualifies K1's two missing-trust-rank cases, K2's
durable-attribute conflicts and K3's sample-prune race. The canonical result is
8,037/8,038 checks: only the unchanged incomplete-D3 guard fails. K2 completes
16 plain/observed executions and four paired cases; K3 completes eight executions
and two paired cases. The [R1/R2 dispositions](support/569-ingestion-reachability.md)
remain source reasoning where marked, not blanket live reachability proof.
K1 still lacks native branch observation; matching confidence is not a substitute.

Both races run unconditionally in the canonical proof and policy suite.
Their current focused harnesses pass 3,381 and 1,147 assertions. Attribute,
shared-validation and metadata harnesses pass 623, 1,430 and 10,614 assertions;
the last includes 10,104 rejected semantic mutations. Canonical evidence at
`artifacts/media-verification/2026-09-13-reachable-concurrency/` retains 31 entries
and 13 matching source registries, including every failed attempt below.

The first live run on `1b725dd2` passes all eight K1 variant/mode executions,
but stops at K2's incorrectly expected parent transaction lock. Actual evidence
binds the blocked child xid `27423` to writer backend `148067`, whose parent xid
is `27422`. The corrected harness additionally records the inserted rows' xmin;
3,381 focused assertions reject substituting the parent for that child. Both
races now run first within D3 without removing or skipping any later case.
The failed source-bound run is retained in
`artifacts/media-verification/2026-09-13-reachable-concurrency/`; the final rerun
result is recorded above.

A separate unpublished plan experiment on `1b725dd2` observes the cast helper
inlined into the reference decision INSERT, but 11 actual SQL helper executions
in final, in both cold/helper-first modes. Ordinary/observed state comparisons
pass independently of reference/final comparisons. This explains the missing
reference SQL entry without supplying a new native setting witness or adopting
a canonical replacement assertion; the prior native trial remains failed.

Revision `37b4048a` completes all 16 K2 plain/observed variant/mode executions
and four paired cases. K3 then exposes PostgreSQL OID JSON string encoding in
the writer's tuple metadata. Both source queries now explicitly cast relation
OIDs to bigint; the numeric validator remains strict. That failed run and its
passing K2 report are retained separately. No application SQL behavior changed.
The next run on `817277eb` also completes K2 and catches a missing derived-table
delimiter in K3's generated lock query (`42601`). The query and original error
are retained; the delimiter and full observer diagnostics are covered by focused
regressions. These are harness corrections, not changes to accepted outcomes.
On `8c26d76f`, K3's exact definition check rejects an observer installation that
reduces the frozen body's 20 backslashes to 10. Literal block replacement now
preserves the body; a controlled transport regression exercises installation and
readback, not only the isolated splice. The failed observer bytes remain retained.

Checkpoint documentation validation also encountered the EBU R128 landing
page's 403 response. Its citation now points to the official recommendation
PDF carrying the same referenced broadcast target. No audio recommendation,
approval, accepted HTTP status or link-check criterion changed; the original
sandbox/network failures remain archived alongside the final rerun.
The final `just docs-link-check` passes all 1,379 links; instruction drift and
diff whitespace checks also pass. These documentation results do not replace
the exact-source CI/UI outcomes or any remaining release gate.

Full `just ci` on `09c24398` initially fails two bootstrap tests with a final
fallback-host DNS diagnostic. A fresh minimal-feature run and full CI retry
both pass, including Rust/script coverage and the release build. The original
cause remains unproven. The retry retains its runner, private server diagnostics
and coverage in `ci-r2-09c24398.tar.gz` (SHA-256
`1cbd75325ba3035a1f3791d55457dfa65a53a249361bb4b8ed1137e04c830190`).
The latest full UI run on that source remains 57 passed, one failed and 72
unrun at schedule enablement, plus missing UI coverage. No new published Sonar
coverage, package acceptance, push, merge or cutover follows from local CI.

### Committed-Warm Cancellation (2026-09-13, In Progress)

The controlled Rust probe now supports explicit cold/committed-warm selection.
A real first ingestion commits a distinct source/hash
on the same single-connection pool. The controller validates the atomic prepared
checkpoint before acquiring the source-insert lock, then publishes a start signal
bound to that database/backend/cache state. Cancellation retains complete prepared,
rollback and read-input images. Frozen warm recovery must retain D4's exact
`42P07`; final recovery independently requires success with distinct identities.
Native in-call settings and full D3 remain unproven. Initial focused checks pass
seven Rust tests and 513 validator assertions. The canonical run on `bed81a12`
passes all four actual cancellation cases and 8,039/8,040 checks, failing only
the unchanged incomplete-D3 guard. All 31 canonical entries and 13 matching
source registries are archived. Full lint finds one duration-notation issue;
the follow-up expresses the same three-minute wait in minutes and moves
cancellation directly after the two races for earlier failure diagnosis. No
case or gate is removed. The follow-up strict all-target Clippy run passes.

At `82e2d3d6`, the repeated canonical run again passes all four cancellation
cases and 8,039/8,040 checks; only the unchanged incomplete-D3 guard fails.
All 31 canonical entries and 13 matching source registries are retained before
later experiments. `just ci` passes on this revision, including all 18 package
coverage gates and the all-target/all-feature release build. This is local
verification, not native-package, published-Sonar or merge acceptance.

The preceding CI attempt has a confirmed infrastructure failure: PostgreSQL
could not write `pg_wal/xlogtemp.880` because Docker's disk was full, aborted the
backend and restarted. Only an exact three-entry, unused Revaer runtime cache
chain was removed, recovering 702.3 MB; unrelated builder data, containers and
volumes were untouched. The passing CI rerun used the existing host-bind test
database pattern. A subsequent host-backed UI setup failed PostgreSQL's directory
ownership check and was retained, not counted as a test run. The Docker-volume UI
rerun reaches the tests: 57 pass, one returns the unchanged
`media_profile_filesystem_identity_required` instead of 200, and 72 dependent
tests do not run; UI coverage remains absent. Stock Alpine locale and local-auth
initialization warnings are retained in the private server logs. No ownership,
durability, warning, deadline or acceptance check was bypassed. These observations
do not establish the cause of the older `09c24398` initial setup failure.

Two separately archived, unpublished native trials observe title/size and magnet
normalization/hash helpers during actual cold and committed-warm ingestion on
the same backend. All four helpers retain observed compiler-setting values 2
in the frozen reference and 0 in the final variant. Plain/traced application
results and full persisted/read images match within each variant using the
existing validated identity/observed-clock normalization; the exact frozen D4
failure remains distinct from final success. Target native
hashes are unchanged. This does not qualify all native branches, skipped
callbacks, cancellation-site settings or complete D3; no experiment was adopted
as a canonical criterion or uploaded.

`just sonar-compile-db`, `just js-release-coverage`, `just js-coverage-merge`
and `just sonar-verify-inputs` pass with current executed inputs. This is not
published coverage. `SONAR_TOKEN` is absent in both working checkouts; the prior
file-level Rust analysis retains its organization-entitlement 403. No scanner,
server criteria or issue disposition changed. A bounded metadata pass reports
104 open stack PRs with conforming titles and VannaDii assigned. PR 72's CLI and
connected-GitHub Copilot requests have no confirmed pending/current-head review
readback; no bulk request or review-resolution success is claimed.

The private `2026-09-13-reachable-concurrency` archive retains canonical reports,
failed and passing gates, runner/server diagnostics, raw coverage and the two
native trials. Every owned test container/volume, both host data directories,
the restored debugger image and the completed agent worktree were removed;
the UI port was verified closed. The tested implementation remains unpublished.

The independent diagnostic fix `f328be01` preserves ordered endpoint/admin
errors instead of replacing the actual endpoint failure with the fallback's
error. Credential-safe rendering, candidate order, successful fallback, probe
failure and redaction have focused coverage. Its worker reports 13 library and
two integration tests plus strict Clippy passing; the parent full CI now passes
at `82e2d3d6`. This improves evidence capture without explaining an earlier
failure whose server log was not retained.

Motivation and design: complete the existing warm-cache cancellation obligation
without changing application behavior or hiding frozen defects. Observability
is confined to disposable proof evidence and test setup diagnostics. No dependency,
schema, runtime, approval or quality-criterion change is included. Rollback removes
these proof additions and restores the earlier diagnostic rendering; retained
evidence and the incomplete-D3 guard remain. Stale-policy review covers root,
Rust, database and DevOps instructions; the scoped cancellation rules now identify
the handshake and exact frozen/final recovery distinction. ADR 589 stays pending.

### In-Call Cancellation Setting (2026-09-13, Experimental)

A bounded unpublished experiment on `2865b1b9` observes the actual Rust
ingestion backend while its source INSERT is blocked by the owned lock. The
read-only debugger captures matching backend PID, database OID and compiler
setting, detaches, then verifies the same backend identity and exact owned wait
before the unchanged cancellation controller delivers cancellation. All four
cold/committed-warm cases report setting 2 in the frozen reference and 0 in the
final variant. The existing oracle retains exact `57014`, full rollback across
18 write and 19 read images, frozen warm `42P07`, and distinct successful final
recovery. Source/native identities remain unchanged.

This is an in-call witness, not canonical D3 acceptance. No plain/observed
pairing or normalization was performed: the unchanged controller does not
record the independent seed transaction clocks. Raw timestamps, errors and
GDB missing-source warnings remain in the evidence. The focused driver does
not invoke or override the full proof driver, keeps `d3_complete=false`, and
does not alter any canonical criterion. Owned containers, anonymous volumes
and processes were removed; the integration owner retains the experiment and
checksummed evidence before removing its clean worker worktree. No experiment
is uploaded or adopted as production behavior.

The separate bounded bootstrap investigation completed four cold starts but
did not reproduce the host-bind ownership failure. A child-directory candidate
is not a proven fix; locale/auth initialization warnings remain unresolved.
Fixture collation/checksum differences were investigated but not adopted.
The worker's evidence is retained in
`target/bootstrap-validation-20260913-01a09d29.tar.gz`; its worktree is removed.

### Paired Cancellation Observation (2026-09-13, Experimental)

At clean source `002942561c7fbcad885456ff382c40808aceecfb`, eight actual
Rust-wrapper executions establish four plain/observed comparisons: reference
and final, each cold and committed-warm. Both arms retain identical owned
lock scaffolding. A read-only statement records the seed transaction clock;
owned source-insert waits expose each applicable warm-up, cancellation and
recovery transaction's `pg_stat_activity.xact_start`. Repeated reads and exact
wait/backend identity checks precede lock release. Only the observed arm
attaches the read-only debugger during the cancelled call, then revalidates
the same backend and wait after detachment. No Rust, frozen SQL, final SQL,
privilege, timeout or canonical acceptance criterion changes.

All eight unchanged cancellation oracles pass again on retained readback.
The separate comparator matches all 18 write-table images, all 19 read-input
images, prepared/checkpoint/final frames, literal errors and caller settings.
Normalization is restricted to validated session/locker identities, independently
bound source/canonical public IDs, and exact columns equal to their independently
observed transaction clocks. Numeric relationships, input timestamps and arbitrary
text remain literal. Its 3,684 unit assertions include mutation of every scalar
in the four retained evidence shapes; their synthetic clocks are not live proof.
The four real comparisons pass, retaining exact `57014`, literal rollback,
frozen warm recovery `42P07`, and the approved final recovery success separately.

Evidence remains unpublished under
`artifacts/media-verification/2026-09-13-paired-cancellation/`. The original
capture report says comparison pending; the separately hashed comparison report
records the later four passes without rewriting that capture. All 75 producer
source hashes and target native hashes remain unchanged. The four native setting
witnesses remain 2 for reference and 0 for final. GDB's missing musl source-line
warning is retained, not suppressed or counted as a warning-free gate.

This closes only the paired-observer gap at the owned cancellation site. It
does not certify every reachable helper/trigger, canonical D3, cutover, UI,
packages, Sonar or release. Full CI/UI are not rerun for this experiment-only
increment; the last executable checkpoint still has passing CI and failing UI.
Root, data and DevOps instructions and the existing ADR 569/588 approval scopes
were reviewed; no policy, dependency or production observability changes.
Rollback discards the experiment only. Cleanup and archive verification are
recorded with this batch; the goal and conditional D3 remain in progress.

### Cached Policy Helper Observation (2026-09-13, Experimental)

At `8e0fff40`, the existing canonical policy fixture, three-call protocol and
full oracles run unchanged for five cases: populated fields, token match,
persisted-signal fallback, and both nested regex failures. Cold/helper-first,
reference/final and plain/native-observed arms produce 40 executions and 20
matching application comparisons. All 282 canonical checks pass, including
18-table rollback/commit continuity, 19 read inputs, exact outcomes and frozen
D4. The native phase gate remains failed; no canonical criterion is replaced.

Independent unit tests found 13 interval-escape holes in the new experimental
phase validator. Tightening it and separating upstream regex work from the
actual failing policy operator yields 20 valid fixtures and 164 rejected
mutations. The first upstream-aware test revision selected the wrong regex
events; its failed output is retained, and mutations now identify the operator
immediately after the policy text helper. Named upstream operators and their
settings remain checked; complete upstream counts/callsites are not certified.
Separate hashed readback qualifies 12 native cases across rollback, commit and
cached reuse/error. Eight remain failed on reference SQL-cast execution or
helper-first calibration, consistent with the previously observed inlining
distinction. The original capture's 6-qualified/14-failed report is unchanged.

The first run stopped on the cast assertion. A second run failed with ENOSPC
during frozen-reference initialization. The complete matrix used the existing
owned host-bind test storage pattern, preserving database SQL, roles and bounds;
it does not qualify installed Linux storage. All 69 producer source hashes and
native hashes remain unchanged. Twenty-one owned containers and the temporary
database directory are removed. The agent is closed and its completed worktree
removed; debugger source warnings and every failed attempt remain retained in
`artifacts/media-verification/2026-09-13-cached-policy-helpers/`.

Next evidence work must address actual inlined-cast execution and compiler scope,
including cached/helper-first contexts, using these retained cases and the prior
plan experiment. Repeating the application matrix alone cannot close that gap.
Full D3, init/root cutover, CI/UI handoff, published Sonar, package and release
acceptance remain open. CI/UI are not rerun for this isolated experiment.
Reviewed root/data/DevOps instructions and ADR 569/588 scope; no dependency,
production observability, architecture, policy or approval changes. Rollback
discards the experiment only; all original acceptance requirements remain.

#### Qualified Cached Cast Readback (2026-09-13)

At `e7e7f4f6`, eight focused reference contexts now pair actual inlined CASE
plans with native executor-entry compiler settings: five helper-first controls
at `error` and twelve ingestion executions at `use_column`. Completed plan
rows, exact frozen SQL/callsite, same-backend phase order, cast definition and
all existing application oracles qualify each observation. Corrected captures
match the independently revalidated retained plain results. Strict offline
readback also revalidates the other twelve contexts: this bounded policy matrix
now has 20 qualified contexts, with no additional application-semantic delta.

The experimental reader omitted the canonical duplicate-key option, and older
aggregate reports contain duplicate cleanup keys. Those summaries are rejected
as acceptance inputs; their original bytes remain retained. Corrected reports
and strictly revalidated individual evidence replace them. A failed mutable-
oracle reconstruction and three initially accepted validator mutations are
also retained. The corrected suite passes 69 checks; mutation-construction
errors cannot masquerade as validator rejection. Canonical readers are unchanged.

Evidence is retained in
`artifacts/media-verification/2026-09-13-cached-cast-scope/`. Owned containers,
host database directories, restored debugger image and the completed agent
worktree are removed; no test media was generated. Debugger source warnings
remain explicit. This is experimental qualification, not canonical profiler
adoption or D3 completion. Next reconcile the remaining native/trigger
obligations with these results; do not repeat this completed policy matrix.
Frozen bootstrap, approval boundaries and all CI/UI/Sonar/package gates remain
unchanged. Only documentation checks were rerun for this increment.

#### Qualified FK Compilation Contexts (2026-09-13)

At `c2484dba`, the existing cold-logger and logger-first cases pass in both
frozen/final and plain/native arms: eight executions, 16 application operations
and 66 original checks. An independent, trace-blind oracle derived from the
authored SQL, fixture/catalog and pinned PostgreSQL `REL_16_14` dispatch source
predicts 18 INSERT plus one changed-key UPDATE callback per ingestion, and five
INSERT callbacks per direct logger. Strict offline readback qualifies all four
native contexts and their 96 entries against those per-operation multisets.
Raw trace/catalog bindings, helper-source identity, transaction phases, all 19
read-only inputs and canonical application comparisons remain checked. Ten
parent-side callback probes record no entries for these unchanged-key cases.

The phase validator initially accepted a callback moved before its nested
logger. Its ownership/order check is corrected; all four valid contexts and
124 negative cases pass, alongside three JSON/parser checks (131 total). The
superseded readback and failed duplicate-fixture setup remain retained in
`artifacts/media-verification/2026-09-13-fk-compilation-scope/`; the accepted
readback is `fk-readback-20260913-70500-c1fasy`. Debugger missing-source warnings
remain explicit. Owned containers, host database data, the restored debugger
image and the completed agent worktree are removed; no test media remains.

This closes only these two cases' callback-count and phase questions, not all
FK paths, native/helper closure, D3, the single-init cutover or release gates.
The remaining external-ID, attribute and best-context bindings need targeted
qualification; this successful matrix must not be rerun merely to recount it.
No production code, dependency, observability, architecture, bootstrap authority
or acceptance criterion changed. Root policy was reviewed without new drift;
rollback discards the local experiment. Documentation gates alone were rerun;
full CI/UI, published Sonar, package and PR acceptance remain outstanding.

#### External-ID And Wrapper FK Qualification (2026-09-13)

At `b6d64901`, canonical `imdb-upsert` and existing-v2 warm-rollback cases
complete eight plain/native executions and 16 tested operations with 38 checks.
Independent source-derived counts qualify all four native contexts: IMDb's
frozen failures enter nine callbacks each; final insertion/reuse enter 18 and
one. Each v2 operation enters 19 callbacks within v1 and two after its return
in wrapper 0120, at the restored caller setting. The 121 observed entries cover
the five targeted external-ID, durable-attribute and best-context bindings.
The wrapper's superseding frozen definition, not its original 0052 body, is
part of the independent source identity. No further semantic delta was found.

Strict readback binds raw traces/catalogs, exact routine bodies and SQL, full
application oracles, all 19 unchanged read-only inputs and transaction phases.
It retains the approved D5 reference failures and D4 lifetime differences.
The checker initially accepted seven invalid helper, lifetime and ownership
mutations; these are repaired. The final suite accepts four valid contexts,
rejects 98 mutations and passes three harness checks (105 total). One test-
cloning setup failure is retained and is not counted as validator rejection.
The final readback is `fk-remaining-readback-20260913-90176-t4oe99` (36 canonical
checks); its earlier readback is superseded, not acceptance evidence.

Evidence is retained in
`artifacts/media-verification/2026-09-13-fk-remaining-scope/`, including initial
failed validator output, the partial setup-failure log and debugger source
warnings. Owned containers, host database data, restored debugger image and
completed agent worktree are removed; fixture cleanup passes. Root, data and
DevOps policy were reviewed; no production, dependency, observability, approval,
bootstrap or criterion change occurred. Rollback discards this local experiment.
Only documentation gates were rerun. Next reconcile the original eleven-family
closure audit with the accumulated qualified evidence; do not repeat this
successful capture matrix. These binding observations do not establish every
FK path, full D3, cutover, CI/UI/Sonar/package or PR acceptance.
