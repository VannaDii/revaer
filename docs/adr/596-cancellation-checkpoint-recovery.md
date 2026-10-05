# Cancellation and checkpoint recovery

- Status: Recorded
- Date: 2026-10-04
- Operator approval: Existing ADRs 593–594 authorize the implementation. The
  physical fingerprint refresh was explicitly approved on 2026-10-04 with
  "Make it so in the simplest safe way possible."
- Supersedes: None. ADRs 593–594 remain authoritative.
- Implementation status: Cancellation/checkpoint recovery and ordinary automatic
  activation are implemented. Step 5 requires focused recovery, full CI and normal
  UI E2E on the final unchanged commit. Exact results are retained in the closeout
  evidence described below. Remaining strict Sonar and package qualification
  belong to goal 7; this record does not claim release readiness.

## Context and decision

Shutdown previously followed failure handling, while unfinished work lacked a
usable durable step boundary. Implement the approved single-owner recovery
workflow: stop and join old work, retain completed checkpoints, rebuild required
missing/corrupt temporary outputs, and restart the unfinished step on the same
attempt. Explicit cancellation remains terminal. No supervisor, lease, takeover
or distributed coordination protocol is introduced.

Content, size, path, attempt and configuration/root bindings remain immutable.
Committed rollback restores the original from a private temporary copy and keeps
its backup and manifest. After verifying the aggregate digest and size against the
original job, the guarded stored procedure refreshes only identity and timestamps
on that existing job. Cleanup follows durable persistence. Interrupted or failed
refresh leaves the original recovery artifacts for retry. No recovery checkpoint
table or new coordination mechanism is introduced.

## Task Record

- Motivation: Make service interruption resumable without consuming a failure
  retry, while preserving explicit cancellation and safe final replacement.
- Design notes: Runtime shutdown and cancellation have separate control paths.
  Inspection, execution and verification join before persistence and replay.
  Startup recovery is bounded and follows root-lock acquisition and replacement
  reconciliation. Normalized stored procedures persist completed steps and
  outcomes; application database calls use typed data-crate adapters. Checkpoints
  validate the compiled prefix, owned path, byte count and digest. Invalid required
  outputs rebuild from an earlier usable checkpoint or the unchanged source.
  Private workspaces retain existing attempt keys; inherited root locks keep
  descendant writers inside the ownership boundary. Fresh capacity observations
  append to attempt audits without mutating the bound plan.
- Test coverage summary: Component tests cover cancellation, interruption,
  checkpoint reuse/reconstruction, source faults, workspace safety and replacement
  reconciliation. `just test-media-service-recovery` exercises a real Linux
  service, restricted database, HTTP admission/cancellation and FFmpeg. Its owned
  ext4 mount and trusted catalog establish actual fixture root readiness. Source
  faults occur only after joined shutdown. The added checkpoint cases interrupt
  a real subtitle-sidecar writer after the main media checkpoint, remove/corrupt
  that output, and require reconstruction and completion on the same attempt.
  Full `just ci` and normal `just ui-e2e` remain required.
- Observability updates: Preserve interruption without a failure outcome; record
  explicit source-fault codes and append fresh preflight observations. Native
  qualification retains child diagnostics and verifies terminal outcomes and
  attempt identities. Fixture proof does not establish packaged-image compliance.
- Status-doc validation: Reviewed ADRs 593–595, Rust/data/DevOps instructions,
  Justfile and media-tooling guidance. ADR index and book summary register this
  record. The former copied tooling README in this file was replaced with this
  task record; historical native source-fault evidence is retained below.
- Risk & rollback plan: Checkpoint persistence and replay affect interruption and
  cleanup ordering; preserve pending replacement artifacts and reject changed
  sources. Roll back the scoped implementation together, including stored
  procedures and callers. Native fixture fault files are generated in private,
  disposable slots; join children and close database clients before cleanup.
  Retain owned container/volume caches; do not remove unrelated containers or
  dirty work. Retain rollback evidence until physical metadata persistence succeeds.
- Dependency rationale: No new dependency. Existing FFmpeg, PostgreSQL, rustix,
  tempfile, hashing and runtime collaborators retain their established roles.
- Stale-policy check: Reviewed root AGENTS.md and scoped Rust, data, UI and
  DevOps instructions against ADRs 593–594. Removed superseded takeover adapters
  and updated the affected guidance with checkpoint/shutdown qualification.
  Runtime database reads remain in the data crate. No lint, Sonar, coverage,
  or full-gate criteria were relaxed. The approved rollback-only physical
  fingerprint exception is documented in the data instructions. Existing tooling container
  naming lacks a task-level project-label override; this limitation remains
  recorded rather than changing unrelated adapters.

## Remaining qualification

The approved physical fingerprint correction passes component, application and
real Linux FFmpeg service recovery checks. A historical full CI and the corrected
release build pass; fresh CI is required for the later E2E fixture changes. The
latest native no-auth API phase passes 55 cases and fails the two positive
automatic discovery activation cases, which still receive the held-contract
response. A separate Chromium diagnostic passes all 22 cases. These selections
do not establish a passing normal UI gate. No overall completion, commit,
deployment or package proof is claimed.

The latest host application suite passes all 406 cases, including the bounded
discovery frontier regression and existing batch-resumption tests. Its data suite
passes 275 cases but fails initializer setup on a PostgreSQL statement timeout;
this is not a passing full test gate. Native coverage passes 409 application cases
before two positive root-proof fixtures reject overlay storage that was declared
disposable. The fixtures now reuse the existing `REVAER_NATIVE_RECOVERY_ROOT`
selection and declare restart-persistent storage when selected. Actual production
mount, ancestry, capability and root-lock checks remain in force. The complete
native coverage retry includes this fixture correction and the bounded frontier.

## Changed and missing sources through native service restart

- Extended the existing native qualification with two independent real sources.
  Each is admitted through the existing manual association, observed in real
  FFmpeg execution, interrupted through SIGTERM, and required to be queued on its
  original attempt after the service and FFmpeg join. Only then does the owned
  fixture replace the source bytes or delete that source file.
- Restart must record terminal failure on the same immutable attempt: changed
  source gives `media_job_source_fingerprint_mismatch`; missing source gives
  `media_job_source_fingerprint_unavailable`. It must neither overwrite the
  changed bytes nor recreate the missing file. These are external source faults,
  separate from the pending byte-identical backup-restoration identity decision.
  No production identity check, recovery protocol or source repair was changed.
- Motivation/test scope: qualify the objective's changed/missing-source paths
  through the serving process and restricted persistence after real joined work,
  supplementing the existing component filesystem faults. Passing service results
  still use fixture compliance metadata and do not prove package/release gates.
- Dependency rationale: none added. Observability retains child diagnostics and
  checked terminal error codes. Risk/rollback is limited to the fixture extension;
  fault files are generated, private and disposable. Stale-policy check reviewed
  root, Rust, data, UI and DevOps guidance and accepted ADRs 593–594. Rust guidance
  and media-tooling documentation name the added source-fault coverage. Full CI
  and normal UI remain required and are run sequentially to preserve asset state.

- Final `just test-media-service-recovery` passes in 140.77 seconds, including
  both new source faults and the existing interruption/replay/cancellation cases.
  No source rebinding or repair occurred: changed bytes remain exactly those
  written by the fault fixture, and the deleted source remains absent after
  failed replay and joined shutdown. Each original attempt identity remains
  unchanged. Native source bytes match the selected worktree's test file hash.
  Evidence: `native-source-service.log`.
- Final Linux `just lint-runtime-shutdown` passes all four configurations;
  `just instruction-drift` and `git diff --check` pass. CI is running alone, to be
  followed by normal UI after that exact handle is terminal. No production
  implementation change, source-identity exception, new dependency, commit or
  push was introduced by this qualification extension.
- Fresh sequential `just ci` passes preceding validation and reports 403
  application passes and one failure in 86.94 seconds. The failure remains the
  pending committed-restoration physical fingerprint mismatch. Fresh normal
  `just ui-e2e` reports 43 passes, 11 failures and four errors in 4.64 seconds,
  stopping in the same legacy media fixtures. Later CI and authentication/browser
  phases remain unproven. Evidence: `native-source-ci.log` and
  `native-source-ui-e2e.log`. Neither gate was weakened or bypassed.
- All started handles are terminal. The owned native service has no child work;
  its PostgreSQL fixture reports zero client sessions, and generated UI assets
  have no working-tree changes. Both owned containers are stopped and retained
  with the named media volume and build cache. Final instruction-drift and diff
  checks pass. The goal remains active: real source-fault qualification improved,
  but the restoration decision and failing full gates still prevent completion.

## Missing and corrupt checkpoints through native service restart

- Added independent real multi-step jobs for missing and corrupt completed main
  outputs. Generated embedded SubRip cues make the existing production sidecar
  extraction observable; no fake runner or synthetic execution step is used.
  After the main output checkpoint, SIGSTOP targets the observed owned subtitle
  FFmpeg child. SIGTERM then joins the serving process and that child before any
  checkpoint fault is applied. The source remains byte-identical after shutdown.
- Restart observes the real `libx265` writer rebuilding the invalid completed
  output, requires successful verified replacement, and preserves the only
  attempt and its claim identity. Both faults pass along with the prior native
  interruption, HTTP cancellation and changed/missing-source cases.
- `just test-media-service-recovery`: one selected test passes in 423.47 seconds.
  `just lint-runtime-shutdown`: all four Linux configurations pass. Local and
  container qualification source SHA-256 both equal
  `6a03f5be536f393afa087f86886db133f30c622470cc5b8bc52b1aff1092c83c`.
  Evidence: `native-checkpoint-service.log` and
  `native-checkpoint-linux-lint.log` in the worktree evidence directory.
- Motivation, design, observability, dependency and rollback scope remain those
  recorded above: this extension qualifies existing production behavior and
  introduces no production change or identity exception. Reviewed Rust guidance
  and media-tooling README now name the checkpoint cases. Instruction drift and
  diff checks pass. Full CI and normal UI results are recorded below.

- Fresh sequential `just ci` reports 403 application passes and one failure in
  85.06 seconds: committed rollback/replay still rejects restored physical
  identity. Fresh normal `just ui-e2e` reports 43 passes, 11 failures and four
  errors in 4.86 seconds in the legacy media API fixtures. Subsequent CI and
  authenticated/browser phases remain unproven. Evidence:
  `native-checkpoint-ci.log` and `native-checkpoint-ui-e2e.log`.
- All started handles are terminal. The native fixture has no remaining service
  or writer, PostgreSQL has zero client sessions, and generated UI assets have
  no changes. Owned service/database containers are stopped and retained with
  their cache and named media volume. No source-contract exception, new
  dependency, commit or push occurred. The goal remains active and incomplete.

## Retired profile request qualification

- The previous goal turn made progress: native missing/corrupt checkpoint cases
  passed. Fresh inspection of the remaining UI failures found two schema
  rejection cases still expecting the retired raw-path writer's SQL error.
  Corrected their expected `media_configuration_invalid` response and names;
  both retain no-persisted-profile and unchanged-source-byte assertions.
- A complete logical-key payload alone cannot qualify positive persistence: the
  current stored procedure first requires an active native catalog. Unverified
  draft rewrites were discarded; original positive persistence, discovery and
  disabled-admission requirements remain failing rather than being replaced by
  negative assertions. A real Linux catalog is required to port those cases.
- Motivation/design: align request rejection with the existing handler contract,
  with no production or architecture change. Observability is the exact error
  code; no dependency was added. Rollback is the two-case expectation/name edit.
  Reviewed root, Rust, Python, UI and accepted ADRs 593–594; UI guidance now
  distinguishes schema rejection from native readiness/admission proof.
  Full CI and normal UI are rerun sequentially; results follow.

- Fresh `just ci`: 403 application tests pass and committed-restoration replay
  fails in 98.90 seconds with the unchanged physical fingerprint mismatch.
  Fresh normal `just ui-e2e`: 45 pass, nine fail and four fixture errors in
  5.38 seconds. Both corrected retired-body cases pass (JUnit has no failure
  entries for either parameter). Authenticated/browser phases remain unproven.
  Evidence: `retired-profile-ci.log`, `retired-profile-ui-e2e.log` and the
  anonymous-phase JUnit result. Formatting, instruction drift and diff checks
  pass. No gate criteria or original positive assertions were removed.
- All started handles are terminal. PostgreSQL has zero clients; generated UI
  assets remain clean. The owned database is stopped and retained; native
  fixtures were already stopped and unchanged. The goal remains active, with
  positive catalog-backed UI fixture migration and the human restoration
  decision still outstanding. No production changes, commits or pushes occurred
  in this continuation.

## Native prepared replacement reconciliation

- Previous continuation made progress by correcting two request-schema cases.
  Recovery audit still finds committed rollback/replay dependent on the pending
  human identity decision. An independent native startup-reconciliation case is
  added without changing that contract.
- After the real sidecar writer and serving process join, the fixture uses the
  production filesystem committer to prepare a durable stage, manifest and
  original backup for the same stopped job. The candidate is the completed main
  output, and preparation must preserve the original source bytes. Dropping the
  in-memory transaction leaves its real on-disk artifacts for startup.
- Restart must observe subtitle extraction rather than main-media encoding,
  complete the same attempt, publish verified media and leave no transaction
  directory. The absence check is read-only; it does not call recovery to hide
  unfinished artifacts. This qualifies service reconciliation of a fixture-
  prepared transaction, not an interruption inside the service's final commit.
- Motivation/design: strengthen existing prepared-replacement and checkpoint-
  reuse proof on the owned native mount. No production implementation, source
  fingerprint rule or new dependency changes. Observability preserves native
  diagnostics and checks the writer/attempt/outcome. Risk and rollback are limited
  to the fixture extension, which retains joined cleanup. Reviewed root, Rust
  and accepted ADRs 593–594; Rust and tooling guidance state the exact proof and
  its limit. Native qualification and Linux lint are running; evidence follows.

### Objective audit (not completion)

| Requirement | Current evidence and limit |
| --- | --- |
| Unfinished-step restart without failure retry | Final native HTTP/FFmpeg interruption and replay pass on the original attempt. |
| Preserve explicit cancellation | Native HTTP cancel joins FFmpeg, preserves source and remains terminal across restart. |
| Missing/corrupt intermediates | Both native completed-output fault cases passed in the 423.47-second qualification. |
| Changed/missing sources | Native failures preserve changed bytes/deleted absence and original attempt identities. |
| Stop old work before replay | Native service and child joins plus `/proc` absence checks precede every fault/restart. |
| Prepared replacement reconciliation | Native serving-process reconciliation of fixture-prepared artifacts passes in the 545.43-second qualification; service-internal commit interruption remains unproven. |
| Committed replacement reconciliation/replay | Approved physical metadata refresh and final native fixture-committed replay pass; service-internal commit-boundary interruption is not claimed. |
| No superseded coordination machinery | Existing root lock and normalized checkpoints retained; no supervisor, lease or registration added. |
| Required full gates | Full CI passes after the scoped compiler-plugin strip workaround. Normal UI fails in retained legacy media API fixtures; authentication/browser phases remain unproven. |

The requested end state remains unchanged. Successful cases above do not waive
committed recovery, native workflow evidence or full gates.

- Fresh full `just ci`: 403 application tests pass and committed-restoration
  replay fails in 95.34 seconds with the unchanged fingerprint mismatch. Fresh
  normal `just ui-e2e`: 45 pass, nine fail and four fixture errors in 4.61
  seconds; authenticated/browser phases remain unproven. Evidence:
  `native-prepared-ci.log` and `native-prepared-ui-e2e.log`. Both handles are
  terminal. Linux `just lint-runtime-shutdown` passes all four configurations.
  Local/container fixture SHA-256 matches
  `9ecd88f1ae3d5e48f7c1f7711b2b10c80a9efbeb4762275e49c6aa945d6d0f69`.
  Native service qualification remains running; no success is yet claimed.

- Final `just test-media-service-recovery`: one selected native test passes in
  545.43 seconds, including the new prepared replacement and all prior
  interruption/cancellation/checkpoint/source-fault cases. Startup reconciles
  the fixture's real prepared artifacts, reuses completed main media, restarts
  actual subtitle extraction and completes the same immutable attempt. Final
  transaction-directory absence is checked without mutating recovery state.
  Evidence: `native-prepared-service.log`. This proves the precise new case; it
  does not approve or qualify committed restored-source replay.
- Final instruction drift and diff checks pass. All started handles are terminal;
  the owned native container has no service or writer, PostgreSQL has zero
  clients, and generated UI assets have no changes. Owned native/database
  containers are stopped and retained with their cache/media volume. No
  production change, new dependency, source-identity exception, commit or push
  occurred. The goal remains active and incomplete: committed recovery needs
  the human decision, and full CI/UI gates remain failing.

## Approved physical fingerprint refresh, 2026-10-04

The operator approved refreshing the existing job after verified restoration.
Only device/inode identity and mtime/ctime change; digest, size and all bindings
remain fixed. Retaining the original backup until persistence succeeds closes the
restore-to-database crash gap. Repeated filesystem restoration and both startup
and in-process recovery use the same acknowledgement ordering. The immutable
trigger still rejects ordinary physical-field updates; a transaction-local marker
is set and cleared only around the guarded procedure's update. No dependency,
checkpoint table or broader source-identity relaxation was introduced.

Qualification adds changed-content rejection, retained-backup retry and a real
service restart from a fixture-committed replacement. Full gates are rerun below.

- Initial validation caught an overbroad immutable-trigger change; the guarded
  procedure now uses a cleared transaction-local marker and ordinary physical
  metadata edits still fail. All 65 media data tests pass on the revised initializer.
- The first application pass exposed a macOS fixture alias mismatch between the
  catalog root and canonical recovery manifest. The fixture now uses canonical
  roots, matching real admission. Production path validation was not weakened.
- Cancellation and post-commit verification failure now use the same retained
  recovery and acknowledgement helper instead of consuming backups directly.

- `just test-media-recovery` passes: 361 runtime unit tests, 5 media integration
  tests (the prepared-media fixture remains explicitly ignored by its existing
  contract), 65 data tests and 43 application recovery tests. The original
  committed-restoration regression and injected database-save failure both pass.
- `just lint-runtime-shutdown` passes in both feature modes on macOS and Linux.
  Full CI passes the complete 405-test application library suite; remaining
  workspace/gate phases are still running. The first native expanded run passes
  in 624.87 seconds, including fixture-committed replacement replay. A final
  snapshot run follows the module-order and in-process rollback corrections.

- Final Linux `just lint-runtime-shutdown` and `just test-media-service-recovery`
  pass. The native test completes in 706.01 seconds with one passing test and no
  ignored selection. Runtime, replacement and initializer source hashes match
  the working tree. The fixture includes real HTTP/FFmpeg cancellation,
  missing/corrupt checkpoints, changed/missing sources, and prepared/committed
  journal reconciliation on the same attempt. Final-replacement transactions
  are deliberately staged by the fixture after joined shutdown; this does not
  claim a service-internal commit-boundary interruption.
- The native fixture has no remaining service or FFmpeg children and is stopped
  for reuse, retaining its source/cache checkout and dedicated volume. Database
  service remains running until the independent full CI/UI gates finish.

- Full CI clears baseline, formatting, workspace lint, Helm, policy drift,
  assets, dependency use/audit/deny, UI build, workspace tests, minimal-feature
  tests, and Rust/native coverage. It then fails tooling coverage with 1744
  passing tests and seven failures. Six come from the database-selection unit
  fixture inheriting the real CI E2E admin URL; the fixture now explicitly sets
  that independent setting to absent for its fallback matrix. Production
  selection and all assertions remain unchanged. The remaining Dockerfile proof
  fails on a transient Alpine package-index DNS error; no pins or gate criteria
  were changed. A fresh full gate retry follows.

- Normal `just ui-e2e` runs after CI terminates and retains its existing failure:
  45 pass, nine failures and four setup errors in 9.96 seconds. Legacy media
  fixtures submit raw-root profile bodies and retired PATCH/discovery contracts.
  They fail before the later browser phases, independently of rollback metadata.
  No UI assertion, admission contract or quality gate was weakened to make the
  targeted restoration change appear fully qualified.

- Full CI retry clears every validation phase. All 1751 tooling tests pass in
  398.77 seconds with 94.91% coverage; the real Dockerfile proof also passes
  without a pin change. Release build then stops because macOS rejects
  `libsqlx_macros-551642c1b450a319.dylib` with a misaligned LINKEDIT string pool.
  No compiler/profile/dependency rule was relaxed. After checking that no Cargo
  or rustc process remains and acquiring the existing release cache lock, the
  malformed generated artifact is preserved in the task evidence directory and
  the canonical `just build-release` is retried once.

- The clean release-artifact rebuild reproduces the same macOS LINKEDIT
  alignment error. The malformed initial artifact and both build logs are
  retained; no binary patch, toolchain/profile change or gate suppression was
  introduced. Overall CI remains failed at release despite passing validation.
- Both owned fixture containers are stopped after their processes terminate and
  PostgreSQL reports zero other client backends. Containers, cache checkout,
  database storage and the native dedicated volume are retained for reuse.
  Source changes and evidence remain uncommitted in the isolated worktree.

- The reproduced release failure matches upstream Rust issue 157750
  (<https://github.com/rust-lang/rust/issues/157750>): LLVM stripping in the
  pinned Rust 1.96 can misalign macOS dynamic-library string tables. The workspace
  now explicitly sets `profile.release.build-override.strip = "none"`, using
  Cargo's documented build-dependency override. This retains symbols in compiler
  plugins without changing shipped runtime optimization, the toolchain pin,
  dependencies, lint posture, or gate criteria. The Rust scoped instructions
  record removal after a verified upstream toolchain fix. Rollback is removal
  of this single profile override; canonical release qualification follows.

- Canonical `just build-release` now passes in 83 seconds, including all targets
  and features. Evidence: `restore-refresh-release-nonstrip.log`. The full
  `just ci` gate is rerun against this exact source; the earlier passing
  validation phases are not substituted for this run.
- E2E inspection confirms that the retained media API fixtures still send
  retired raw-root/PATCH/profile-discovery bodies, while catalog assertions
  require a missing startup source. Positive association discovery requires a
  real Linux catalog, and catalog authority is startup-only. A body-only fixture
  rewrite on macOS cannot qualify that workflow. Preserve both missing-catalog
  negative checks and qualified-root positive discovery; do not invent
  attestation, weaken assertions, or add a runtime catalog reload to evade the
  harness mismatch. The existing native service proof remains separate from
  full operator/browser acceptance.

- Prepared a separate local Debian 13 arm64 browser fixture because the existing
  recovery fixtures use Alpine, outside Playwright's supported Linux platforms.
  Rust remains 1.96.0 and uv remains 0.12.13; locked project setup installs
  Chromium and Trunk 0.21.14. Native libtorrent is 2.0.11 and FFmpeg is 7.1.5.
  The named `revaer-recovery-ui` container carries project/purpose labels and
  reuses the ext4 fixture volume only while the old recovery service is stopped.
  Docker CLI access and the explicit existing database endpoint are verified.
  `just check` passes in 24.86 seconds; Cargo manifest, recovery runtime,
  replacement and initializer hashes match the host checkout. The initial setup
  retries corrected missing fixture source/Justfile, argument quoting, the
  Debian Docker CLI package split and ownership of a root-created uv cache.
  These are fixture preparation results, not passing API/browser scenarios.
  No production service, catalog authority, dependency or gate is changed.

- Fresh `just ci` passes all validation, coverage and release stages. All 1751
  tooling tests pass in 456.10 seconds; the release stage passes in 0.41 seconds
  using the earlier verified build. Evidence:
  `restore-refresh-ci-nonstrip.log`. Normal `just ui-e2e` then reports 45 passing
  tests, nine failures and four fixture errors in 9.91 seconds, all in the
  retained legacy media profile/lifecycle scenarios. Authentication and Chromium
  phases remain not run. Evidence: `restore-refresh-ui-nonstrip.log` and the
  failed current-run E2E summary. No UI assertion or admission gate was weakened.
- Every started command is terminal. The prepared browser fixture is stopped
  for reuse with its cache retained; the old native fixtures remain stopped.
  PostgreSQL reports zero other client backends before shutdown. The goal remains
  active: port positive catalog/association API and browser cases on the prepared
  real Linux fixture while retaining independent missing-catalog checks. No
  production architecture change, commit or push occurred in this continuation.

## Media E2E catalog lifecycle

- Current source establishes a second harness failure: `configure_auth` calls
  factory reset after the API starts, while the initializer's reset procedure
  truncates public tables and reinstates missing catalog state. Startup-only
  attestation is therefore lost even with a valid configured Linux catalog.
- The media harness now uses the existing owned service lifecycle for setup,
  joins that service, then starts the scenario service to attest the actual
  catalog. No runtime reload, synthetic generation, lease or coordinator is added.
  Original missing-catalog checks remain separate under both authentication modes;
  positive phases retain foundation and remaining media scenarios. Default media
  selection, shard validation and raw Python/JavaScript coverage require all five
  phases. Setup and scenario logs remain separate. Failed readiness fails the run.
- This is fixture ordering within the approved startup authority, not a change
  to application/catalog architecture. No production dependency is added. Rollback
  restores the previous harness ordering, which cannot qualify native positive
  fixtures. Reviewed root, Python, UI, devops and ADRs 593–594; scoped guidance and
  the E2E README now describe this ordering. Orchestration and native verification
  results follow; positive fixture payload migration remains required.

- Native Debian qualification proves the retained anonymous missing-catalog
  checks (three passes), joined setup/scenario restart, and actual ready catalog
  before positive API collection. The first run retains 42 passing tests, nine
  legacy request failures and four lifecycle fixture errors; API-key/browser
  phases are not reached. Evidence: `ui-catalog-lifecycle-native.log` and copied
  `ui-catalog-lifecycle-results`/`ui-catalog-lifecycle-logs`.
- Migrated profile creation fixtures use complete immutable requests, explicit
  real target/policy versions, conditional PUT, association-relative candidates
  and the real source slot as the in-place output binding. A profile requesting
  non-dry-run cannot override the selected policy's dry-run setting. Disabled
  schedule/watcher admission, duplicate protection, diagnostics, original bytes
  and retired-body rejection remain asserted. Metadata updates use the profile
  description; global job retention retains its independent lifecycle scenario.
  No runtime attestation is manufactured and no positive automation scenario
  is replaced by an expected rejection.
- Intermediate native retries exposed incomplete verification settings in the
  policy fixture and the required same-source in-place output binding. After
  correcting these, 50 positive API tests pass, with one remaining absolute-path
  expectation and four untouched legacy lifecycle fixture errors (9.63 seconds).
  The job detail response preserves the root-relative candidate, so its
  expectation was corrected. A later run fails during authentication factory
  reset with an HTTP connection failure before positive collection; the owned
  service is joined and the disposable database dropped. Evidence:
  `ui-profile-contract-native.log`, `ui-profile-complete-policy-native.log`,
  `ui-profile-inplace-native.log`, `ui-profile-final-native.log`. Final retry
  results follow.
- Final native retry passes all eleven migrated profile cases: the positive API
  phase reports 51 passes and four errors in 9.02 seconds, all four in untouched
  legacy lifecycle setup bodies. Missing-catalog checks still pass independently.
  Evidence: `ui-profile-final-retry-native.log`, `ui-profile-final-results` and
  `ui-profile-final-logs`. Authentication with API keys and Chromium remain
  unreached. The full UI gate has not passed; no completion is claimed.
- `just tooling-check` passes all 1756 tests in 520.21 seconds. The shared
  orchestration fixture now includes the readiness endpoint in its OpenAPI
  document, and the cleanup assertion follows whether UI startup was reached.
  Evidence: `ui-catalog-tooling-check.log`. `just fmt` and diff whitespace
  checks pass. A fresh complete `just ci` is running for the current changes.
- Fresh full CI passes all ordinary workspace tests (including 405 app and
  276 data tests) and the 344-test minimal-feature app suite. Coverage then fails
  with 404 app passes and one database connection error: PostgreSQL SSLRequest receives
  an invalid `0x00` response in the startup workspace ownership test. No ownership
  assertion failure is reported; the message does not identify which connection
  failed. Coverage, tooling coverage and release are not
  complete in this run. Evidence: `ui-catalog-profile-ci.log` and
  `ui-catalog-profile-db.log`. The recovery gate is rerun against the same owned
  disposable local endpoint with explicit `sslmode=disable`; this changes only
  test transport selection, not production settings, assertions or gate criteria.
- The explicit local non-SSL transport rerun of `just test-media-recovery`
  passes: 361 runtime tests, five integrations (one retained existing ignore),
  65 media-data tests and 43 application recovery tests. The startup workspace
  ownership case passes in that complete recovery selection. Evidence:
  `ui-catalog-recovery-db-transport.log`. This does not retroactively pass the
  failed coverage run; fresh complete CI remains required.
- Instruction drift and diff whitespace checks pass. The qualified profile
  fixture hash exactly matched the then-current host source (`e037281dbf3f3595d956aadca772832946b6f24839a125ad8e76e3039a1a66b8`);
  the native copy and hash evidence are retained. All started commands are now
  terminal. The browser fixture is stopped for reuse; PostgreSQL reports zero
  other client backends and no remaining disposable databases before shutdown.
  Containers, the ext4 media volume and caches are retained. The goal remains
  active; no commit, push, acceptance-gate relaxation or production design change
  occurred in this continuation.


## Native lifecycle fixtures and authentication continuation

- Shared native profile prerequisites now create real complete target and policy
  versions. Lifecycle inputs are files inside the owned catalog source slot;
  requests use logical root keys, exact version pins and relative candidates.
  Fixture teardown verifies the original source bytes. These small text files
  prove admission and diagnostics, not FFmpeg execution; native execution proof
  remains in the earlier service recovery qualification.
- Lifecycle coverage now uses complete conditional profile updates, actual
  capability/readiness responses and the current portable YAML create/match
  contract. Local diagnostic snapshots remain explicitly non-importable.
  Manual discovery, duplicates and job diagnostics are tested independently
  from the two positive schedule/watcher activation cases. The latter still
  require successful activation and remain failing; no rejection expectation
  replaces the approved workflow.
- The current native no-auth selection passes all three absent-catalog checks
  and 55 positive API cases; only the two automatic activation cases fail with
  409. Evidence: `ui-lifecycle-association-native.log` and its retained results
  and logs. The canonical association route parameter now matches OpenAPI.
- The management browser fixture binds a real native profile and association,
  uses the current profile/root and association editors and supplies explicit
  target/policy versions. A selected Chromium diagnostic passes all 22 cases
  in 20.19 seconds. Evidence: `ui-lifecycle-browser-diagnostic.log` and retained
  results/logs. It does not prove profile form submission, media execution or
  the complete default UI gate.
- `just tooling-check` passes 1756 tests in 396.47 seconds after these fixture
  changes. Evidence: `ui-lifecycle-tooling-check.log`. A subsequent API-key
  diagnostic exposes a harness reset error: service restart retains the previous
  API key, but the next reset client omitted it and receives 401.
- Corrected only the setup client to carry the current session into reset, then
  replace it with newly issued credentials. The coordinator test verifies the
  four media setup clients, including authenticated key-to-key reset. No runtime
  authentication, catalog activation or automatic-discovery guard is relaxed.
- The corrected native API-key diagnostic passes three absent-catalog checks
  (1.25 seconds) and 55 positive API cases (102.77 seconds), failing only the two
  schedule/watcher activation cases with `media_configuration_pending_contract`.
  The owned service is joined and its disposable database dropped. Evidence:
  `ui-auth-api-key-native.log`, `ui-auth-api-key-results` and
  `ui-auth-api-key-logs`. This selected phase run is not the full default gate.
- Initial CI attempts reject unsupported URL overrides and missing explicit
  lifecycle credentials before validation. The corrected run uses the existing
  owned PostgreSQL endpoint and the canonical `REVAER_LOCAL_DB_*` inputs, without
  URL overrides. Fresh complete CI and post-auth tooling checks are in progress;
  results follow. Evidence: `ui-lifecycle-ci-admin.log` and
  `ui-auth-tooling-check.log`.
- Reviewed the actual ADR 594 approval and current native admission path.
  Existing background discovery still reads legacy profile roots and modes;
  native association mode activation remains deliberately held. The next
  implementation step is the already approved native association integration
  into the existing bounded watcher/scan/schedule loop, using retained catalog
  roots and duplicate-safe native admission. Removing the hold without that
  integration would manufacture a pass and is not an acceptable correction.

- Post-authentication `just tooling-check` passes all 1756 tests in 444.55 seconds,
  including the authenticated phase-transition regression. Formatting, lint and
  strict typing pass; instruction drift and diff whitespace checks pass.
  Evidence: `ui-auth-tooling-check.log` and `ui-auth-instruction-drift.log`.
  The idle owned browser container is stopped and retained. Fresh CI continues
  against the separate owned database; it must finish before database shutdown.

- Fresh CI passes the ordinary app (405), data (276) and minimal app (344)
  suites. Coverage reports 404 app passes and one failure in
  `media_profile_readiness_returns_none_for_missing_profile`: an unexpected
  `0x00` SSLRequest response. This repeats the same transport symptom previously
  seen in another coverage test; it does not identify the failing connection or
  prove the underlying cause. Tooling coverage and release are not reached.
  Evidence: `ui-lifecycle-ci-admin.log` and `ui-auth-ci-db.log`.
- Reviewed the actual settings precedence and CI composition. A fresh full CI
  run supplies the plain owned endpoint as `E2E_DB_ADMIN_URL` for container
  ownership/initializer verification, while `REVAER_TEST_DATABASE_URL` selects
  `sslmode=disable` on that same local test endpoint. This preserves the strict
  no-override administrative guard and avoids SSL negotiation only in the
  explicitly selected disposable SQLx test connections. No production settings,
  automatic-discovery guard, assertion or gate criteria change. Evidence:
  `ui-auth-ci-db-transport.log`; this run is in progress.

- Completion audit inspected the native fixture's actual replacement calls.
  Prepared and committed faults leave real SystemReplacementCommitter-created
  durable transactions after joined service shutdown; restart reconciles those
  actual journals and preserves the attempt. This qualifies recovery from those
  durable states, without claiming a signal at the service-internal rename
  boundary. No additional crash-boundary acceptance requirement was invented.
- Reviewed the retained native discovery schema and scoped instructions before
  integration. The private execution-slot/lease routines are superseded by ADR
  594 and must not be reused. Clarified the scoped data guidance to preserve
  immutable binding/activation fences and complete-scan acknowledgement while
  using the existing root lock, without scanner leases or distributed slots.
  This applies existing approval; automatic activation remains held until its
  real bounded runtime is integrated and tested. Production source remains
  unchanged during the live complete CI run.

- The explicit non-SSL full CI run passes ordinary app (405), data (276) and
  minimal app (344) suites. Coverage then reports 404 app passes and one indexer
  failure: `missing_indexer_operations_surface_not_found_across_runtime_views`
  receives an unknown PostgreSQL message type `\0`. Both earlier failing media
  cases pass in this coverage run. Disabling SSL has not resolved the underlying
  invalid-byte symptom; its cause remains unproven. The complete run is terminal
  and its owned outer database is dropped. Evidence: `ui-auth-ci-db-transport.log`.
- Next diagnostic uses the existing owned Linux qualification container and a
  direct connection to the same inspected PostgreSQL container address, while
  retaining the published, ownership-verified administrative endpoint. No test
  assertion or gate is relaxed. Prepare an exact current source snapshot and
  authentic shallow Git base objects so source/guardrail checks do not rely on
  the image's original empty Git inventory. This creates no commit or branch.
  Linux prerequisites are installed only through the existing pinned setup task.

- Reused `revaer-recovery-ui`; verified it was idle before synchronizing 2547
  current source files. Reconstructed its authentic shallow Git base
  `930bd3e06186d284da27451543603b4b02766dcf` from local commit/tree/blob objects
  (2533 tracked files), without creating a commit, branch or changing the host
  checkout. This replaces the image's original empty Git inventory for accurate
  guardrail/source inspection. Native diff whitespace checks pass.
- `just setup --profile ci` installs the reviewed `cargo-llvm-cov` 0.8.7 and
  clang-19 selection inside that owned container; no repository dependency or
  host installation changes. An initial empty browser argument is rejected by
  the Just argument forwarding, then the supported `--browsers=` spelling succeeds.
  Evidence: `native-ci-coverage-setup.log`. Source pack/archive/base records are
  retained under the task evidence directory.
- Inspected PostgreSQL's current owned container address `172.17.0.16`; direct
  readiness succeeds. Native `just cov` uses that same owned server directly with
  explicit non-SSL test transport, clang-19/clang++-19 and Rust's own LLVM tools.
  The complete workspace/all-feature/FFI collection is running, without selecting
  fewer tests or altering thresholds. This is a diagnostic coverage run, not a
  passing complete CI or UI gate. Evidence: `native-ci-coverage-direct.log`.

## Bounded discovery frontier and qualified native fixtures

- Replaced the existing breadth-first pending-directory queue with a retained
  ancestor chain. Batch limits, depth skips, deterministic candidate ordering and
  resumption remain in place. The frontier is bounded by traversal depth rather
  than accumulated directory count. No persistence or coordination mechanism was
  added. A wide-tree regression checks all candidates, no duplicates, completion
  and the retained frontier bound across one-file batches.
- The first scan change unnecessarily retained an exhausted parent and failed
  the existing entry-budget batch-count assertion. Retaining a parent only when
  siblings remain fixes that failure. The unchanged assertion and all other scan
  cases pass in the complete 406-case application suite. The full `just test`
  retry subsequently fails one database initializer on its statement timeout
  (275 data cases pass); no timeout or test requirement is relaxed.
  Evidence: `native-discovery-frontier-tests-retry.log`.
- The first Linux complete coverage collection is terminal: 409 app cases pass,
  two positive root fixtures fail, and one pre-existing native service test is
  ignored. Both failures correctly reject the container's overlay filesystem
  declared as disposable storage. The fixtures now use the same existing owned
  persistent-root selection as service recovery and accurately declare its
  durability. The fallback private HOME declaration remains tmpfs-only. Native
  mount and ancestry checks remain unchanged. This tests actual owned ext4
  storage; it does not qualify release packaging or Kubernetes.
- Native `just cov` is retrying the complete workspace and all features with the
  corrected fixtures, bounded scanner and `REVAER_NATIVE_RECOVERY_ROOT=/srv/revaer`.
  Evidence: `native-ci-coverage-qualified-storage.log`. The complete collection
  and thresholds are not yet a passing result. Host `just fmt`,
  `just instruction-drift` and `git diff --check` pass; the application Clippy
  checks are running. No commit, PR or production mutation is made.
- Both formerly failing native startup fixtures pass in the qualified-storage
  coverage retry. Its remaining collection is still running. Application Clippy
  catches the scan function length (106 lines, then 104 after the first extraction).
  Extracted the existing no-follow metadata observation and frontier-descent
  operations without changing traversal behavior or suppressing the lint.
  The frozen native run predates these helper extractions, so it cannot certify
  the final source snapshot. Host formatting and whitespace checks pass for the
  extracted version; final Clippy is running in both feature modes.
- Final `just lint-runtime-shutdown` passes test and production Clippy in both
  feature modes with no warnings or source suppressions. Final `just fmt`,
  `just instruction-drift` and whitespace checks pass. Evidence:
  `native-qualified-storage-lint-final.log`,
  `native-qualified-storage-fmt-final.log` and
  `native-qualified-storage-instruction-drift-final.log`.

## Remove superseded discovery execution leases

- A current source audit finds no runtime caller for the two discovery execution
  slots or their 20-second claim/renew/release protocol. Only the initializer,
  runtime grant list and old TypeScript rescan test retain them. ADR 594 explicitly
  supersedes this protocol; removal follows existing approval.
- Removed the unused execution-slot table/indexes, seed inserts, three procedures
  and their runtime grants from the single init. Preserved the coalesced rescan
  publisher, seven closed reasons, sequence overflow protection, schedules and
  immutable association/catalog binding. Existing job claim and replacement
  guards are separate and remain in place.
- Replaced the old rescan test's lease/takeover section with absence checks, while
  retaining its useful request-coalescing and service-restart assertions. Added
  an executed Rust data regression for full-init absence of the superseded schema,
  denied restricted-runtime publication, bounded reason coalescing, unchanged
  unsatisfied high-water mark and transaction rollback on sequence overflow.
  Catalog evidence in this data regression is explicitly synthetic; real native
  service restart evidence remains a separate requirement.
- Linux coverage on the prior frozen snapshot now passes all 412 app cases,
  including both positive root fixtures and the frontier-bound regression, with
  the one pre-existing ignored native service case. Its remaining collection is
  running. That snapshot predates helper extraction and execution-lease removal,
  so it is diagnostic evidence rather than a final current-source gate.
- `just test-media-recovery` passes 361 runtime unit cases, five runtime
  integration cases (one existing ignored case), 66 selected data cases including
  the new lease-free rescan regression, and 43 serialized application recovery
  cases. Evidence: `lease-free-rescan-recovery-tests.log`. That run uses the first
  overflow assertion implementation. Policy subsequently rejects its inline
  fixture mutation; moved the mutation and expected-overflow rollback into the
  existing owned SQL-fixture boundary without relaxing policy. The updated
  version passes full `just lint` including both workspace Clippy selections;
  its fixture execution still needs a fresh run.
- Linux complete coverage is terminal: app 412 and data 276 cases pass. Runtime
  reaches 369 passes and seven process-control failures, so reports/thresholds are
  not qualified. The container's PID 1 is `sleep`, and inspection finds 13 zombie
  test descendants parented to it. Tests correctly retain cleanup-error evidence
  while those groups remain present. Reuse the image's existing Tini 0.19.0 as a
  standard subreaper for the next qualification command; do not alter deadlines,
  cleanup assertions or production supervision. Evidence:
  `native-ci-coverage-qualified-storage.log`.
- Current-source Linux `just test-media-recovery` under existing Tini's standard
  subreaper passes all 376 runtime unit cases, five integration cases (one
  pre-existing ignored case), 66 selected data cases and 43 application recovery
  cases. All seven formerly failing process cases pass without production or
  assertion changes. The new owned overflow SQL fixture executes successfully
  and preserves the sequence. Evidence: `lease-free-native-recovery-subreaper.log`.
- Full host `just lint` passes source/workflow policy, Python lint/types and both
  workspace Clippy selections after the fixture boundary correction. Instruction
  drift and whitespace checks pass. Evidence: `lease-free-rescan-lint-retry.log`
  and `lease-free-rescan-final-instruction-drift.log`. The next native coverage
  collection uses the current source under the same standard subreaper; no
  coverage, deadline, cleanup or quality criteria are weakened.

## Retained native source qualification

- The complete Linux coverage run under Tini passes all application, data and
  runtime tests, including the seven process cases that failed without orphan
  reaping. Its application coverage is 21,008 of 23,358 lines (89.94%), below the
  required 90%; it is a failed coverage gate, not a passing CI result. Evidence:
  `lease-free-native-coverage-subreaper.log` and the retained coverage reports.
- Coverage identifies a real integration-test gap: API tests substitute the
  source provider rather than exercising the production retained-root reader.
  Added actual retained-directory tests for source and sidecar fingerprints,
  unknown and output-only keys, missing paths, replaced root identity and symlink
  confinement. The directory authority and reads are real; generation values in
  these focused tests are explicitly synthetic. No admission, coverage or cleanup
  criteria changed.
- Full host formatting and lint pass for these tests, including the final
  symlink-confinement assertion. Evidence: `retained-native-source-fmt-final.log`
  and `retained-native-source-lint.log`. The native full test run passes both new
  provider tests before the final symlink addition and completes the full
  workspace successfully, including 277 data cases. Evidence:
  `retained-native-source-tests.log`. Fresh `just cov` under the same standard
  subreaper is running against the final provider tests, including symlink
  confinement; its result remains pending.
- The final retained-reader tests, including symlink confinement, pass in that
  fresh coverage collection. The complete collection and thresholds remain
  pending; this does not turn a selected test result into a full coverage pass.

## Linux CI prerequisite preparation

- Reuse the existing qualification container and its caches. Install the pinned
  audit, deny and unused-dependency tools through `just setup` with a separate
  installer target. Installation passes for cargo-audit 0.22.0, cargo-deny 0.18.9
  and cargo-udeps 0.1.57, including the pinned unused-dependency toolchain.
  Evidence: `native-ci-tool-prerequisites.log`. No application dependency, Rust
  edition, gate or analyzer scope changes.
- Installed the workflow's Helm v3.19.0 for Linux arm64 after verifying the
  official archive SHA-256. The Linux chart gate passes annotation and compliance
  selections but stops in its real registry tests because `htpasswd` is absent.
  Installed the precise `apache2-utils` prerequisite through `just setup`; no
  broad package upgrade. Evidence: `native-ci-helm-lint.log` and
  `native-ci-chart-prerequisite.log`.
- Inspection of that fixture also identifies unnamed Docker containers. Give
  each distinct test credential/storage scope a Revaer name and project/purpose
  labels, check existing containers before creation, and retain its existing
  context-manager cleanup. Do not reuse another concurrent test's certificate
  or authentication state. Host `just helm-lint` passes all 155 fixture tests,
  strict chart lint and packaging; full `just lint` passes. No labeled registry
  fixture remains after the successful run. Evidence:
  `project-owned-chart-registry-host.log` and
  `project-owned-chart-registry-lint.log`. The running native Rust collection
  keeps its frozen source; this helper change does not change Rust coverage.
- Fresh full host `just ci` rejects the administrative URL's `sslmode` query
  parameter at the owned-database preflight. Removed that invocation override,
  leaving the ownership check intact; the rerun is active. Evidence:
  `retained-native-source-full-ci.log` and
  `retained-native-source-full-ci-local-url.log`. Linux native coverage remains
  a separate qualification, and neither pending command is a passing final gate.
- Completion audit re-reads ADRs 593–594 and the current native service fixture.
  Their existing requirements remain in force. Corrected stale predecessor
  implementation headings that still said product rollback/recovery had not
  begun; linked this task's evidence while retaining the explicit incomplete
  workflow/release status. No approval, design or acceptance criterion changed.
- The corrected current-source host CI invocation passes the full workspace
  tests (406 application, 277 data and 361 media-runtime cases) and both
  minimal-feature test selections, then enters complete coverage collection.
  The separate Linux run passes all 414 application cases, including the final
  retained-reader assertions. Both coverage collections are still live; no full
  CI, coverage-threshold, UI or release qualification claimed from these stages.
- Current-source Linux `just cov` completes successfully under the existing
  standard subreaper. All 414 application cases, 277 data cases and 376 runtime
  cases pass; the one pre-existing native service test remains explicitly
  qualified by its separate outside-in command. Every crate meets its unchanged
  90% Rust line minimum. Application coverage is 21,039 of 23,358 lines (90.07%).
  Complete native-inclusive LCOV, text, HTML, per-crate JSON and toolchain evidence
  are retained in the container and copied to
  `target/cancellation-recovery-evidence/retained-native-source-qualified-coverage`.
  Evidence: `retained-native-source-coverage.log`. This passes native coverage,
  not the still-running full host CI or the unfinished automatic-discovery UI.

## Retained directory traversal for native discovery

- Implemented read-only directory opening in the existing retained-root adapter
  in the idle Linux qualification checkout. An explicitly empty relative path
  opens the declared root; child paths retain strict raw component validation,
  no-follow opens and root identity checks before/after traversal. Existing
  candidate-parent reads use the same implementation. No temporary sentinel
  filename, new dependency, lease or owner protocol is needed.
- Added actual filesystem checks for root/child identity, invalid components,
  missing directories, symlinks and replaced roots, and exercised trusted-catalog
  slot lookup. All 98 root-catalog cases pass, including the real FFmpeg root-lock
  case. Final full Linux `just lint` and formatting pass. Evidence:
  `retained-directory-reader-native-final-tests.log`,
  `retained-directory-reader-native-final-lint.log` and
  `retained-directory-reader-native-final-fmt.log`.
- Exact copies of the three changed root-catalog source files are retained under
  `target/cancellation-recovery-evidence/native-discovery-directory-reader`;
  the preceding qualified versions are retained under
  `target/cancellation-recovery-evidence/recovery-qualified-root-catalog`.
  The host CI run completes successfully against that preceding recovery
  implementation, including complete Rust/native and tooling coverage, all 1,756
  tooling tests and the release build. Evidence:
  `retained-native-source-full-ci-local-url.log`. After confirming terminal
  success and reviewing the three-file delta, synchronized the tested reader into
  the worktree. Integrate the native bounded scanner next and run fresh final
  gates for the completed workflow. Automatic activation remains held.

## Native bounded scanning and cooperative shutdown

- Added retained-source scanning to the existing injected association source.
  Each directory inventory counts raw names before selection, does not follow
  child links, and compares descriptor observations before/after listing and
  against the freshly reopened logical directory. The ancestor-only cursor also
  compares observations between batches; changed or replaced directories reject
  that traversal rather than silently skipping names before its cursor. These
  observations are local memory, not persisted directory epochs or a filename
  queue. Originals and outside symlink targets remain untouched.
- Bootstrap now injects the watcher and retained-source media facade into the
  background discovery runtime. Its native path pages one association at a time,
  reads coalesced rescan requests through a typed existing stored-procedure
  adapter, checks the exact association snapshot, and uses existing mode,
  generation, fingerprint and job admission fences. Empty batches advance without
  issuing an invalid admission request; candidate batches use the request's
  canonical 128-file maximum. No new dependency, lease or coordinator is added.
  A review of the retained DISC-1 values then restores its initial native batch
  limits (256 processed entries, 64 selected files, 1 TiB observed primary bytes,
  two seconds and depth 32) and one-second polling. Partial scans do not wait for
  the next configured schedule interval. These two constant changes postdate the
  currently running frozen workspace snapshot and need fresh final qualification.
- Cooperative shutdown sets the scan cancellation flag and keeps awaiting its
  join before the discovery runtime returns. A completed cancelled scan does not
  submit a new candidate batch. Added an actual blocking-task regression for
  cancellation/join ordering and real retained-root tests for bounded traversal,
  symlink confinement, and directory mutation/replacement between batches.
- Application launch/compliance/bootstrap regressions pass. Workspace lint
  passes after correcting platform-specific compilation and the two small lint
  findings; final scan/shutdown lint also passes. Evidence:
  `native-association-scanner-app-tests.log`,
  `native-association-scanner-corrected-lint.log` and
  `native-scan-shutdown-lint.log`.
- The initial native workspace test invocation omitted serial execution and
  failed the root-lock reacquisition test while other children could inherit
  its lock. The unchanged lock implementation deliberately extends exclusion
  into children. That run records 414 passing app tests, one failure and the
  separately qualified service test's pre-existing ignore. The serial rerun
  against frozen `native-scan-shutdown-source.tar` passes all 417 app tests,
  with that one pre-existing service qualification ignore. Both retained scan
  tests, root-lock reacquisition and blocking scan cancellation/join pass.
  The remaining workspace packages are still running. Full Linux lint of that
  frozen snapshot also passes. Evidence:
  `native-association-scanner-corrected-workspace-tests.log` and
  `native-scan-shutdown-serial-workspace-tests.log` and
  `native-scan-shutdown-linux-lint.log`. Host formatting, instruction drift and
  whitespace checks pass; the retained-default constant changes receive fresh
  formatting and workspace lint checks separately. The typed rescan read also
  passes its actual restricted-runtime test, retaining all seven reason rows and
  the private-publisher denial/overflow checks. Evidence for the constants:
  `native-discovery-retained-defaults-fmt.log`,
  `native-discovery-retained-defaults-lint.log` and
  `native-discovery-retained-defaults-instruction-drift.log`.
- Automatic activation remains held. Watcher registration/request publication,
  schedule due progression/coalescing, restart rescan publication and guarded
  clean-census acknowledgement remain to be integrated and exercised outside-in.
  An EOF currently leaves the durable request unsatisfied. The preceding full
  CI/coverage evidence does not qualify this newer scanner implementation; fresh
  final CI and normal UI gates remain required after integration.

## Guarded rescan publication and captured completion

- The preceding frozen native workspace run completes successfully, including
  all 417 app tests and 277 data tests. Its pre-existing explicit service-test
  ignore remains separately qualified by the earlier service recovery evidence.
  Evidence: `native-scan-shutdown-serial-workspace-tests.log`.
- Added private shared catalog/association/mode fencing and two narrowly granted
  runtime procedures in the single init. Restart/uncertainty publication uses the
  existing normalized publisher. Clean completion advances only the captured
  request sequence, rejects invalid/future sequences, cannot regress satisfaction
  and does not erase newer reason occurrences. The private fence and publisher
  have no runtime grant. No lease, owner registry or new dependency is added.
- The native loop publishes restart re-evaluation before capturing pending work,
  and acknowledges guarded completion after clean EOF. A request/storage failure
  does not advance its association page cursor past that entry. Cancellation,
  changed directories, depth failure and oversized primaries leave work pending;
  depth/size failures are explicit errors instead of a manufactured clean census.
- Restricted-role procedure tests pass for disabled mode, denied private-helper
  access, stale generation/digest/version, invalid future acknowledgement,
  forbidden activation reason, monotone satisfaction and preserved newer work.
  The positive automated-mode fixture modifies only its owned disposable database
  and is explicitly synthetic procedure qualification. The packaged activation
  constraint and application holds remain unchanged; this is not native watcher
  or schedule qualification.
- `just test-media-recovery` now includes eight real native catalog/inventory
  checks on Linux, serially in minimal-feature mode. Its run passes all 377
  runtime unit tests, the existing runtime integration selection, 67 media-data
  tests, 43 application recovery tests and eight native catalog tests. Host and
  Linux workspace lint pass for the guarded-rescan implementation. Evidence:
  `guarded-native-rescan-recovery-tests.log`,
  `guarded-native-rescan-lint.log`, `guarded-native-rescan-linux-lint.log` and
  `guarded-native-recovery-selection-native-tests.log`.
- The first tooling verification passes all 1,756 tests and complete tooling
  coverage, but type checking rejects the conditional four-element selection's
  inferred fixed tuple type. Corrected its annotation to a variable-length tuple
  and added explicit Linux/Darwin CLI fixture cases. These are selection/feature
  tests, not native filesystem evidence. The corrected annotation and new fixture
  cases still need their verification; final CI remains required. Evidence for
  the preceding test pass and the reported type error:
  `guarded-native-recovery-selection-tooling-tests.log` and
  `guarded-native-recovery-selection-tooling-check.log`.
- Reviewed the root, Rust, data and DevOps instructions; updated the affected
  contracts and existing recovery recipe without relaxing gates. Watcher wiring,
  due progression/coalescing and actual automatic service/restart qualification
  are still outstanding. The newer init and runtime are not covered by the
  preceding full CI snapshot, and normal UI activation remains held.

### Guarded native schedule due observation

- The corrected variable-length recovery selection annotation passes host
  `just lint` (`guarded-native-recovery-selection-corrected-lint.log`). The new
  Linux/Darwin selection fixture still awaits functional verification.
- Added one guarded schedule observer to the existing normalized cadence state.
  It reuses the private association/catalog/mode fence, locks configured cadence,
  coalesces all overdue intervals into one schedule request, and advances due time
  in the same transaction. Missing cadence and a future due time return absence.
  Request overflow rolls back cadence advancement. No default cadence, new table,
  supervisor, lease, dependency or production activation change is introduced.
- Native polling observes schedule mode even when watcher mode is also enabled;
  a pending schedule reason selects schedule admission. Added a restricted-role
  test covering disabled mode, missing cadence, immediate first due, repeat
  observation, eleven coalesced intervals, stale digest, forbidden trigger and
  publication rollback. Owned SQL fixtures remain synthetic DB qualification.
- Reviewed root, Rust and data instructions and updated the affected contracts.
  This implementation and its new regression are awaiting execution. Watcher
  wiring and actual automatic service qualification remain outstanding; fresh
  full CI and normal UI gates are still required.

- Executed qualification: `just test-media-recovery` passes 377 runtime unit
  tests, five runtime integration tests (one pre-existing explicit ignore),
  68 media-data tests, 43 application recovery tests and eight native catalog
  tests. The new guarded schedule regression passes. Host and Linux `just lint`
  pass. `just tooling-cov` passes all 1,757 tests and complete coverage thresholds;
  `tasks/testing.py` has 100% statement and branch coverage, including both
  platform selections. Evidence: `guarded-native-schedule-recovery-tests.log`,
  `guarded-native-schedule-lint.log`, `guarded-native-schedule-linux-lint.log` and
  `guarded-native-schedule-tooling-tests.log`. This replaces the pending scoped
  verification above; it does not claim a new full CI/UI or automatic workflow pass.
- Qualification setup: the first Linux invocation stopped before tests because
  an earlier root-run tooling installation left its package metadata root-owned.
  Corrected only that dedicated environment's metadata ownership to its existing
  recovery user and reran the same recipe successfully. No product permission,
  schema criterion or test assertion was relaxed.

### Retained association watcher integration

- Native association discovery now supplies validated retained directory
  descriptors to the injected existing notify adapter. Descriptors remain alive
  while watches use `/proc/self/fd/<fd>/.`; descendant symlink following is
  disabled. Callback paths are hints only: candidate reads, fingerprints and
  admission still use retained roots and exact stored-procedure fences.
- The native debounce route coalesces binding rescan hints with a one-second quiet
  interval and five-second maximum residence. Watch registrations are bounded at
  the retained 128-instance limit, and callback storage checks the retained
  per-root 256-record/2-MiB and instance 1,024-record/8-MiB bounds before cloning.
  Overflow, backend errors, explicit rescan flags and pathless change events
  retain uncertainty. Failed publication stays pending while other due entries
  are visited. Page-cycle pruning removes obsolete registrations.
- Added adapter tests for pathless/backend uncertainty and record/byte overflow,
  a real Linux descriptor-based nested event and outside-symlink rejection test,
  retained-root watch descriptor checks, and an injected-catalog application
  regression for actual notify/admission plus object restart de-duplication.
  The last test uses owned synthetic binding activation and real media paths;
  it is not packaged automatic activation or serving-process restart evidence.
- The first lint run found a production-only `sqlx` import and style issues;
  corrected these by injecting the existing MediaStore instead of adding a
  dependency. A second run passes before the new workflow regression. The
  regression's direct version lookup then failed the data-boundary policy;
  moved that setup into its owned fixture SQL without relaxing the guard.
- Reviewed root, Rust and data instructions and updated the Rust contract. No
  architecture, lease, owner protocol, dependency or activation hold change.
  Fresh verification is pending; automatic background/process qualification,
  fair continued scans and clean absence diagnostics still need completion.

- Corrected the Linux fixture's synchronous `TestDatabase::close` call after
  compilation rejected an incorrect await. Separated backend/rescan uncertainty
  from callback buffer overflow so published reasons stay accurate. Current
  host `just lint` passes (`native-watch-workflow-sync-cleanup-lint.log`).
- The next full Linux run initially stopped in existing asset-sync setup: source
  files extracted as root were not writable by the dedicated recovery user.
  Restored ownership only for the named qualification checkout's authored-file
  manifest and parent directories, preserving the configured service identity.
  No product/test policy was relaxed. Future source copies must preserve that
  ownership instead of replacing it with root ownership.
- The corrected owned-checkout `just test` run passes all five new watcher cases:
  quiet/max-residence debounce, callback uncertainty, root record/byte bounds,
  descriptor-based nested creation with outside-symlink rejection, and actual
  association notify/admission/object-restart de-duplication. It also passes
  retained catalog watch-directory checks. Evidence:
  `native-watch-workflow-owned-workspace-tests.log`. The complete workspace run
  is still live; these individual successes do not claim its final result, fresh
  full CI/UI, or serving-process/packaged activation qualification.

### Serving-process discovery recovery qualification

- The preceding owned Linux `just test` completes successfully: 422 application
  tests (one existing explicit service-fixture ignore), 279 data tests and all
  remaining workspace/integration/doctests pass. Evidence:
  `native-watch-workflow-owned-workspace-tests.log`. Linux lint separately flags
  the enlarged retained-scan test at 107 lines; extracted its watch-directory
  assertions into a helper, preserving every assertion and native check.
- Extended the existing explicit `just test-media-service-recovery` selection
  rather than adding another recipe or bypassing its one-test execution guard.
  Two new serving-process phases use short real FFmpeg media, a real declared
  native catalog on the persistent mount, and 131 primaries so observation can
  stop an unfinished bounded scan. Both watcher and explicitly configured
  schedule modes retain pending requests across joined shutdown, preserve
  admitted job IDs, finish rescan after restart, admit a later file via their
  trigger, then check a further clean restart for de-duplication and unchanged
  originals. The original active-FFmpeg/checkpoint/source/replacement phases
  remain in the same selection.
- Mode enablement and overdue-clock simulation are exact owned-fixture SQL.
  Configuration otherwise enters through the real API and constrained runtime
  role. These phases qualify native background/process behavior without claiming
  the packaged operator activation hold was lifted or qualified. Job observation
  uses existing bounded stored-procedure pages, not direct application SQL.
- Reviewed root, Rust and data instructions and updated the explicit service
  recovery contract. Host `just lint` passes for the current source; new Linux
  lint and explicit serving-process qualification are pending. Fresh full CI/UI
  and remaining whole-workflow requirements are still required.

- Current Linux `just lint`, `just fmt`, instruction-drift checks and
  `git diff --check` pass. The corrected retained-scan helper preserves every
  descriptor assertion. Evidence: `native-service-discovery-linux-lint.log`,
  `native-service-discovery-host-lint.log`,
  `native-service-discovery-final-format.log` and
  `native-service-discovery-instruction-drift.log`.
- Executed `just test-media-service-recovery` successfully: the one explicitly
  selected native case passes in 630.75 seconds. Its unconditional watcher and
  schedule phases run before all retained active-FFmpeg, cancellation, changed/
  missing-source, missing/corrupt-checkpoint, prepared/committed-replacement
  phases. Each serving child is signalled and joined before replay. The two
  discovery phases assert an unsatisfied request at interruption, preserve
  already admitted IDs, finish all 131 original candidates after restart, admit
  a 132nd candidate through the configured trigger, and preserve the complete
  identity set on another restart. Original bytes remain unchanged throughout
  these dry-run discovery phases. Evidence:
  `native-service-discovery-qualification.log` and frozen
  `native-service-discovery-source.tar`. Successful Rust test capture reports
  the complete selected-case result; it does not emit its captured println
  progress markers. Scope comes from the inspected unconditional case body,
  not from invented log excerpts or a fake serving fixture.
- All fixture databases/roles and native temporary directories are closed by
  the existing finalizers; the native qualification container is idle after
  completion and retained for upcoming work. No product activation, CI/UI,
  fairness/absence, coverage, Sonar, package or release pass is claimed beyond
  the named evidence. Remaining whole-workflow acceptance stays required.

### Clean absence tracking in the existing fingerprint records

- ADR 594 retains two clean absence observations; the retained DISC-1 values
  require at least one second between them and reset evidence on uncertainty.
  Added presence sequence and diagnostic absence fields to the existing normalized
  fingerprint rows. No observation table, lease, ownership service or deletion
  authority was introduced.
- Native admission records known presence only after successful admission or
  unchanged-fingerprint deduplication. Unstable/rejected candidates and failed
  scans preserve uncertainty through the existing guarded request procedure;
  they cannot acknowledge a clean census. A complete latest request records
  missing known paths and preserves a confirmation request after the first pass.
  Old acknowledgements and scans superseded by newer requests do not count.
- Guarded presence batches retain the API's 128-path limit and reject stale
  authority, invalid relative paths and already acknowledged/future sequences.
  Positive presence clears absence. Diagnostic reads select one path from the
  latest association and require the same ready active catalog generation before
  reporting confirmed absence. Jobs and files are never deleted by this evidence.
- Added restricted-role database regressions for tentative/confirmed absence,
  minimum observation separation, replay, reappearance, uncertainty, newer
  requests, stale authority and invalid/beyond-cap input. Extended the existing
  real notify/admission test with file removal and reappearance. Execution results
  for these changes are pending; earlier passes do not qualify this snapshot.
- Reviewed ADR 594, its retained DISC-1 values, Rust and data instructions.
  Updated the scoped data contract; no conflicting rule or new dependency found.
  Rollback is removal of this unreleased candidate change, preserving existing
  fingerprints and immutable job semantics. No caller-owned database is migrated.

- Scoped execution: `just test-media-recovery` passed on Linux: 377 runtime
  units, five integration cases (one existing explicit fixture ignore), 70 data
  media tests including both new absence cases, 43 application recovery tests
  and eight native root-catalog cases. Evidence:
  `native-clean-absence-recovery-tests.log`. This snapshot predates the added
  notify-driven absence/reappearance phase.
- The later application-test snapshot passed Linux `just lint`; host lint,
  formatting, instruction drift and whitespace checks also passed. Full Linux
  CI is running on `native-clean-absence-app-source.tar`. Its initial URL
  preflight rejected the direct query-bearing test endpoint; the retry selects
  the owned container's approved local URL through `E2E_DB_ADMIN_URL`, retaining
  the direct restricted-role endpoint for tests. No endpoint guard was relaxed.

### Watch uncertainty before clean completion

- Found an implementation ordering gap while inspecting the actual native loop:
  debounce publication errors were logged independently, while a later scan could
  acknowledge completion without checking pending watcher hints. This could count
  a second missing-source observation even though uncertainty had not persisted.
  It is an implementation defect under the approved clean-census rule.
- Native ticks now drain callbacks before scanning and again before completion.
  Pending debounce/publication hints preserve a guarded rescan rather than a clean
  acknowledgement. A failed scan reset retains only its captured binding in local
  memory and retries before scanning; obsolete association/generation state is
  dropped. No coordination service, supervisor, lease or new persistence table.
- Added an owned constraint fault that rejects absence resets for the missing
  `nested/second.mkv` while still allowing present-source observations and a
  completion increment. The existing native watcher case establishes a real first
  absence observation, injects backend/publication failure, waits past the retained
  minimum separation, and proves repeated ticks cannot confirm absence. It clears
  the fault and verifies reappearance. The fixture constraint is removed even when
  qualification returns an error; source bytes are restored and cleanup failures
  are reported.
- The fixed native snapshot passed `just lint` and complete `just test`: 422 app
  tests (one existing explicit service ignore), 281 data tests, 377 media-runtime
  tests and the remaining workspace/integration/doctests. Evidence:
  `native-watch-ordering-workspace-tests.log` and
  `native-watch-ordering-linux-lint.log`. This predates the new portable regressions.
- Fresh host CI passed its application/data suites, minimum-feature tests and all
  preceding gates, then failed the unchanged application coverage threshold:
  21,124/23,968 lines (88.13%, minimum 90%). Evidence:
  `native-clean-absence-host-full-ci.log`. The report identified the common native
  coordination logic as largely uncovered on that host. Added portable scripted
  watcher and schedule/scan-reset regressions using the existing injected source,
  actual filesystem fingerprint reader and restricted procedures. Scripted event
  hints remain non-authoritative; real notify, native catalog and serving-process
  qualification remain separate Linux evidence. Coverage has not yet been rerun.
- Host lint passed for those portable regressions. All final CI/UI and remaining
  workflow qualification are still required. Reviewed Rust/data instructions and
  ADR 594; updated the Rust contract. No new dependency or criteria relaxation.

### Portable ordering qualification and retained traversal limits

- The frozen portable-ordering snapshot passed full host `just ci`, including
  release compilation, 1,757 tooling tests, 93% complete tooling coverage and
  application Rust coverage of 22,145/24,494 lines (90.41%). Evidence:
  `native-watch-portable-host-full-ci.log`.
- The same native snapshot passed the explicit instrumented service recovery
  selection: one test, 671.05 seconds. It exercised watcher/schedule restart,
  active FFmpeg shutdown and explicit cancellation, source change/loss,
  missing/corrupt checkpoints and prepared/committed replacement recovery.
  Combining its profiles with complete workspace coverage passed every crate;
  application Rust coverage was 22,912/25,199 lines (90.92%). Evidence:
  `native-watch-portable-instrumented-service-recovery.log`,
  `native-watch-portable-linux-combined-coverage.log` and the retained
  `native-watch-portable-linux-qualified-coverage` reports. No service child or
  owned service fixture remained after completion.
- Normal UI verification with the actual persistent catalog and matching
  `/srv/revaer/ui` filesystem passed 55 API cases and failed schedule/watcher
  activation with `media_configuration_pending_contract`. This is not a UI
  pass. The first attempt used the default unrelated fixture path and failed
  setup; that environment error was corrected before the normal rerun.
- The activation audit found missing retained whole-scan traversal counters.
  Added them to the existing in-memory cursor: 65,536 examined entries,
  4,096 directory visits, 16,384 selected primaries, 16 TiB primary bytes and
  six hours of traversal metadata work. Complete inventories, including rereads,
  count before cursor filtering; the raw reader receives the remaining entry
  limit. Exhaustion leaves the scan incomplete. No new persistence or dependency.
- The exact scan-bound patch passed Linux lint. Both new real-filesystem
  boundary tests passed, and the application suite passed 426 tests with the
  existing separate service qualification ignore. The remainder of complete
  `just test` is still running. Its host preimage and resulting source hashes
  are retained in `native-run-bounds-manifest.json`. Earlier CI/service/coverage
  passes precede this new patch and do not qualify the final snapshot.
- Aggregate admission accounting, metadata work outside traversal and cooperative
  cancellation during fingerprint reads remain unfinished. Automatic activation
  remains held. Fresh final CI/UI and applicable release checks are still required.
- Complete Linux `just test` passed for the scan-bound snapshot: 426 app tests
  (one existing separate service ignore), 281 data tests, 377 media-runtime tests,
  and all remaining workspace/integration/doctests. Evidence:
  `native-run-bounds-linux-workspace-tests.log`. Host lint exposed two platform
  representation issues: the native budget initializer needed the same Linux/test
  condition as its field, and the legacy batch constructor needed a const empty
  progress constructor. Applied those semantic-preserving repairs on both
  checkouts; host lint then passed in 17.57 seconds. Evidence:
  `native-run-bounds-host-lint-final.log`. Updated the retained patch and exact
  source hashes against the frozen portable-source archive. These repairs do not
  lift automatic activation or qualify the pending cancellable-reader draft.


### Cooperative aggregate reads and joined job outcomes

- Aggregate fingerprint reads now check the caller's cancellation signal before
  each 64-KiB read and each member. Native association admission uses its existing
  shutdown flag and stops before accepting another candidate. Job capture and
  revalidation use the existing job-control monitor; shutdown interruption remains
  resumable, while explicit cancellation remains terminal. Reads and monitors are
  joined before persisting either outcome. No persistence, dependencies or
  coordination protocol were added.
- Both host and Linux `just lint` passed. Host `just test-media-recovery` passed
  362 runtime unit tests, five runtime integration tests, 70 media data tests and
  45 selected job-runtime tests. The existing separate runtime integration ignore
  is unchanged. Both new joined-fingerprint outcome regressions passed on host.
- Linux application tests passed 429 cases with the existing explicit serving
  qualification ignore. The new physical-read regression stops after exactly
  64 KiB of a real 1-MiB file, preserves the original bytes and confirms normal
  fingerprint equivalence. Both new job regressions prove the reader stopped
  before same-attempt replay or terminal explicit cancellation. Remaining Linux
  workspace checks are still running.
- Evidence: `cancellable-reader-host-lint-final.log`,
  `cancellable-reader-linux-lint.log`, `cancellable-reader-host-recovery-tests.log`,
  `cancellable-reader-linux-workspace-tests.log` and
  `cancellable-reader-source-manifest.json` in the ignored task evidence folder.
  The ten-file archive records the exact synchronized source snapshot.
- Aggregate admission accounting and metadata work outside traversal still need
  completion. Automatic activation remains held; fresh full CI/UI, current native
  serving qualification and applicable release checks remain required. Rollback
  removes this unreleased reader change without changing job records. Reviewed
  the Rust scoped instructions; no instruction drift or dependency change found.


### Restored-source cancellation and initial file bounds

- The completed cancellable-reader snapshot passed complete Linux `just test`:
  429 application cases (one existing explicit service ignore), 281 data cases,
  377 media-runtime cases and remaining workspace/integration/doctests. Its host
  CI passed the ordinary and minimal-feature test phases, then stopped in
  coverage when one fixture failed before execution during PostgreSQL SSL
  negotiation. The coverage application phase passed 416 other cases. The owned
  test database reports `ssl=off`; subsequent explicit test URLs disable that
  negotiation. This is test-connection configuration, not a quality-gate change.
- Fresh actual service qualification failed after 560.75 seconds when the
  checkpoint replay checked replacement artifacts immediately after database
  completion. Production commits completion before backup finalization and
  publishes completion afterward. Moved the same artifact assertion after the
  existing service shutdown-and-join barrier; no polling budget or assertion was
  weakened. The stopped service and FFmpeg children were absent afterward.
- Corrected the reader to the approved initial DISC envelope: 256-GiB primary,
  1-GiB individual sidecar, 4-GiB total sidecars, 32 logical groups and 65 physical
  members. Metadata checks reject oversize before hashing. Reads stop at the
  observed member length and reject an early EOF; descriptor identity checks
  still reject mutation. The prior 256-MiB total sidecar cap was an implementation
  mismatch, not an approved design change. No automatic escalation to hard
  policy ceilings was added.
- Restored-source hashing during startup and per-job rollback now receives the
  existing shutdown receiver. Its read joins before returning interruption and
  retains the existing backup/manifest until guarded source refresh succeeds.
  Startup stops admitting further reconciliation or event work after shutdown.
  The new regression exercises a real committed replacement, interruption while
  hashing the restored original, retained original backup/manifest, then joined
  same-attempt recovery on restart.
- The focused `just test-media-recovery` recipe now runs fingerprint regressions
  first. Updated its actual-execution tooling fixture and Rust scoped contract.
  Latest production code passed Linux lint; the final startup guard and all new
  runtime regressions still need execution. Aggregate admission accounting and
  metadata outside traversal remain unfinished; activation stays held. Fresh
  final CI/UI, strict Sonar, native service and applicable release checks remain
  required.
- Evidence is retained in the ignored task evidence folder: reader CI/service
  logs, `initial-fingerprint-bounds-linux-lint-repaired.log`,
  `rollback-hash-cancellation-linux-lint-final.log` and the seven-file
  `rollback-hash-cancellation-source.tar`. No dependencies, persistence or
  coordination protocols were added. Reviewed Rust scoped instructions; added
  the focused recipe and join semantics there, with no stale reference removed.

- Executed the final guarded snapshot: host `just lint` passed; Linux focused
  `just test-media-recovery` passed all 11 fingerprint, 377 runtime unit, five
  runtime integration, 70 media-data, 46 job-runtime and eight retained-root
  tests. The existing separate runtime integration ignore remains. The new
  rollback-hash interruption test passed with a real committed replacement,
  retained original backup and `replacement.json`, followed by same-attempt
  reconciliation after the stopped read had joined. Evidence:
  `rollback-hash-cancellation-host-lint.log` and
  `rollback-hash-cancellation-linux-recovery-tests.log`.
- Host `just tooling-check` passed 1,756 tests but failed the real container
  build when the ephemeral builder's pinned Dockerfile frontend lookup chose an
  unreachable Docker Hub IPv6 endpoint. No source assertion was relaxed. A fresh
  read of the exact frontend manifest then succeeded. The serving-process
  recovery rerun remains active; neither that nor current full CI/UI is claimed
  passed. Reviewed the Python scoped instructions alongside Rust.

- The current actual Linux service recovery rerun passed in 643.69 seconds,
  including automatic discovery interruption/restart, real active FFmpeg stop
  and same-attempt replay, explicit cancellation, source change/loss,
  missing/corrupt checkpoint output and prepared/committed replacement recovery.
  The unchanged replacement-artifact assertion passed after the owner join.
  Evidence: `rollback-hash-cancellation-native-service-recovery.log`. No service
  or FFmpeg children remained after the terminal check. This owned fixture's
  synthetic mode activation still does not certify packaged automatic activation.
- Host `just tooling-check` rerun passed all 1,757 tests in 367.33 seconds;
  the real pinned-frontend container build also passed. Evidence:
  `rollback-hash-cancellation-host-tooling-check-retry.log`. No Docker network
  setting, source assertion or criteria was changed. Full current CI/UI,
  aggregate admission/metadata accounting, strict Sonar and applicable release
  checks remain required; the earlier coverage provisioning failure is retained.

### Aggregate admission accounting (qualification pending)

Native traversal already counted primary bytes, but admission had not included
sidecars in its batch/run accounting. The same aggregate fingerprint size now
feeds the accepted initial 1040-GiB batch and 16.25-TiB run limits. A fitting
candidate that exceeds the remaining batch is retained with the bounded current
batch and cursor for the next tick; clean census acknowledgement waits for it.
No durable frontier, quota ledger, policy field or dependency was introduced.

The existing reader reports member hashing duration separately from metadata
execution. Traversal, binding/admission lookup, fingerprint metadata and enqueue
service time count toward the existing two-second batch quantum and six-hour
run limit. Checks are cooperative between operations; they do not claim syscall
preemption. Failed/over-limit scans retain the existing uncertainty/rescan path.
Native admission boxes its existing future because the compiled worker frame
exceeded the enforced Clippy size limit after accounting was added; this is one
allocation per batch, not another worker or coordination mechanism.

New fixtures exercise exact aggregate limits, batch yield, run exhaustion and
metadata yield/exhaustion. A database fixture uses real source reads to queue one
candidate, retain the next, then admit it in a fresh batch with both originals
unchanged. These additions are not qualified until executed checks pass.
The Rust scoped instructions were reviewed and updated. No criteria or
architecture changes were made; normal activation and all final gates remain
pending.

### Empty-admission metadata yield correction

The first aggregate-bound snapshot passed Linux workspace tests (437 app tests,
281 data tests and 377 media-runtime tests, plus other workspace/integration/doc
checks); the explicit active-service test remains separately selected. Host lint
also passed. Review then found that an exhausted metadata quantum with zero
admissions would send an empty presence list to a procedure that rejects it.
The native loop now retains the pending batch without publishing that empty list.
A deliberately delayed metadata scan must queue nothing or acknowledge no EOF
on its first tick, then admit its retained candidate and finish on the next tick
with original bytes unchanged. The focused recovery recipe now includes existing
association/admission regressions and its real Rust fixture asserts that selection
actually executes. Qualification of this correction is pending.

Reviewed the Rust and Python scoped instructions. No source suppressions, gate
weakening, new dependencies or durable coordination state were introduced.

The delayed-metadata regression initially asserted that all current requests
were satisfied on tick two. Diagnostic output proved the captured request 2
was satisfied, while a newer watcher-uncertainty request 3 remained pending
(`configuration_activated`, `restart_reconcile`, `watcher_uncertain`: requested 3,
satisfied 2). The implementation preserved the required fence. The test now
asserts exact satisfaction of the captured high-water mark, retaining real
notify and the newer pending request semantics. The temporary scan trace was
removed; both failing diagnostic runs remain evidence. No production fence,
watcher predicate, stored procedure or wait budget was relaxed.

Host `just ci` passed for the preceding aggregate-admission snapshot, including
425 app tests, 281 data tests, complete workspace/minimal selections, Rust
per-package coverage (app 22957/25384 = 90.44%), tooling coverage and release
build. Exact scoped source hashes and logs are retained in
`aggregate-admission-host-ci-qualified.json`. This predates the empty-presence
guard and corrected captured-fence fixture; it is not final qualification.
The current Linux six-file guarded snapshot was copied to the isolated Host
checkout only after CI terminated. Normal automatic API activation, UI, current
service recovery, strict Sonar and remaining package checks are still pending.

### Ordinary automatic-mode activation (final qualification pending)

The guarded focused recovery selection passed: 11 fingerprint, 82 association/
admission, 377 runtime plus five integration, 70 data, 46 job/checkpoint and eight
retained-root tests. The captured-fence delayed metadata test passed with real
notify. Host lint and all 1757 tooling tests passed (383.03 s), including the
added real Rust admission-selection fixture. The active native notify regression
is Linux-only because its retained hints use `/proc/self/fd`; portable aggregate
batch admission remains tested on both hosts.

With the bounded loop integrated and exercised, remove the precise temporary
automatic-mode constraint and two writer rejections. Ordinary create preserves
requested mode flags; import preserves them only for active, resolved bodies.
Active imports now publish the same existing configuration-activation rescan
as create, in the same transaction, so existing files do not await a later hint.
Keep root, immutable version, actor, overlap, disabled-mode and admission fences.
Retire the unused pending-contract error mapping and only the obsolete hold
removal from owned synthetic mode-transition scripts. Those scripts remain
procedure fixtures, not operator-activation evidence.

A restricted-runtime data fixture exercises watcher, schedule and combined
creation plus serializable import/audit; each must preserve mode flags and
publish its initial pending rescan. The retained normal API/UI assertions are
unchanged. Reviewed Rust, Python, data and UI scoped instructions; data/UI
instructions now describe integrated activation. No dependency, architecture,
quality criterion or runtime table grant changed. Normal UI, current active
service recovery, fresh CI, strict Sonar and applicable release checks remain
pending.

### Current recovery qualification and scanner prerequisite

The ordinary-activation snapshot passed fresh `just ci`, including workspace and
minimal-feature tests, tooling coverage and release build. App Rust coverage was
22959/25387 lines (90.44%). Normal `just ui-e2e` passed all five phases: 3 negative
authentication, 57 positive API, 3 negative API-key, 57 browser API and 22 media UI
tests. The explicit current Linux service-recovery test passed in 681.87 seconds;
no service, FFmpeg or test child remained after it terminated. This proves the
current local recovery and ordinary activation paths, not hosted or package gates.
The current Linux whole-workspace native coverage collection remains active.

Analysis setup then exposed a bounded tooling defect: the hash-verified and
signature-verified pinned Linux arm64 scanner archive contains internal JRE
license symlinks, which its extractor rejected. Materialize only targets that
are regular members inside that same verified archive, preserving their bytes
and permissions without creating filesystem links. Reject escaping, dangling
and chained targets and retain the expanded-byte limit. Meaningful extraction
fixtures exercise these boundaries. No scanner property, analysis criterion,
dependency or application behavior changes. Reviewed Python and DevOps scoped
instructions; DevOps now records this installation behavior. Qualification of
this prerequisite fix, strict Sonar and applicable delivery checks is pending.

### Step-5 closeout boundary, 2026-10-05

The operator explicitly directed completion of cancellation/checkpoint recovery,
its focused tests, full CI, normal UI E2E, a scoped conventional commit and cleanup.
Preserve earlier analysis/package work and carry remaining strict Sonar and package
qualification into goal 7. This changes the staged delivery boundary, not any
scanner property, threshold, assertion or release criterion.

The interrupted preceding run made concrete progress: all 1762 Host tooling tests
passed (386.11 s), the verified pinned native scanner installed successfully, and
instrumented real-service recovery passed (1063.59 s). Both Linux architectures
built into the local OCI archive, with image digest
`sha256:abebd8187312649aed44221c6761dcf99d36e1139a5a6560552e2e7edb9223da`.
Keep its archive, metadata, scanner installation, raw profiles and measured
bootstrap evidence for goal 7. Image construction alone is not complete package
or release qualification.

The latest full-CI retry failed with 280 data tests passing and two failures:
restricted-baseline initialization exceeded its existing statement timeout, and
an indexer test received an invalid PostgreSQL message byte. Both selected URL
helpers preserve the explicit endpoint and its SSL query. Do not change SQL,
timeouts or TLS policy based on these failures; replay the gate without concurrent
package and test workloads before deciding whether implementation changes are
needed. The browser-analysis retry first omitted explicit lifecycle credentials;
after correcting those inputs, its initial factory-reset request lost the HTTP
connection. Its service log records reset execution followed by a joined service
exit. Recheck normal E2E in isolation with the owned catalogue and filesystem root.

Linux tooling coverage finished with 1688 passed, 27 failed and 47 errors. Its
diagnostics include missing kcov at collection start (subsequently installed;
all four measured bootstrap cases then passed), missing native CLIs and Firefox/
WebKit executables, and Docker fixtures using client-local loopback addresses
and bind paths from inside a container. These are analysis-runner dependencies,
not evidence that the recovery design is flawed. Keep the complete diagnostics;
broader Linux analysis-runner qualification belongs to goal 7. Host full CI still
must exercise all its ordinary tooling tests and gates.

The first resumed focused run passed fingerprint/admission/runtime, all 71 media
data tests and all 46 checkpoint/job tests, then passed six of eight retained-root
tests. The two native activation proofs correctly rejected unproven persistence:
the invocation omitted the existing `REVAER_NATIVE_RECOVERY_ROOT` input. Final
Linux recovery checks explicitly select the owned `/srv/revaer` ext4 mount; do
not grant an overlay filesystem persistence or bypass either proof assertion.

Acceptance evidence is retained under `target/cancellation-recovery-evidence`:
`step5-final-source-identity.json` records every authored file's bytes and mode;
`step5-final-validation.json` binds actual command results to their revision,
source identity and execution environment. The final focused, native-service,
CI and UI logs use the `step5-final-` prefix. Results are recorded only after the
corresponding process terminates, and final CI/UI run on the committed snapshot
without source mutations. These references allow the final acceptance record to
be produced after commit without modifying the tested source.

Reviewed root policy, ADRs 593–594, the scoped Rust/data/UI/Python/DevOps rules,
Justfile, recovery selectors and E2E lifecycle wiring. Corrected the stale header
claim that automatic activation still failed; its implementation and earlier
normal UI pass are recorded above. No dependency, architecture or gate weakened.
