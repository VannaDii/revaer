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
