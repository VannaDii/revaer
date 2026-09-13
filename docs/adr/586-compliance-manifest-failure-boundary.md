# Compliance manifest failure boundary

- Status: Accepted
- Date: 2026-09-11
- Operator approval: 2026-09-11, explicit C1/C1-D and conditional E1 approval
  in [ADR 588](588-first-release-decision-package.md#approval-resolution).
- Implementation status: Approved for implementation; not package-qualified.
- Context:
  - Provider reconstruction reaches a retained loader that converts filesystem
    and JSON failures into `None`, then reports a digest-shaped unavailable
    sentinel. Root policy prohibits hiding recoverable failure in absence,
    sentinel values or logs. G1 reserves observable failure classification and
    availability changes for operator approval.
- Decision:
  - C1 below is accepted together with ADR 588's exact C1-D delivery contract.
    No degraded fallback or criteria exception is selected.
- Consequences:
  - The recommendation makes the packaged manifest a startup prerequisite.
    Missing or malformed evidence stops startup rather than degrading silently.
  - Development and E2E bootstrap must supply real valid fixture evidence or
    explicitly inject a test loader; they may not manufacture a production
    digest or treat a missing artifact as verified compliance.
- Follow-up:
  - Implement and qualify the approved loader/bootstrap/delivery contract.
    Earlier pending wording below is retained proposal history; the exact
    ADR 588 resolution governs current implementation authority.

## Verified Conflict

At donor `d540f67c` and retained integration `65dbefb7`,
`crates/revaer-api/src/http/router.rs::load_source_compliance_bundle_digest`
reads `/app/compliance/final-image-compliance-bundle.json`. Read errors, JSON
errors, missing `source_compliance_sha256`, and invalid digest text are logged
and returned as `None`. A syntactically valid 64-character hexadecimal value
is returned with a `sha256:` prefix. This is a format check, not an independent
content, authenticity or package verification.

`with_config_with_media` places that optional result into API state. The media
compliance handler replaces absence with
`unavailable-until-final-image-bundle-is-present` and can return HTTP 200. The
same absence representation therefore covers several real failures. Retaining
logs does not satisfy the root's explicit `Result` requirement.

The newly validated schema-only children do not introduce this loader. The
conflict arises at the next provider/bootstrap owner; it must not be silently
reintroduced or changed under a mechanical-reconstruction claim. The existing
operator approvals for R1-R4, B1-B3, D1/D2/conditional D3, S1 and narrow F1 do
not select this startup failure policy.

## C1: Recommended Startup Failure Policy

Approve the following bounded behavior together:

1. Load the existing manifest and its existing digest field through an injected
   bootstrap collaborator before spawning application background tasks or
   accepting requests. Preserve the existing production location and digest
   syntax; do not add a new environment override, bypass or numeric limit.
2. Return an explicit typed error for unreadable/missing files, malformed JSON,
   absent/non-string digest fields and invalid digest text. Log once at the
   origin with a bounded cause category, without document contents or invented
   evidence. Propagate to a failed startup and nonzero process exit.
3. Inject the successfully loaded digest into the real media-enabled API state.
   Remove the failure-to-absence conversion and unavailable digest sentinel
   from this production path. Keep successful response fields and values
   unchanged. Test-only collaborators must remain explicit and must not be
   selected by production fallback logic.
4. Do not add a degraded-ready mode, HTTP status alternative, retry loop,
   partial media admission, shutdown deadline or process-containment fallback.
   Performing this check before task creation avoids selecting new cleanup
   semantics for tasks that have already started. S2 remains held.
5. Do not call a parsed digest verified package evidence. Actual manifest
   integrity, complete artifact closure, Linux amd64/arm64 package tests and
   all existing compliance gates remain separately required.

This would intentionally make all service startup unavailable when required
manifest metadata is absent or malformed, including otherwise unrelated API
features. That availability consequence is the reason approval is required.
It is not permission to weaken any GitHub, Sonar, dependency or release check.

## Alternatives

- Retain the current absence/sentinel path. This does not satisfy root policy;
  it would require an explicit separately scoped policy exception. None is
  requested or recommended here.
- Keep the process running with an explicit failed compliance state, HTTP
  errors, degraded readiness and media-admission fencing. This preserves
  unrelated service availability but requires a larger state/health/admission
  contract. It is not selected or partially implemented by C1.

## Validation Required After Approval

- Exercise real missing files, access failures, malformed JSON, absent/wrong
  field types, invalid hexadecimal/length and valid upper/lowercase digests.
  Assert typed errors, exact event counts and no source-content leakage.
- Prove failure occurs before any background-task spawn, listener admission or
  media command; prove the top-level process exits unsuccessfully. Do not use
  boolean-only mocks as a substitute for the real bootstrap failure path.
- Prove valid injected fixture evidence starts the actual application, exposes
  the unchanged successful compliance response and records real capabilities.
  Verify no production fallback selects fixture data.
- Run full `just ci`, `just ui-e2e`, authoritative Sonar and applicable GitHub
  checks, then validate real amd64/arm64 package manifests independently. None
  of these outcomes is established by this proposal.

## Task Record

- Motivation: Resolve a concrete policy conflict before exposing the first
  reconstructed real provider routes, without inventing operator consent.
- Design notes: C1 selects early failure rather than a new degraded-service
  state machine. It preserves successful wire shape and existing file/field
  names; private typed error layout remains an implementation detail.
- Test coverage summary: Read-only donor and integration inspection establishes
  the conversion path. No new runtime test, filesystem failure experiment or
  service behavior is claimed. Documentation validation is recorded in ADR581.
- Observability updates: Proposed bounded error classification only; no logger,
  severity, health, readiness or error response is changed in this task.
- Status-doc validation: Reviewed ADR559's G1 boundary, ADR564's completion
  ledger, root policy and retained provider code. Operator guides remain
  unchanged because C1 is not implemented. Both ADR indexes are updated.
- Risk & rollback plan: Missing package evidence would prevent startup under
  C1. Rejection keeps affected reconstruction unpublished pending another
  approved contract; it does not authorize restoring a hidden-failure fallback.
- Dependency rationale: No dependency is added or proposed. Existing standard
  file I/O, JSON parser and injected bootstrap patterns suffice.
- Stale-policy check: Reviewed root AGENTS and Rust/devops scoped guidance.
  Found retained loader behavior inconsistent with failure/absence policy;
  no policy is relaxed and no existing approval is rewritten. The fix remains
  pending a named decision rather than treating an agent delegation as consent.

## C1 Implementation Checkpoint: 2026-09-11

This checkpoint implements only ADR 588's approved early C1 boundary. It does
not change the approval record above or implement C1-D delivery, Helm, E1,
database finalization, shutdown policy, or release qualification.

- Motivation/design: Both public app entrypoints call the same synchronous
  typed preflight before `BootstrapDependencies` construction. Constructors
  require `SourceComplianceMetadata`, which is carried into the real API state.
  The type can only be loaded successfully (apart from explicit API unit-test
  fixtures); it has no production default, optional-error value, or sentinel.
- Compatibility: Preserve `/app/compliance/final-image-compliance-bundle.json`,
  `source_compliance_sha256`, the existing 64-ASCII-hex check, trimming, lowercase
  `sha256:` wire value, JSON duplicate-key behavior and symlink following. No
  environment override, file-size limit, retry, installer, or new dependency.
  Parsing this field is not authenticity, artifact-closure, or current-package
  certification. API constructors now require the metadata witness explicitly.
- Observability: The preflight writes exactly one task-free bounded category
  to its injected diagnostic writer before telemetry exporters can start.
  File/document/digest contents are not included. Read/JSON/field/digest errors
  remain typed, with original sources retained. A diagnostic-write failure is
  also returned with the original compliance failure; it is not retried or
  silenced. The binary exits unsuccessfully without a second error report.
- Test coverage: Real missing files, directory reads, permission denial,
  non-UTF-8 files, malformed JSON, missing/wrong-type fields and invalid digest
  syntax exercise the actual bootstrap's first poll without a Tokio runtime.
  Tests assert one loader call and one exact diagnostic, including no content
  leakage. Public entrypoints and the real binary are also exercised on the
  host's packaged-metadata path. Every metadata failure maps to process failure.
  Runtime tests moved into a test-only bootstrap module to inject a loader
  explicitly, never a production fallback. An isolated test child starts the
  actual application, serves the unchanged successful compliance response and
  reads back real persisted ffmpeg/ffprobe capabilities. Test-child cancellation
  does not qualify production shutdown or native package behavior.
- Validation: `just fmt`, `just instruction-drift` and all four passes of
  `just lint-runtime-shutdown` passed. Final `just test-features-min` passed:
  API 436 library + 2 binary tests; app 257 library + 4 binary + 2 integration
  tests; both doctest targets completed with zero tests. Zero failures or
  ignored tests, no compiler warnings or database-skip messages. The real binary
  exited 1 with exactly `compliance_metadata_startup_failed cause=missing_file`
  on this unpackaged host. The injected-success child passed in 2.16 seconds;
  the complete app library run passed in 112.96 seconds. These are observations,
  not new numerical acceptance criteria. Evidence logs remain local at
  `/private/tmp/revaer-c1-features-min-final.log` and
  `/private/tmp/revaer-c1-lint-2.log`.
  Initial compilation/lint findings were corrected without adding
  dependencies or weakening criteria. Full `just ci`, `just ui-e2e`, coverage,
  Sonar, remote checks and native amd64/arm64 package verification are not
  claimed by this worker.
- Reproducibility: Base `62c5d6a7e604eb2a112fd2578ae650a6900a7a9a` plus this
  checkpoint's scoped app/API Rust delta; worktree
  `/private/tmp/revaer-compliance-startup`, branch `work/media3-compliance-startup`.
  The binary-format Git diff of the tested app/API Rust delta has SHA-256
  `ce8dd269ae75e67d33c098d106b6a96240049e0ca29afeef8fecf94b168bb36a`;
  only this documentation checkpoint changed after those code gates.
  Own target directory, four Cargo build jobs and serialized Rust tests.
  Both `DATABASE_URL` and `REVAER_TEST_DATABASE_URL` point explicitly to the
  owned container `revaer-c1-startup-20260911-62c5` at `127.0.0.1:55263`.
  PostgreSQL image:
  `docker.io/library/postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`
  (linux/arm64). No generic database container, port 7070, CI/UI overlap,
  browser monitoring, deployment, or remote push is used.
  The owned container and anonymous volume were removed and their absence
  verified after the final tests. No task media was acquired or retained;
  fixture files and the child workspace used disposable temporary directories.
- Risk/rollback: Production startup now intentionally fails when required
  metadata is absent or malformed. Real bundle delivery remains required before
  package activation. Revert this scoped code checkpoint only to withdraw the
  implementation; doing so does not authorize publishing the old sentinel path.
- Dependency rationale/stale-policy check: No dependencies added. Reviewed root
  AGENTS, scoped Rust/data instructions, ADR 586, ADR 588's current approval
  resolution and C1 detail. Removed the retained failure-to-absence conflict;
  no instruction contract, frozen migration, init SQL, workflow, Just recipe,
  approval status, generated documentation index or parent ledger was edited.

## C1 E2E Bootstrap Checkpoint: 2026-09-11

- Execution status: Implemented with focused verification; integration/full
  gates remain pending. This is not a Recorded completion or package claim.
  Existing ADR 586/588 approval permits explicit test-loader bootstrap; ADR
  588's production-entry prohibition and canonical full UI requirement remain.
- Motivation/design: Replace E2E's unpackaged production-binary launch with
  the app library's exact `bootstrap::runtime_tests::e2e_serving_entry`, defined
  only in the existing `cfg(test)` runtime module. The exact argument vector
  plus `REVAER_E2E_SERVING_ENTRY=1` selects serving; ordinary Rust test runs
  exercise invalid/missing selection guards instead of starting a server.
  The entry uses the existing explicit fixture loader, actual shared typed
  preflight and concrete application runtime. No production/bootstrap API,
  dependency, feature, fallback, `/app` fixture, auth mode or policy is added.
- Wiring/lifecycle: `tests/global-setup.ts` invokes `just ui-e2e-app-build`,
  parses completed Cargo JSON and accepts exactly one current-source library
  test executable. It launches that executable directly, preserving the owned
  serving PID/process group, database URL, media workspace and normal startup
  configuration. Temporary database state is saved before migrations and UI
  PID state before readiness checks so failed starts remain cleanup-visible.
  Occupied ports now fail without broad process searches or termination.
  Existing global teardown, fixtures, Playwright projects/specs, auth/setup
  flows, route assertions and coverage criteria are unchanged.
  Parent-review tightening makes `ui-e2e-bootstrap-test` a direct dependency of
  canonical `ui-e2e`, retaining direct `trunk-install` and `api-test-client`
  dependencies. Just deduplicates API preparation; the 17 bootstrap guards are
  enforced before the full suite, not available only as a manual recipe.
- Focused evidence, from base `e48364da66b025f15c9bb87735b8e2160c5b1c0f` in
  `/private/tmp/revaer-c1-e2e-bootstrap`, branch `work/media3-c1-e2e-bootstrap`:
  `CARGO_BUILD_JOBS=2 just ui-e2e-app-build` passed; actual Cargo output selected
  one executable whose exact test listing contained one serving entry.
  `just ui-e2e-bootstrap-test` passed its strict bootstrap dependency-tree
  typecheck and 17 Node regressions, zero failed/skipped; locked npm install
  and mandatory audit reported zero vulnerabilities. Selection tests reject
  incomplete/failed/ambiguous builds, production/foreign artifacts and invalid
  executable paths; a real occupied listener remains alive after refusal.
- Dependency-tightening verification: Reran `just ui-e2e-bootstrap-test`: all
  17 tests and strict typechecking passed, with zero npm audit vulnerabilities.
  `just --dry-run ui-e2e` and parsed Just JSON assertions proved all three
  direct dependencies, one shared API preparation, and the guard runner before
  Playwright. `just --command bash scripts/workflow-guardrails.sh` and
  `just --command bash scripts/policy-guardrails.sh` passed, as did
  `git diff --check`. Logs under `target/`: `e2e-bootstrap-dependency-test.log`,
  `e2e-dependency-dry-run.log`, `e2e-dependency-assertions.log`,
  `e2e-dependency-workflow-guardrails.log` and
  `e2e-dependency-policy-guardrails.log`. No Sonar analysis was run on this
  checkpoint's eight changed files; canonical full qualification is unclaimed.
- Runtime evidence: With both database URL variables explicitly pointing to
  the owned disposable database, `CARGO_BUILD_JOBS=2 just ui-e2e-app-test`
  passed 8 top-level tests: 2 E2E entry/setup, 4 compliance and 2 production
  entry regressions, zero failed/ignored/skipped. The serving regression invokes
  the exact child entry, requires real HTTP 200, typed health `mode=setup` and
  healthy database, and verifies persisted Setup/auth state is unchanged.
  Only the dynamic listener port is configured. The focused child is killed
  and reaped after assertions; this is not production-shutdown qualification.
  With the test selector present, the real production binary still exited 1
  with exactly `compliance_metadata_startup_failed cause=missing_file`.
  `CARGO_BUILD_JOBS=2 just lint-runtime-shutdown` passed all four existing
  all-feature/minimal and production-panic-boundary passes; `just fmt` and
  `git diff --check` passed, without compiler warnings or new suppressions.
- Evidence files beneath the worktree's `target/`: `e2e-app-build.jsonl`,
  `e2e-entry-selection.log`, `e2e-bootstrap-test-final.log`,
  `e2e-app-test-final.log`, `e2e-app-lint-final.log`,
  `e2e-instruction-drift.log`, `e2e-strict-types.log` and `e2e-cleanup-check.log`.
  Parent must preserve these logs before removing the worktree if needed.
- Limitations: `just instruction-drift` fails only for the parent-owned DevOps
  companion to `just/ui.just`; Rust/UI instructions are updated here. A separate
  `just --command bash scripts/with-node.sh tests/node_modules/.bin/tsc --project
  tests/tsconfig.coverage.json --noCheck false --noEmit` probe reported 72
  errors in unchanged fixtures/specs/API wrappers; no criteria were changed.
  Initial npm audit DNS and compiled-test module resolution failures preceded
  the clean final Node run; canonical `NODE_PATH` wiring fixed the latter.
  One initial disposable DB had an SSLRequest error; final DB startup initially
  lacked its published port, then the existing `just db-start` recovered on one
  bounded retry. These failures are not counted as passing runs.
- Cleanup/reproducibility: Final owned Postgres container
  `revaer-c1-e2e-1b1ba4e646df` used `127.0.0.1:51284`, image ID
  `sha256:7e7dbab8d3b431a20793a6d99cb5a6bc84e44914309917f1bf5589a7568cdefd`.
  Its exact container, anonymous volumes and private database directory were
  removed; earlier attempt resources were also removed. Runtime children used
  the established disposable media workspace; no test media was acquired.
  No parent database/target, generic service port, full CI/UI, browser, cluster,
  upload, push, publication or signing operation was used.
- Operator preparation: Canonical `just ui-e2e` now builds the host app test
  executable itself; no image, signature or `/app/compliance` bundle is an E2E
  prerequisite. Retain the established native/FFmpeg toolchain, exact NVM Node,
  Trunk/browser prerequisites, disposable DB inputs and private `E2E_FS_ROOT`.
  Port 7070 and the configured UI port must be free; setup will not evict owners.
  Full `just ci`/`just ui-e2e`, Sonar and separate package qualification remain
  parent-owned and unclaimed, not waived by this scoped handoff.
- Observability/risk/rollback: No production diagnostics change. Build selection
  and launch-guard failures are explicit. Reverting this scoped checkpoint
  restores the prior E2E prerequisite failure, not permission to install fake
  compliance evidence or bypass production preflight. No dependencies added.
- Stale-policy check: Reviewed root AGENTS, scoped Rust/UI/DevOps and approved
  ADR 586/588 C1/C1-D authority. This checkpoint specializes explicit test-only
  bootstrap without weakening full-suite or production rules. Parent owns
  DevOps, ledger, navigation/generated docs and full-gate reconciliation.

### Parent Integration Result (2026-09-11)

- Amended worker `bee07560` was integrated byte-for-byte as `1c470560`;
  matching DevOps guidance is in the parent delta. The completed worktree was
  removed after its gate logs were archived under
  `artifacts/media-verification/2026-09-11-c1-e2e-bootstrap/`.
- Full `just ui-e2e` on `1c470560` plus the recorded parent proof/documentation
  delta reaches the real API: 57 passed, one failed, 72 did not run. Schedule
  enablement in `media.spec.ts` still returns 400 with
  `media_profile_filesystem_identity_required` instead of the required 200.
  The unchanged positive assertion remains blocking. No UI coverage files were
  produced because the suite stopped before UI execution, so teardown also
  fails. This is not full E2E or automation qualification.
- The owned database and anonymous volumes were removed; canonical fixture
  cleanup passes. Full-file MAIN TypeScript Sonar reports zero issues for the
  setup and its new guard tests. Authoritative Sonar, published coverage,
  stable-source full CI and both native packages remain outstanding.

The legacy procedure rejects schedule/watch enablement and a supplied interval
unconditionally (`0182_media_normalized_configuration_identity.sql:1417`).
There is no missing fixture step that makes this PATCH valid. Complete D3 and
the accepted init transition, implement ADR 557's attested root catalog and
versioned profile/source-association workflow, then exercise real association
activation in E2E. Positive enablement/run assertions remain required; no SQL
fixture bypass or migration edit is authorized.

The companion paragraph is implemented in the parent DevOps instruction delta:

> Canonical `just ui-e2e` prepares the host app with `just ui-e2e-app-build`,
> selects the exact completed Cargo library-test artifact and directly runs
> `bootstrap::runtime_tests::e2e_serving_entry`. Its explicit compliance loader
> is `cfg(test)` only; production entrypoints retain required packaged metadata.
> Preserve the real runtime, disposable database, setup/auth flows, owned process
> groups, complete suites, route assertions and coverage gates. Occupied ports
> must fail without terminating other workloads. Keep focused preparation checks
> at `just ui-e2e-bootstrap-test` and `just ui-e2e-app-test`, using the existing
> Node wrapper; these checks are neither the full UI gate nor package evidence.

## Child Coverage And Export Checkpoint (2026-09-11)

- Motivation: The two environment-isolated bootstrap tests also cleared
  `LLVM_PROFILE_FILE`, leaving child profiles outside the collector. Worker
  `7b84dfa0`, replayed as `f766560a`, preserves only that supplied value.
  Tests cover supplied, empty and absent values and removal of stale/unrelated
  entries. Production startup, environment handling and C1 assertions are unchanged.
- Verification: The workspace/all-feature instrumented `bootstrap` target
  passes three tests. Child-only preflight lines 61-66 each record a hit; no
  new default profile appears under the app crate. Earlier profiles are retained,
  not discarded. The first command combined incompatible `--no-report` and
  `--no-clean` options and failed before execution; the corrected command uses
  `--no-report` alone. Both logs are retained.
- Export correction: Pinned cargo-llvm-cov 0.8.7 hides test/example/benchmark
  paths and `*_tests.rs` by default. `just cov-report` disables that default for
  LCOV, HTML and native exports, preserves existing Rust/native separation and
  propagates each command's failure. `just cov` now fails immediately if its
  instrumentation or tests fail. The separate 90% package gates are unchanged;
  adding test lines cannot dilute them. No scanner property or server criterion
  changes, and no new dependency or authored-file exclusion is introduced.
- Evidence: Actual exports increase first-party Rust file records from 242 to
  310, adding 68 and removing zero. Six structural mutations reject missing
  exports, test hiding, changed package gates and masked failures. Four executable
  recipe cases retain success or stop at the exact failed LCOV/HTML/native command.
  The first failure-control draft targeted another recipe's shell setup; its
  failure is retained, and the corrected test targets `cov` specifically.
- Limits: This strengthens local coverage collection, not published Sonar
  coverage or release readiness. Exports retain external native/macro records;
  canonical scanner acceptance remains unverified without `SONAR_TOKEN`.
  The earlier full CI run on `4981eef4` exits zero with eight watcher warnings;
  it does not certify these later child/export changes. UI remains 57 passed,
  one failed and 72 not run at the required scheduling workflow.
- Observability/rollback: Preserve commands, raw profiles, default-filter and
  inclusive reports under `artifacts/media-verification/2026-09-11-c1-e2e-bootstrap/`.
  Revert only this test/export delta to roll back; do not suppress coverage or
  runtime failures. No test media or completed child worktree remains.
- Stale-policy check: Reviewed root, Rust, DevOps, Sonar instructions and the
  approved ADR 586/588 boundary. Rust instructions preserve child instrumentation;
  DevOps and the existing required-checks owner enforce inclusive exports and
  unchanged package gates. Architecture, frozen SQL and approval scope are unchanged.

### Pending Gate Findings

The fixed 48-file source-only secrets scan ran after local inventory validation
and found five hard-coded PostgreSQL password defaults in `just/quality.just`.
They remain required fixes, not accepted test-fixture exceptions. The current
full CI run evaluates the child/export checkpoint; it does not clear this finding.
The documentation link gate also failed with one connection error and 37
timeouts after a network-enabled retry. No scanner or link criterion was changed.

### Test Database Input Correction

The five literal defaults are removed. Database-backed quality recipes now
require either existing caller-supplied database URL, preserve both when distinct,
and propagate the selected values to their database-backed children. `cov` no
longer falls back to the credential-construction helper either. `validate` passes
the resolved environment to its entire unchanged gate sequence in one invocation,
instead of losing assignments between separate recipe shells. This is a local
test-input correction, not a runtime authentication or database-schema change.

The real Just recipes pass 50 isolated mock-command runs and 160 assertions for
absent/empty inputs, URL precedence and failure propagation. Workflow guardrails
and instruction drift pass. The exact four-file follow-up secrets scan passes;
full-file MAIN Ruby analysis reports zero issues. CLI `sonar verify` was rejected
with the organization's agentic-analysis 403, not a clean analysis result. These
focused checks do not replace full CI/UI, canonical Sonar or published coverage.
The test creates no database or media and removes its temporary fixtures. No
dependency, suppression, threshold or production behavior changes; roll back only
the recipe/test correction, retaining the original finding as unresolved.

Stale-policy check: root, DevOps and Sonar instructions were reviewed. DevOps now
forbids credential fallbacks in these recipes, including moving them into helpers,
and requires propagation and missing-input regression coverage. Remaining watcher,
UI, canonical Sonar and documentation-network failures stay visible above.

### Integrated Coverage Result

Full `just ci` against the stable executable source at `8cf7c928` exits zero,
including all 18 unchanged package gates, script coverage and the release build.
Documentation alone changed during execution. Eight config-watcher shutdown
WARNs remain, so this is not warning-free handoff evidence. The exact owned
database `revaer-approved-ci-4d5753ae1397` and anonymous volumes were removed.

The fresh export confirms 310 first-party Rust files, 68 added and none removed
relative to the retained default-filter report. It has 122,233 positive
first-party line records out of 131,114; bootstrap child-only lines 61-66 each
have one hit. Seven external native/macro records remain visible and are not
counted as first-party. The log and raw coverage are retained under
`artifacts/media-verification/2026-09-11-c1-e2e-bootstrap/`.

The later credential fix is `7ae86f2e`; this CI execution does not certify it.
Fresh full `just ui-e2e` on `7ae86f2e` plus documentation-only changes again
reports 57 passed, one failed and 72 not run. The unchanged schedule PATCH
assertion receives 400 `media_profile_filesystem_identity_required`; teardown
also rejects missing UI coverage. The owned database
`revaer-approved-ui-e2e-cb26faa89ef6` and volumes were removed, and canonical
fixture cleanup passes. Canonical Sonar still lacks its token; no published
coverage, native package or remote-check acceptance is established.
The credential-fix worktree was removed after its clean commit was retained
in integration and its focused evidence was archived.

### Final Integrated Checkpoint (2026-09-12)

Full `just ci` on `021656b4` plus the parent validation wiring/report correction
exits zero with all 18 package gates, script coverage and the release build.
Executable source stayed unchanged during the run; only documentation changed.
This run includes the explicit database-input correction and its 50 recipe runs,
but still emits eight watcher-shutdown WARNs. It is not warning-free acceptance.
The owned database `revaer-approved-ci-13d9879c395b` and volumes were removed.

Fresh LCOV contains 310 first-party Rust files, 131,114 first-party line records
and 122,231 positive records. Bootstrap child-only lines 61-66 each have one hit;
all seven external native/macro records remain visible. These are local
coverage measurements, not published Sonar metrics. Raw exports and the log
are retained under `artifacts/media-verification/2026-09-11-d3-validation/`.

The reviewed 52-file `sonar analyze secrets` invocation ran successfully.
Canonical `just sonar-scan` again stopped before analysis for missing
`SONAR_TOKEN`; the per-file checks do not substitute for it. The UI result and
scheduling/root-association failure above remain the latest full UI evidence.
Neither native package qualification nor remote PR/check acceptance is claimed.

### Settings And Attribute Checkpoint (2026-09-12)

Full `just ci` and `just ui-e2e` were rerun on `52c7ef13` plus the reviewed
settings/canonical-wiring delta. Executable files stayed unchanged throughout;
only documentation changed. CI exits zero, including the 18 unchanged package
gates, script coverage and release build. Eight watcher-shutdown WARNs remain.
UI reports 57 passed, one failed and 72 not run at the same required schedule
PATCH: 400 `media_profile_filesystem_identity_required`, followed by missing
UI-coverage rejection during teardown. Neither run establishes a clean handoff.

Fresh Rust LCOV retains 310 first-party files, 131,114 line records and 122,231
positive records, the six child-process hits and all seven other native/macro
records. The new attribute/settings modules have 153/215 and 129/209 covered
line records in the real CI script report; their tests have 211/273 and 169/170.
Uncovered lines remain visible. No unpublished live-run coverage was merged
into the canonical CI reports or substituted for published Sonar metrics.

The owned databases `revaer-approved-ci-51738467ff3a` and
`revaer-approved-ui-e2e-bb2d81729e9b` and their volumes were removed. Full logs,
unmodified coverage, UI results and the disposable gate harness are retained
under `artifacts/media-verification/2026-09-12-d3-setting-paths/`. Canonical
`just sonar-scan` stops before analysis because `SONAR_TOKEN` remains unavailable.
The reviewed per-file Ruby/secrets scans are auxiliary evidence only. D3,
root/scheduling workflow, warning-free shutdown, both native packages and exact
PR checks/reviews remain required; no criteria or production behavior changed.

### Hash-Fill Gate Checkpoint (2026-09-12)

Full `just ci` on clean `9c861f2b` exits zero, including all 18 unchanged package
gates, script coverage and the release build. It still emits eight config-watcher
shutdown WARNs. The prepared-media fixture test remains explicitly ignored in
the ordinary test and coverage runs; these runs do not replace the required
unfiltered conversion gate. Full `just ui-e2e` on the same source exits one:
57 passed, one failed and 72 did not run. Schedule enablement receives 400
`media_profile_filesystem_identity_required` instead of 200; teardown also
rejects missing UI coverage. Neither result establishes a clean handoff.

Unmodified Rust LCOV contains 317 source records: 310 first-party Rust files
and seven external native/macro records. Across all records, 122,294 of 131,488
line records have hits; first-party counts are 122,231 of 131,114. Real script
coverage reports 97/97 lines for the new hash-fill proof and 88/88 for its tests.
The wrapper retains its uncovered lines at 64/158. All raw coverage inputs and
exports remain evidence, not positive published Sonar metrics.

Canonical `just sonar-scan` again stops before analysis for missing
`SONAR_TOKEN`. Full-file Ruby MAIN and exact-file secrets scans are auxiliary
only. The completed gates, raw coverage, UI results and source identities are
retained under `artifacts/media-verification/2026-09-12-d3-hash-fill/`. The first
CI attempt, interrupted after independent review findings with exit 130, remains
separate from this corrected run. Later documentation-only recording does not
extend the tested executable scope or claim a new full-gate pass.

Both owned databases, `revaer-approved-ci-86d6c176dfb7` and
`revaer-approved-ui-e2e-f779fd76c9b2`, were removed with their volumes; exact-name
absence and UI-listener closure were verified. Canonical fixture cleanup passes.
The user's conflicted checkout is preserved. No PR was pushed or merged, no
native Linux package is qualified, and D3 remains incomplete. ADR 589 is a
separate proposed clarification, not an extension of ADR 588's approval.

This is continued validation only: no runtime, dependency, observability or
criteria changes. Root, DevOps and Sonar instructions were reviewed; this
checkpoint removes no rule and resolves no failure by documentation. Rollback
removes only this record; all original failing evidence remains retained.

### Watcher Warning Reproduction (2026-09-12)

- Status: Blocked implementation boundary; this is not a runtime correction.
- Motivation: reproduce the eight actual watcher-abort WARNs in
  `/private/tmp/revaer-integrated-metadata-ci.log` on assigned source
  `2af9c420ff2865a7ad46dc4b7af4be302ab45ee2` before selecting a fix.
- Design finding: bootstrap still aborts an unfinished watcher without joining
  it. Its twelve relevant runtime/test files are unchanged from the prior
  source review at `a19e5856`. The existing injected shutdown channel carries
  only a boolean, not S2's shared immutable deadline/ownership receipt. Native
  configuration returns on enqueue and its worker exposes no join handle.
  An idle-only stop does not settle admitted work; cancelling active preparation
  can leave partial configuration; reusing the per-task grace/unconditional
  join is not ADR 588's approved bounded design. No substitute was implemented.
- Required delta: implement the already-approved S2 ownership/deadline and
  native configuration settlement prerequisites outside this bootstrap-only
  assignment, then connect cooperative watcher cancellation and classified
  settlement. This requests no new architecture and does not touch ADR 589.
- Test coverage: an offline, warnings-denied, all-feature app test build through
  Just succeeded on Darwin arm64/Rust 1.96.0. The four existing bootstrap
  bind-failure tests ran twice through Just with their real child processes:
  eight successful invocations, zero skips, and exactly eight matching WARNs.
  This adds current runtime reproduction to the prior source-only finding;
  it is neither a fixed result nor a repeat of full CI/coverage/UI.
- Evidence: ignored `target/watcher-graceful-stop/` retains the exact original
  CI log, original line mapping, reproduction driver, per-test commands/raw
  stdout/stderr, source/binary hashes, build evidence and cleanup result.
- Cleanup: pinned PostgreSQL image `57c72fd2a128...ffc07777`, resolved arm64
  image `7e7dbab8d3b4...cdefd`; unique container
  `revaer-watcher-graceful-ab6a05b60270` removed with volumes, exact-name absence
  verified, and loopback port 57959 successfully rebound. No test media acquired
  or generated; existing test temporary-workspace cleanup remained in use.
- Observability/dependencies: unchanged. All eight warnings remain unresolved;
  no suppression, timing, criteria, production, test or dependency edit occurred.
- Risk/rollback: the known unobserved shutdown remains; rollback removes this
  documentation checkpoint only. Parent owns integration and both full gates.
- Stale-policy check: reviewed root AGENTS, scoped Rust/data/DevOps guidance,
  ADR 559 G1, ADR 577's later S2 resolution, ADR 586, and ADR 588's actual
  approval/S2/LIFE-1 bounds. The approved shutdown appendices differ from reviewed
  `9575c077` only by dated approval banners. Historical held wording is not a
  renewed approval hold. No policy contradiction or rule was edited away.

### Shared Shutdown Authority Prerequisite (2026-09-12)

- Implementation status: production-used prerequisite, focused verification
  passed under ADR 588 S2/LIFE-1 and the explicit continuation scope. Full S2
  and the parent's combined gates remain incomplete.
- Motivation: replace boolean-only shutdown with production-used immutable
  origin/absolute-deadline authority before implementing downstream settlement.
- Design: one private publisher retains the first observed monotonic origin;
  request publication atomically clamps the shared deadline to the earliest
  supplied bound and original 30-second application budget. Duplicate/later
  requests coalesce; expiry cannot be revived. Owner drop publishes the same
  stop authority before closing the channel, including when no receivers remain.
  Existing runtime receivers retain their cooperative stop interface.
- Production integration: bootstrap requests the shared authority immediately
  after serving returns, before any existing background-task shutdown await.
  All three sequential media-task grace waits observe that same absolute
  authority, including shortening while pending; no per-task budget restarts.
- Observability: preserve existing grace-expiry WARN and actual JoinError
  classification, including WARN for panic/external cancellation. Unexpected
  loss of deadline observation is a distinct WARN, not a successful timeout.
  The config watcher abort/drop path and its warning guard remain unchanged.
- Remaining coupling: immediate-abort joins and post-grace abort joins retain
  their existing settlement behavior. This does not independently bound
  non-yielding work, logging, Tokio teardown or native termination. PID1's exact
  28/30-second phases, native enforcement, configuration acknowledgements,
  listener/worker ownership and recovery still require their approved owners.
  ADR 589 and every existing duration/quality criterion remain unchanged.
- Test coverage: focused shared-authority and bootstrap regressions cover first
  origin, deadline clipping, duplicate/concurrent requests, irrevocable expiry,
  owner loss, observer retention, pending-wait shortening, sequential media
  waits without replenishment, and unchanged S1 warning/cleanup assertions.
  Final warnings-denied Just runs of `bootstrap::shutdown_tests` plus
  `runtime_shutdown::tests` passed 25 tests each with all features and no default
  features, zero failures/ignored tests. The canonical shutdown recipe also
  passed its 15 bootstrap cases in both configurations. All four strict
  `just lint-runtime-shutdown` invocations and `just fmt` passed. Deliberate
  panic hooks and failure-path WARNs remain visible and asserted. Initial lint
  failure on two test-only style issues is retained separately from the corrected
  pass; no rule or expected event was relaxed.
- Risk/rollback: this removes redundant cooperative waiting without certifying
  whole-process quiescence. Revert this prerequisite as one patch if required;
  never represent the prior per-task waits as accepted aggregate enforcement.
- Dependency rationale: existing Tokio watch/time and standard library only;
  no dependency, runtime tuning, native control, database or media change.
- Stale-policy check: existing root/Rust and ADR 588 S2/LIFE-1 authority remain
  applicable; the matching Rust instruction now records this narrow invariant
  and its qualification limits. No criterion was weakened or approval inferred.
  Parent retains integration, D3 and the combined full CI/UI gates. Prior
  reproduction evidence remains separate under `target/watcher-graceful-stop/`.
- Evidence/cleanup: `target/shutdown-authority/` retains commands, full logs,
  the exact staged delta from `2d8e7c2d`, source/test-binary hashes, tool/host
  identity and native build evidence. Its owned build directory was removed
  after verification; no database, service, test media or remote operation was
  created for these focused tests. The prior 80-file evidence seal still matches.

### Helper And Shutdown Integration Checkpoint (2026-09-12)

- Source: database proof at clean `60357fbf`; combined CI/UI at clean
  `c139ab9c`. The intervening shutdown commits do not change database proof
  sources. The final checkpoint is documentation only.
- Database: 4,827/4,828 canonical checks pass. Only the explicit incomplete-D3
  guard fails. The 99 paired wrapper cases pass 2,038 checks, including all
  three two-sample modes. The 24 metadata variants pass all 37 matrix checks.
  Focused harness suites pass 13,496 assertions. These are bounded results,
  not complete helper/native closure or single-init cutover qualification.
- Combined `just ci` exits zero with the release build and positive coverage,
  but retains eight config-watcher-abort WARNs. It is not a clean handoff.
  Rust LCOV contains 318 records, 122,596 covered of 131,783 lines; 311 tracked
  first-party Rust files account for 122,533 of 131,409 lines. All 53 raw Rust
  instrumentation inputs and real script/JavaScript coverage are retained.
- Combined `just ui-e2e` exits one: 57 passed, one failed, 72 did not run.
  The schedule PATCH still returns `media_profile_filesystem_identity_required`;
  teardown also rejects missing UI coverage. No expectation or coverage gate
  was relaxed. The approved coordinated cutover remains required.
- Canonical Sonar remains unavailable because its token is absent. The
  previously denied upload was not retried, and no published-coverage or
  current-source Sonar pass is claimed. Native packages and PR acceptance
  remain unqualified; no push or merge occurred.
- Evidence: `artifacts/media-verification/2026-09-12-helper-order/` retains the
  exact reports, logs, code bundle, worker evidence and coverage. Both workers
  are closed and their clean worktrees removed. Owned database containers and
  volumes were removed, exact container absence and the UI listener's release
  verified, and canonical test-media cleanup passed. User checkout changes
  remain untouched. The local code boundary is 812/9,999 changed lines;
  this is not an actual remote PR base/head qualification.
- Documentation build exits zero but warns that its search index is about
  15 MB. This warning remains visible; documentation restructuring is not
  included in this runtime/proof increment.
- Scope/policy: root, Rust, data and DevOps instructions remain binding.
  This checkpoint changes no architecture, dependency, runtime behavior,
  observability or acceptance criterion; rollback removes only this record.

### Runtime Configuration ACK Prerequisite (2026-09-12)

- Implementation status: production-used ACK prerequisite on base `24723ee7`,
  within approved ADR 588 S2. The parent delegated command/adapter/worker edits;
  that delegation is not a separate operator architectural approval.
  Parent retains app orchestration/publication, D3 and the combined full gates.
- Motivation: `apply_runtime_config` previously returned enqueue success before
  `Worker::handle_apply_config` ran, so its caller could not observe application
  failure or completion.
- Design: `ApplyConfig` now carries the existing Tokio oneshot reply pattern.
  The adapter waits for the completed session application and alternate-speed
  reconciliation result. Original `TorrentError` variants, operation/identity
  fields and boxed typed sources move unchanged to the caller. Subsequent alert
  polling is independent of that completed application result.
- Observability: the worker retains its existing command-failure WARN and
  degraded-health reporting before moving an error into the reply. The failed
  application still skips the immediate event flush, preserving its health
  state. The existing closed-response WARN remains unchanged, including when
  the abandoned caller can no longer receive an application failure.
- Cancellation boundary: cancellation before enqueue leaves no command;
  dropping a caller after enqueue does not cancel admitted work. A command or
  reply channel failure is typed as transport failure, never enqueue success or
  proof of rollback/non-execution. This does not implement S2's separate queued
  `not_started_shutdown` classification or close shutdown admission.
- Test coverage: 11 new focused tests plus three existing alternate-speed and
  piece-deadline regressions pass in both all-features and no-default-features
  configurations: 14 passed each, zero failures/ignored tests. Tests exercise
  adapter-to-worker dispatch, separate pending application/reconciliation
  phases, typed early/late errors, closed channels, cancellation before/after
  enqueue, dropped receivers, retained health and independent poll failure.
  Four scoped Just Clippy passes (all targets and panic-free production targets
  in both feature configurations), `just fmt` and `git diff --check` pass.
  Initial test-only lint failures and the corrected pass remain separate;
  no assertion or lint criterion was weakened.
- Limits: all-features builds the local arm64 libtorrent 2.1.1 native backend,
  but these focused runtime tests inject sessions and channels. They do not
  qualify native atomic rollback, command admission budgets, owner/PID1 or
  native settlement, app applied-revision publication, Linux packages, complete
  S2, or the original watcher WARNs. No timeout, duration, dependency, runtime
  tuning or ADR 589 REGISTER_HASH behavior changed. Full CI/UI remains with the
  parent and was not duplicated here.
- Risk/rollback: callers now observe real failures and wait for completion;
  this adds no independent deadline. A later reconciliation failure may follow
  already completed changes. Revert the command/adapter/worker ACK together if
  necessary; the previous enqueue-only response is not settlement evidence.
- Dependency rationale: existing Tokio oneshot and standard-library test
  controls only. No new dependency or infrastructure collaborator.
- Stale-policy check: root, Rust and FFI instructions reviewed; the matching
  FFI instruction now records the ACK invariant and its limits. Actual ADR 588
  approval and its active-apply acknowledgement clause remain the authority;
  no approval, exception or new architecture was inferred.
- Evidence/cleanup: `target/config-ack/evidence/` retains exact commands, raw
  initial/final logs, tested deltas, source/test-binary/native-library hashes and
  tool/host identity. The worktree and shared focused build cache are retained
  for parent review and parent cleanup. No DB, service, listener, test media,
  remote operation or upload was created; no build output was deleted. Original
  watcher reproduction remains separately sealed at the parent's
  `artifacts/media-verification/2026-09-12-helper-order/watcher-reproduction-2d8e7c2d.tar.gz`.

Parent integration at `ccf6f7ff` reviewed the actual error/health and response
paths, corrected the delegation-versus-approval wording, and refreshed the
documentation index. Final executable `be1a2a53` includes only subsequent D3
proof corrections. Full CI exits zero with eight original watcher warnings;
full UI/API remains 57 passed, one schedule-PATCH failure and 72 not run, with
missing UI coverage rejected. Native/PID1 settlement, queued shutdown outcomes
and app revision publication remain unfinished; ACK is not their substitute.
The single worker was closed after delivery, all 24 evidence checksums verified,
and its clean worktree removed. Evidence is retained under
`artifacts/media-verification/2026-09-12-size-config/`. No GitHub mutation or
upload occurred, and no gate, deadline or approval condition was weakened.

### Completed Engine-Profile Publication Boundary (2026-09-12)

- Status: bounded production change and focused verification complete on base
  `a8133fdf`. Authority is operator-approved ADR 588 S2's actual-acknowledgement
  and no-publication-after-partial-application requirements, not delegation or
  an inferred new architectural decision. Parent retains combined full gates.
- Motivation/reproduction: four strengthened existing tests use genuinely
  distinct old/attempted profiles, including changed rate limits. Before the
  production change, tracker preparation, proxy preparation, engine application
  and limit-update failures all left the attempted profile published. All four
  original failing assertions and their exact base/test delta are retained.
- Caller boundary: live startup awaits `spawn_libtorrent_orchestrator` before
  spawning the configuration watcher (`bootstrap.rs:368`, `:380`); the watcher
  awaits each `apply_config_snapshot` (`:747`) before taking another snapshot.
  No independent production blocklist-refresh caller exists. Its metadata
  changes can trigger later watcher snapshots, but do not launch overlapping
  refresh calls. No new lock, queue, generation or cancellation policy was added.
- Design: `UpdateLimits` now uses the existing oneshot ACK pattern for global
  and per-torrent callers. The reply carries the completed handler result,
  including global alternate-speed reconciliation, with original typed native
  errors and target identity intact. The app publishes `engine_profile` only
  after preparation, completed engine application and completed global limits
  succeed. Blocklist metadata preparation now receives the current candidate
  explicitly, rather than relying on prematurely published state.
- Observability: handler errors retain worker-origin command WARN and degraded
  health reporting before their result moves to the reply, including receiver
  loss. Failed handlers retain the skipped event flush; successful replies still
  precede independent polling. Later polling failure cannot replace a completed
  handler result. Existing log levels, warning text and quality criteria remain.
- Cancellation: a dropped pre-admission caller leaves no command; after enqueue,
  receiver loss does not cancel admitted work. Channel closure is typed transport
  failure, not success or proof of non-execution. Pending/cancelled app waits
  retain the prior profile even when earlier mutations have completed.
- Verification: focused Just runs pass 17 app tests and 22 libtorrent tests with
  all features, plus 22 libtorrent tests without default features. Four scoped
  Clippy passes cover both crates' all-target and panic-free production surfaces
  in both feature configurations. Formatting and patch whitespace checks pass.
  Existing app orchestrator feature gating means two initial minimal-feature
  probes selected zero tests; neither is counted as validation. A test-only
  read-guard type error and two sandbox-denied local-listener tests are retained
  separately from their corrected/authorized passes. No assertion was weakened.
- Test scope: real adapter/worker dispatch with injected sessions; pending
  application/limits/publication, original preparation/apply/limit failures,
  typed global/per-torrent native errors and NotFound, early/late cancellation,
  lost receivers, global reconciliation success/failure, event-flush isolation,
  retained health, candidate-bound metadata and the existing cache regressions.
  Local arm64 libtorrent 2.1.1 compiled; this is not native/package qualification.
  These runs are not instrumented coverage and make no new coverage claim.
- Limits/risk: the field boundary relies on the existing serial refresh callers,
  not arbitrary concurrent calls. Manual rate overrides and persisted settings
  remain independently mutable. Best-effort metadata/cache preparation can still
  have side effects before a later failure; no DB or revision-wide atomicity,
  filesystem-policy rollback, native rollback/settlement, owner/PID1 closure,
  queued shutdown classification, watcher-WARN fix or S2 completion is claimed.
  Waiting callers now observe actual limit failures and latency, without a new
  timeout, duration, dependency, numeric budget or REGISTER_HASH change.
- Rollback: revert the limit ACK and app publication change together; do not
  present restored enqueue-only success as completion evidence.
- Dependency/stale-policy check: existing Tokio oneshots and standard-library
  test controls only. Root, Rust and FFI instructions and actual ADR 588 approval
  were reviewed; matching Rust/FFI guidance records this narrow invariant and
  its remaining limits. No approval, exception or criteria relaxation was added.
- Evidence/cleanup: `target/config-publication/evidence/` retains exact commands,
  raw failed/final logs, source deltas, base identities, binary/native hashes and
  host/tool identity. One owned build cache was reused throughout and retained
  with the clean worktree for parent review. The two mock listeners used owned
  `127.0.0.1:0` binds and ended with their test process. No DB or test media was
  created; `.server_root` remains absent. No old/parent checkout was edited, no
  agents were launched, and no full CI/UI, upload, remote mutation or media
  download was performed. Parent-owned D3/scripts, ADR 569 and indexes are untouched.

### Combined Configuration And Disambiguation Gates (2026-09-13)

Parent reviewed `396c87fc` and integrated its unchanged runtime/test changes as
`749ba0b4`, with the database result and generated index. Full `just ci` on that
clean executable revision exits 1: `deadline_includes_pipe_setup_time` retains
the expected 20 ms primary deadline but adds `output pipes remained open after
process-group cleanup`; 323 other media-runtime tests pass. Four watcher WARNs
also remain. Source inspection confirms this diagnostic records unproven pipe
closure at return, not permission to discard secondary evidence. No assertion,
deadline, cleanup policy or native implementation was changed to hide it. The
approved S2 settlement work remains necessary. CI stops before minimal-feature
tests, Rust coverage and release build; prior coverage is not current evidence.

Full `just ui-e2e` on the same revision has 57 passed, one failed and 72 not run.
The unchanged schedule PATCH requires 200 and receives 400 with
`media_profile_filesystem_identity_required`; teardown rejects missing UI route
coverage. Actual partial Playwright LCOV/V8 data is retained, not presented as a
completed UI run or published Sonar coverage. Canonical Sonar token presence was
false; no refused upload was retried. Documentation builds with the existing
15,114,563-byte search-index WARN, not a clean documentation gate.

Database canonical proof at `c9a25f31` passes 6,049/6,050 checks, failing only
incomplete D3. All 37 recorded disambiguation proof inputs still match the
integrated revision. Focused Ruby suites pass 14,621 assertions, including 235
new family assertions; 24 live variants preserve 48 actual unique-key failures
and 24 successful no-rule fixtures. Neither these errors nor the passing local
configuration tests constitute native, package or feature completion.

Evidence and source bundle are retained under
`artifacts/media-verification/2026-09-13-config-disambiguation/`. The worker was
closed, its clean worktree/cache removed, and all 36 worker evidence checksums
verified. The user checkout and its existing conflicts were preserved. No new
architecture, criteria exception, GitHub mutation, upload or merge occurred.

### Native Pipe-Closure Verification (In Progress)

- Motivation: retained `749ba0b4` CI reports unclosed output pipes after the
  20 ms deadline. Cleanup's completion predicate checks only leader reaping and
  process-group absence, allowing it to skip its existing forced verification
  while pipe closure remains unobserved.
- Authority/design: ADR 501, already Accepted at operator-reviewed `9575c077`,
  authorizes the shared supervisor, bounded inherited-pipe cleanup and unchanged
  five-second grace with primary/secondary failure preservation. Existing
  `system.rs` already requires pipe EOF and reports its absence. Require both
  captures to reach EOF in cleanup's existing completion predicate; retain
  graceful/forced phases, polling interval, budgets, deadline classification
  and final unclosed-pipe diagnostic. Do not signal a group already observed
  absent just because its pipes need observation. No new S2 owner or deadline.
- Test evidence: two new real-pipe regressions fail on `ac094da9` plus the
  retained test-only delta: exited-group cleanup returns before EOF and before
  exhausting available verification steps. Tests control injected process
  observations and pipe-writer lifetime; no child or media is created. Original
  failing CI, new failures and exact source delta are retained. Corrected
  focused verification passes all 41 process tests, including the unchanged
  setup-time deadline test, and strict all-target/production Clippy passes.
  Formatting, instruction drift and patch whitespace checks pass. Full gates
  remain pending; these focused results are not package or coverage evidence.
- Observability/risk: read errors, deadline and unclosed-pipe evidence remain
  intact. This addresses an exposed cleanup gap, not every OS scheduling detail
  in the original CI failure or full native/PID1/recovery qualification. Roll
  back the predicate and matching signal guard together if necessary, retaining
  failing evidence and the release hold instead of relaxing assertions.
- Dependency/stale-policy check: existing standard-library pipes and descriptor
  conversions only; no dependency or manifest change. Root, Rust instructions,
  ADR 501, ADR 559 G1 and approved ADR 588 S2 were reviewed. Matching Rust guidance
  references existing policy; no approval, threshold or exception is added.

### Pipe-Closure Full-Gate Checkpoint (2026-09-13)

Full `just ci` at clean `132a7fbc` exits zero. All 327 media-runtime tests pass
in ordinary and instrumented runs, including the unchanged setup-time deadline
assertion. All 18 package coverage gates pass; fresh Rust LCOV, native text,
52 raw profiles, merged profile data and real script coverage are retained.
The release build succeeds. Eight existing configuration-watcher shutdown
WARNs remain, so this is not clean handoff or complete S2 evidence.

Full `just ui-e2e` on the same revision exits one: 57 passed, one failed and
72 not run. Schedule enablement still receives 400 with
`media_profile_filesystem_identity_required` instead of the required 200, and
teardown rejects missing UI coverage. Actual partial JS coverage is preserved.
Canonical `SONAR_TOKEN` remains absent; no refused upload was retried and no
published coverage or remote acceptance is claimed.

Documentation builds successfully but retains its oversized search-index WARN;
that warning is not a clean documentation result or a reason to disable indexing.

The sole database subagent completed reviewed sidecar `bbf50836`: 208 unit
assertions and 306 bounded live checks establish that internal RI callbacks
cannot be counted by the selected PostgreSQL function-statistics mechanism.
Its guard rejects substituted callback identities; NULL is not zero invocations.
The failed attempts, real FK counterexamples, final archive and original
`8797ae00` archive are retained with 460 verified manifest entries. The parent
corrected delegation-versus-operator-approval wording before closing the agent.
That commit is ready for the next integration, not part of the frozen `132a7fbc`
full-gate source. No second profiler or duplicate full gate was launched.

Evidence is retained at `artifacts/media-verification/2026-09-13-pipe-eof/`.
Both exact parent database names and the UI listener were confirmed absent,
managed test-media cleanup passed, and the completed clean worker worktree was
removed. The user checkout and conflicts remain untouched. No new architectural
approval, criteria relaxation, GitHub mutation, upload or merge occurred.

### Owned Configuration-Watcher Shutdown (2026-09-13)

- Status: bounded implementation and focused verification; parent integration
  and full gates remain pending. Assigned clean base `0ef147c0`, isolated branch
  `work/media3-config-owned-shutdown`. This is parent delegation under actual
  ADR 588 S2/LIFE-1 approval at reviewed `9575c077`, not new operator approval.
  ADR 559 G1 and the exact approved appendices remain binding; ADR 589 remains
  Proposed with no REGISTER_HASH field/deadline change.
- Motivation: replace bootstrap's unconditional idle-watcher abort/drop path
  after the shared-authority, completed ApplyConfig/UpdateLimits ACK and old
  engine-profile publication prerequisites. The eight actual watcher WARNs in
  the retained `132a7fbc` full-CI evidence are historical counterevidence, not
  erased or represented as retested by this database-free assignment.
- Design: bootstrap constructs its existing shared shutdown channel before
  spawning the watcher and passes the same receiver authority to media tasks.
  It requests drain after serving returns, before any background wait. The
  watcher prioritizes drain over its next-update future and rechecks the latch
  after that future resolves. This final check is snapshot admission; an already
  admitted snapshot retains its serial, awaited application and limits result.
  No second refresh or queue is introduced. Existing apply/health reporting and
  the old-profile publication boundary remain unchanged.
- Owned join: bootstrap retains the watcher handle and waits cooperatively
  against the same dynamically shortening absolute deadline. Expiry requests
  abort and polls the join once without waiting again; a pending join remains
  explicitly unconfirmed. The handle stays with bootstrap until scope teardown.
  Neither abort nor that later handle drop is reported as completed settlement.
  Actual successful, cancelled and panicked joins retain their real outcomes.
  `runtime_shutdown.rs`, its first origin and the 30-second budget are unchanged;
  there is no new per-task grace or unbounded watcher join.
- Observability: retain the original watcher-abort WARN, now with
  `reason=shared_shutdown_deadline`; add WARN for settlement still unconfirmed
  after the abort request. Genuine external cancellation/panic remains WARN;
  only an observed locally requested cancellation uses the existing INFO
  classification. Authority-observation failure remains WARN. No failure message
  or application acknowledgement is weakened to remove the original warning.
- Test coverage: `just test-runtime-shutdown` exercises idle owned join,
  prelatched drain, a ready-update/drain race, drain during the next future's
  poll, no subsequent admission, completed in-flight success/failure, live
  deadline shortening and expiry without replenishment, abort without a second
  wait, and genuine join success/cancellation/panic. Four libtorrent-feature
  cases run the real `apply_config_snapshot` and orchestrator against injected
  engine application/limits replies, checking pending acknowledgements, retained
  failure/health events and no false success before cooperative exit. The
  all-feature and no-default-feature suites pass 29 and 25 tests respectively,
  with zero failures/ignored tests. All four `just lint-runtime-shutdown`
  invocations pass; final formatting, instruction drift and whitespace checks
  are retained with the focused evidence.
- First-failure history: retain the two original bootstrap-wiring failures;
  the initial lending-closure Send compilation failure; test failures caused by
  an intentionally dropped observer and by assuming the existing generic
  AppError Display exposed its typed source; and the first strict lint failure.
  The final helper accepts one next/apply future at a time. The 18,920-byte
  composed future uses the existing `Box::pin` pattern, and test helpers are
  smaller and Send-compatible. The corrected event assertion is exact for the
  existing generic text; controlled apply/limits failures still require WARN,
  degraded health and absence of a successful acknowledgement. No timer was
  increased, assertion disabled, warning suppressed or criterion relaxed.
- Limitations: these are injected-future/collaborator tests, not a live database
  listener or full bootstrap reproduction. Local arm64 libtorrent 2.1.1 compiles,
  but no native operation, Linux package, coverage or PID1 qualification follows.
  Listener/pool closure, native worker/session/destructor settlement, queued
  command shutdown outcomes, blocking work, startup-error teardown, revision-wide
  atomicity, independent PID1 enforcement and recovery remain explicit S2 work.
  A forced watcher abort can abandon observation of admitted native work; that
  is unconfirmed settlement, never cancellation/rollback/success of that work.
  Other runtime post-abort joins retain their prior behavior. Parent owns D3 and
  the final integrated `just ci`/`just ui-e2e`; neither ran here.
- Risk and rollback: this closes watcher admission and allows genuine
  cooperative exit, without bounding a blocked executor, logger or destructor.
  Revert the watcher/channel-wiring/test change together if necessary; preserve
  the failure evidence and do not relabel the prior abort/drop path as clean.
- Dependency rationale: existing standard-library futures/ControlFlow, Tokio
  watch/time/task handles and test oneshots only. No manifest, dependency,
  architecture, numerical limit, queue, parallel refresh or gate change.
- Stale-policy check: root AGENTS, scoped Rust/FFI instructions, ADR 559 G1,
  ADR 588 approval resolution and exact S2/LIFE-1 appendices were reviewed before
  code. Their dated approval banners do not reopen historical design holds.
  Matching Rust guidance now states this watcher invariant and remaining S2
  obligations; no policy contradiction was removed by relaxing a rule. Parent
  checkout, database evidence/scripts, ADR 569, ledgers and indexes are untouched.
- Evidence and cleanup: bounded archive outside the worktree at
  `/private/tmp/revaer-config-owned-shutdown-evidence.tar.gz` retains source
  snapshots/deltas, exact commands, first-failure/corrected logs, tool/native
  identities and cleanup receipts. No database, container, listener, media,
  credential/private-environment read, Node, browser, remote operation, upload
  or new agent was needed. Test-owned temporary directories and task handles
  are closed; owned build output is removed after evidence sealing, leaving the
  committed worktree clean for parent integration/removal.

### Sampling And Owned-Watcher Integration (2026-09-13)

Parent reviewed `8c4b95be` and integrated it as `fe0948a4`, following sampling
proof `f5981931` and the native-counter limitation `0ef147c0`. No runtime SQL,
deadline, approval or quality criterion changed. The worker is closed, its
clean worktree removed, and all 203 archived evidence entries verified.

On clean executable revision `fe0948a4`, full `just ci` exits zero with no
WARN or compiler-warning lines. The prior eight watcher-abort warnings no
longer occur. All 18 package coverage gates pass, and fresh Rust/native reports,
raw profiles, merged profiles and script coverage are retained. This is an
integrated cooperative-shutdown result, not complete S2/native containment.

Full `just ui-e2e` on the same revision exits one: 57 passed, one failed and
72 not run. The unchanged schedule-PATCH assertion requires 200 but receives
400, `media_profile_filesystem_identity_required`, at `media.spec.ts:484`.
Teardown also rejects absent UI coverage. Partial JS coverage is retained,
not presented as complete route coverage. Both gates use separately owned
pinned disposable databases; each removed its container and anonymous volumes.
Test-media cleanup passed. Canonical Sonar authentication remains absent;
no refused upload was retried or positive published coverage claimed.

The bounded evidence is under
`artifacts/media-verification/2026-09-13-sampling-shutdown/`: exact-source
database proof, worker archive, full CI/UI logs, Rust/script and partial JS
coverage, first-failure history and cleanup checks. Scheduling/root binding,
complete D3, broader S2, both native packages, Sonar and remote review/check
qualification remain open. No GitHub mutation, push, merge or release occurred.
Documentation indexing and instruction-drift checks pass. The documentation
build exits zero but retains its oversized-search-index WARN (15,228,470 bytes);
it is not a warning-free documentation result or permission to exclude ADRs.

### Coverage Retention And Deadline Fixture (2026-09-13)

- Motivation: the preceding CI rerun erased the partial E2E JS evidence because
  `cov-report` cleared the entire coverage directory. Its failed JS archive is
  retained and remains an evidence gap. The same CI checkpoint first failed
  because native group force-kill returned EPERM after the deadline.
- Implementation: Rust report generation now replaces only its LCOV, native
  text and HTML outputs. It rejects a symlinked coverage root and preserves
  independent coverage and native analyzer inputs. All generator errors remain
  fatal; retained partial or stale inputs cannot qualify a later Sonar scan.
  The actual recipe is exercised with isolated, explicitly synthetic generators
  for success, repetition, each failed format and filesystem boundaries.
- Native evidence: a bounded Darwin arm64 syscall experiment observed EPERM
  for 10/10 unreaped zombie groups, versus successful KILL for 10/10 live groups;
  163/200 deadline-order cases also returned EPERM. Every child was reaped and
  its pipes reached EOF. This Ruby syscall model is not an instrumented Rust
  reproduction or a reason to suppress a native error.
- Test-only refinement: a TERM-ignoring fixture emits readiness before the
  existing setup delay completes. The 20ms request, 40ms setup/grace and exact
  deadline, empty-secondary and elapsed-time assertions remain unchanged.
  Added safe-pipe/readiness tests reject missing, malformed and EOF markers;
  an injected cleanup regression preserves exact EPERM secondary evidence after
  successful reap, group disappearance and pipe EOF. Production code is intact.
- Focused evidence: the recipe regression passes 9 runs and 94 assertions.
  The worker reports 44 process tests, 20/20 deadline repetitions and strict
  all-feature lint/format checks passing. Parent reviewed its bounded patch;
  integrated full CI/UI and Sonar input validation are still pending here.
- Authority: these are internal test/evidence-retention refinements, not new
  architecture or relaxed criteria. The worker report incorrectly describes
  parent delegation as an operator request; no additional operator approval
  occurred. Preserve that report with this correction rather than treating the
  agent's wording as approval evidence. Existing D3/S2 and release obligations
  remain open; no SQL, GUC, runtime deadline or scanner criteria changed.
- Observability: preserve first-failure, native experiment, focused and final
  gate logs separately. No production logging or error disposition changes.
- Risk and rollback: integration must prove independent inputs survive real
  regeneration and remain source-current before submission. Revert recipe,
  tests and matching guidance together if ownership is incorrect; do not erase
  evidence or waive required inputs. Revert only the test fixture if its native
  assumptions fail on a supported target; keep the original failure recorded.
- Dependency rationale: existing Ruby standard library, Just, Rust standard
  library and rustix only. No new dependency, unsafe code or native adapter.
- Stale-policy check: root AGENTS and scoped Rust, DevOps and Sonar guidance
  were reviewed. DevOps and Sonar now document exact report ownership and
  current-source validation. No contradiction was resolved by weakening policy.

The first integrated `just ci` on `d08598ea` stopped in policy: its exact parsed
recipe contract still required the destructive whole-directory reset. The
guardrail now requires the narrower cleanup and symlink rejection, with four
additional negative fixtures. Existing report commands, inclusive scope,
failure propagation and 90% per-package coverage contracts are unchanged.
Retain this first failure separately from the subsequent integrated run.

On clean executable revision `9b988225`, the corrected workflow regression and
full `just ci` pass. All 18 package coverage thresholds pass; ANSI-normalized
CI output has no WARN or compiler-warning lines. The test-only deadline and
permission-error regressions pass in the integrated suites. Full `just ui-e2e`
still fails: 57 passed, one failed, 72 not run, plus the missing-UI-coverage
teardown error. Schedule PATCH still requires 200 and receives the unchanged
filesystem-identity-required 400. No assertion or route gate was weakened.

Fresh native inputs and executed release JS coverage generation pass. Real
Rust report regeneration preserves all 494 independent input/evidence entries
byte-for-byte, including seven required JS/script/native paths. The subsequent
`just sonar-verify-inputs` passes. Rust LCOV contains 320 sources, 133,355 line
records and 124,171 positive records; JS LCOV contains 61 sources, 5,402 line
records and 4,249 positive records. JS includes the failed E2E run and is not
complete UI route coverage. Input validity and positive local counts are not
published Sonar metrics, successful analysis or release acceptance.

The canonical Sonar token remains absent; no scanner or external upload ran.
The earlier refused transfer was not retried. D3/cutover, functional root and
schedule binding, complete E2E, broader S2, both native packages and remote
stack/review/check acceptance remain open. No GitHub mutation or source push
occurred. The completed worker was closed and its worktree removed only after
its patch matched committed parent files and its archive/manifests verified.
Each gate removed its owned database and anonymous volumes. Exact-source logs,
all coverage outputs, 53 raw/merged LLVM profiles and the native experiment are
retained under `artifacts/media-verification/2026-09-13-coverage-retention/`.

### Deterministic Cleanup Fixtures (2026-09-13)

Motivation: full CI at `e1925839` reproduced
`exited_group_waits_for_pipe_eof_within_forced_verification` with 334 observations
instead of eight. An isolated worker reproduced 340/eight during concurrent
spawns. Test pipe writers can be inherited during macOS's separate pipe/CLOEXEC
setup. Shutdown-controlled Unix socketpairs now make the fake-operation fixture
deliver EOF at the specified observation, including when writer copies remain.
A new regression still requires both independent EOFs and exact stdout/stderr.
No production operation, verification phase, deadline, error or assertion was
weakened. Seven cleanup tests passed 100 focused repetitions on macOS.

The worker's concurrent sweep also observed the real-pipe
`deadline_includes_pipe_setup_time` test retaining an unclosed pipe. The holder
was not traced; inter-test inheritance is a hypothesis, not an established
production defect. This exact test now runs in an isolated test subprocess,
retaining real pipes, its real shell, the 20 ms deadline, setup-time accounting,
cleanup assertions and inherited coverage environment. Seventeen post-change
focused repetitions and both strict lint passes succeeded. Full combined gates
remain pending; these results are not Linux package or broader S2 qualification.

Observability is test output only. The existing standard library supplies the
socketpairs and child runner; no dependency is added. Rollback removes the
test-only changes and new regression together, with production behavior intact.
The root and Rust instructions were reviewed; no criteria or approval drift was
introduced. Parent validation must retain the initial failure and every remaining
failure rather than treating repetition counts as release acceptance.
