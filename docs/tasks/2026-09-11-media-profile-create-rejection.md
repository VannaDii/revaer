# Media Profile Create Rejection

- Status: In progress
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Bounded profile fixtures and API regression tests;
  parent-reviewed local integration, unpublished, runtime acceptance pending.
- Approved authority: [Specification](../../MEDIA_TRANSCODING.md),
  [ADR 419](../adr/419-pr76-media-data-review-remediation.md), and the existing
  [ADR 557](../adr/557-root-persistence-contract.md) /
  [ADR 559](../adr/559-media-approval-delta.md) cutover boundary. The operator's
  [ADR 588](../adr/588-first-release-decision-package.md) approval remains intact.
- Requirement ledger: [Media completion ledger](../adr/564-media-completion-ledger.md),
  configuration and discovery evidence; the parent recorded the bounded proof
  there without changing any approval status.

## Motivation And Design Notes

The retained full E2E failure at `tests/specs/api/media.spec.ts:130` expected
profile creation to return 201 but received 400. Its body enabled scheduling
with a 60-minute interval through the legacy path-taking create API.

This is an invalid fixture, not a demonstrated production rejection defect:

- ADR 419 explicitly makes legacy creation insert-only and rejects automation
  without verified normalized roots. The specification requires new profiles
  to be dry-run and automation disabled by default, without a default cadence.
- The API handler forwards scheduling to `MediaService`, which calls the
  `MediaStore` profile adapter and frozen `media_profile_upsert_v2` procedure.
  Migration 0182 lines 1306-1308 deliberately reject scheduling or watching with
  `media_profile_filesystem_identity_required`. The application maps that
  detail to an invalid request; the API retains SQLSTATE `P0001` as HTTP 400.
- The initial correction disables scheduling and omits the create interval.
  Retain its exact 201 assertion and add error-body diagnostics to create and
  the immediately following profile PATCH assertion. No validation, expected
  status, route-coverage requirement, production code, or migration changes.
- Add three profile API regressions: safe persisted creation even when the
  caller requests non-dry-run, rejected scheduler creation, and rejected
  watcher creation. The latter two require the exact error context and absence
  of a persisted profile. All three verify unchanged source bytes and remove
  their private temporary trees in `afterEach`, including assertion failures.

## Test Coverage Summary

- Exact runtime source: `62c5d6a7e604eb2a112fd2578ae650a6900a7a9a` plus a temporary
  test-only host in the media test module, retained as evidence and removed
  before commit. C1 `81947a65` is intentionally not in this reproduction base.
- The host injects existing configuration, indexer and production media
  services, with the existing static capability detector; it starts no UI,
  background media worker or native process. HTTP and persistence are real.
  It supplies no compliance manifest and provides no package/compliance proof.
- PostgreSQL 16 image:
  `postgres@sha256:57c72fd2a128e416c7fcc499958864df5301e940bca0a56f58fddf30ffc07777`.
  Owned container: `revaer-profile-create-proof-pg-20260911`; loopback port
  56830. `DATABASE_URL`, `REVAER_TEST_DATABASE_URL` and `E2E_DB_ADMIN_URL` were
  explicit. Each host used a disposable child database and frozen migrations.
  API ports were 58931 and 59444; port 7070 was never used.
- Focused Just command-mode Cargo host run: one host test passed in each run,
  with explicit termination and database cleanup. This is not `just test` or
  full CI. An earlier build-only invocation was interrupted before tests;
  a sandbox-blocked host attempt failed rather than silently skipping.
- New API regressions via the retained diagnostic Playwright config: **6/6
  passed**, zero skipped and no warnings in the final run, across anonymous and
  API-key auth. The initial assertion-shape mistake and color-environment
  conflict remain in their original failed-run log; the final assertions use
  the API's actual name/value context array.
- Original request reproduced at line 130: **5 passed, 1 failed, 3 not run** in
  the diagnostic dependency selection, including the three new anonymous cases
  and the original three media cases. The corrected original media scenario:
  **2 passed, 1 failed**, now at line 154 after successful create/list/get.
  The next PATCH sets `schedule_interval_minutes: 120` and receives the same
  identity rejection from `media_profile_update_v1` (0182 lines 1417-1425).
  That original failure remains retained; the continuation below corrects the
  metadata-only PATCH fixture without relaxing its status assertion.
- `just fmt`, focused policy guardrails, local schema generation and TypeScript
  `--noEmit` checks passed.
  `just api-test-client` failed at npm audit because sandbox DNS could not
  resolve the registry; no successful dependency audit is claimed.
- The initial `just instruction-drift` failed because E2E edits require a scoped
  instruction update. The operator subsequently authorized the matching UI
  instruction and routine-task navigation edits; the continuation addresses it.
  `just ci` and `just ui-e2e` remain pending parent integration/revalidation.
  No full UI run, coverage waiver, upload, push, or release claim was made.
- Durable local evidence, including the host source, both configs, exact commands,
  JSON test counts and original failure logs:
  `/Users/vanna/Source/revaer/artifacts/media-verification/2026-09-11-profile-create-proof/`.

## Continued Profile And Scenario Proof

The reviewed initial correction is commit
`6fbf4ab038d94ae124ef8a53f29d4125b22a3d84`. This continuation changes only
fixtures, regression tests, the matching UI instruction and this task's
navigation/record. It does not implement missing root workflow or alter C1.

- Frozen `media_profile_update_v1` (0182 lines 1417-1425) rejects any supplied
  interval, including with scheduling disabled, and rejects watcher/schedule
  enablement without verified identity. Remove the interval from the retention
  PATCH and the post-import metadata PATCH. The validation success fixture now
  also requests disabled automation without a cadence. Keep all exact success
  assertions, including the later actual schedule/watcher enablement and runs.
- Import/export already forces dry-run, disables automation and omits cadence
  (`media_yaml_active_profile`); `existing_profile_matches_yaml` requires the
  same safe representation. The corrected post-import PATCH retains the same
  source/output roots rather than rebinding unverified paths.
- Add a positive persisted metadata update and four discriminating negative
  updates: schedule enablement, interval alone, interval with scheduling
  disabled, and watcher enablement. Every rejected update asserts HTTP 400,
  the exact identity/P0001 context, the entire unchanged persisted profile
  (including retention), and unchanged source bytes. The watcher-create
  negative now supplies no interval, so an invalid cadence cannot satisfy it.
- Add independent API coverage for a real manual dry-run admission, duplicate
  suppression, profile readiness, planning/discovery previews, job list/detail,
  all seven diagnostic reads, cancel/retry, and unchanged profile/source state.
  The job ID comes only from successful manual admission; no worker-owned write,
  injected job row, root identity, or package witness is manufactured. Separate
  disabled watcher/schedule run tests assert rejection and zero admitted jobs.

One bounded anonymous-auth run of both media API files produced **13 passed,
1 failed, 0 skipped, 0 flaky**. The original three-test file produced **2 passed,
1 failed**; all eleven profile regressions passed. The full scenario passed
creation, metadata update/validation, both target catalog operations and target
pinning, policy operations, invalid profile controls, retention, capability and
profile readiness reads, export, both valid imports, invalid import controls,
restored-profile metadata, and disabled schedule listing. It then failed at
`tests/specs/api/media.spec.ts:484`: schedule enablement expected 200, received
400 with operation `media_profile_patch`, error code
`media_profile_filesystem_identity_required`, SQLSTATE `P0001`.

The later positive watcher enablement, watcher/scheduled runs and combined job
reads in that monolithic scenario were **not reached**. Their assertions remain
unchanged. The independent manual test passed those job-read routes using its
own genuinely admitted dry-run job; this does not prove enabled automation or
pass the full route-coverage gate. The current specification still includes
enabled watcher/scheduled discovery. ADR 419 requires verified roots; ADR 557's
approved replacement root contract is not wired through this legacy profile
surface. Completing that capability is separate from fixture correction.

Parent review then requested removal of the watcher-create test's unrelated
interval. After verifying the first run left no media, the corrected watcher-only
case was rerun against the same host: **1 passed**, zero skipped. The first
run's exact pre-refinement source and counts remain preserved alongside the
final source. No new full API run or API-key continuation run is claimed.

Runtime proof still uses base `62c5d6a7` plus the retained temporary injected host;
the tested fixture delta is over `6fbf4ab0`. PostgreSQL uses the same pinned image
above, owned container `revaer-profile-patch-proof-pg-20260911`, loopback port
62635 and explicit `DATABASE_URL`, `REVAER_TEST_DATABASE_URL`, `E2E_DB_ADMIN_URL`.
API port 63051 was ephemeral. The host test exited successfully (1 passed,
241 library tests filtered; binary/integration selections executed zero tests).
The host's static capability collaborator and pre-C1 compliance response are
not media execution, packaged metadata, real startup or whole-E2E evidence.
Parent integration `4a67468f` contains C1 and requires a real metadata witness;
this fixture-only patch does not bypass that boundary or change architecture.

Final focused checks: `just fmt`, TypeScript `--noEmit`, policy guardrails,
`just instruction-drift`, and `git diff --check` passed. Canonical `just ci`
and `just ui-e2e` were not rerun in this bounded checkpoint and remain required
on integrated source. No full-gate, warning-free CI or completion claim is made.
Continuation evidence is retained under the same durable directory's
`profile-patch-proof/`, including command/source records, original logs, JSON
counts, both tested fixture revisions and cleanup verification.

## Parent Integration Checkpoint

The parent reviewed original commits `6fbf4ab0` and `f63c1849`, then replayed
their fixture-only changes onto C1/D3 integration `4a67468f`. Both authored
TypeScript files remain byte-identical to the retained final tested sources.
The replay does not certify the combined runtime: the focused API host used
pre-C1 source `62c5d6a7`, and full integrated CI/UI and canonical Sonar remain
required. The task stays In progress; no release readiness, GitHub push or
merge is implied.

The parent also corrected this record's attribution of the watcher-only
refinement: it was an implementation-review request, not an operator decision
or new approval. The existing architectural approval boundary is unchanged.

## Observability And Status Docs

### 2026-09-12 Bounded Schedule Boundary Review

Reviewed source `df2921fb55869626da65848b08d7eb0e25a3bd51` in the isolated
`work/media3-ui-schedule-proof` worktree. The requested narrow UI/API/E2E
correction is blocked by the existing contract, not another fixture value:

- `tests/specs/api/media.spec.ts:476-485` supplies scheduling enabled and a
  120-minute interval, then requires HTTP 200 and persisted enablement.
- `crates/revaer-api/src/http/handlers/media.rs:238-253` forwards both values;
  `crates/revaer-app/src/media.rs:483-497` forwards them to `update_profile`.
  `crates/revaer-data/src/media/profiles.rs:8,156-176` calls the frozen
  `media_profile_update_v1` procedure.
- Frozen `0182_media_normalized_configuration_identity.sql:1417-1425` rejects
  every true schedule/watcher flag or non-null interval before mutation. This
  predicate does not consult persisted root identity. Even an actually resolved
  and persisted device/inode cannot make that legacy PATCH accept the request.
- ADR 557's Legacy Cutover (`:1356-1380`) excludes compatibility tables, views,
  dual writes and fallback procedures; its follow-up (`:1585-1586`) preserves
  current runtime behavior until the coordinated cutover removes legacy paths.
  ADR 588's accepted discovery design still binds to ADR 557's immutable
  associations and attestations. This review does not reopen either approval.

No schedule/API/runtime/E2E patch was made. A pre-cutover compatibility route
around this rejection would change the accepted filesystem/API boundary, not
constitute a semantics-preserving internal refinement. The existing approved
root/profile/discovery integration and coordinated cutover remain the route
forward; they exceed this bounded UI/API/E2E ownership. No new architecture or
operator consent was invented. All positive automation assertions, fail-closed
negative tests, actual-identity requirements and dry-run defaults remain intact.

The original `/private/tmp/revaer-approved-ui-hash-fill.log` still records
57 passed, 1 failed and 72 not run. The UI project depends on the API-key
project, which depends on the failing anonymous API project; no UI coverage
was produced. Teardown correctly rejects the missing coverage. No coverage
exception, altered dependency or fabricated browser visit was introduced.

Focused command: `just --command ruby --disable-gems
target/ui-schedule-proof/source-boundary-check.rb`, **15/15 source checks**.
This is explicitly not a new runtime regression or UI pass. Original source
bytes/hashes, the parent-log hash and its failure excerpt are retained at
`/private/tmp/revaer-ui-schedule-proof/target/ui-schedule-proof/source-20260913T012152071639/`.
No new UI build, Rust build, database or server was started after identifying
the boundary; no test media, credentials, uploads or remote mutations were
created or read. The metadata author's worktree was left untouched. Parent
subsequently reran full CI/UI on `7ee7df07`: CI exited zero with eight watcher
WARNs; UI reproduced the same 57/1/72 result and missing coverage. The completed
worktree was removed after retaining its source review in
`artifacts/media-verification/2026-09-12-d3-metadata/ui-schedule-416023b5.tar.gz`.

Reviewed root AGENTS and scoped UI/Rust instructions before this record-only
change, plus the existing task and ADR 419/557/559/588 boundaries. No instruction,
workflow, index, database-proof, ASSET or Sonar criteria changes were made.
Rollback is removal of this record subsection; there is no executable delta.

No runtime metric, event, log, API schema or support-claim changes. Assertion
messages now expose the existing error body. Later automation expectations still
require their approved workflow integration; this correction is not a completed
discovery or media vertical slice. This continued record is registered in
`docs/tasks/index.md` and `docs/SUMMARY.md`; generated documentation remains with
the parent. No acceptance criterion or route-coverage threshold changed.

## Risk, Rollback And Dependencies

Test-only change, no new dependency. Roll back the scoped commit to restore the
previous fixture. Neither relaxing the identity guard nor activating a legacy
automation path is authorized or necessary for this bounded correction.

## Stale-Policy Check And Cleanup

Reviewed `AGENTS.md`, scoped Rust/data/UI/DevOps instructions, the task template,
the Just command surface, specification and cited ADRs. The stale create
fixture and metadata-only cadence drift are corrected. Included automation
remains a recorded failure, not a disabled assertion. The matching UI instruction
now states the discriminating fixture/negative-proof boundary. No policy or
architecture relaxation was made. All owned API hosts terminated and their
child databases were verified absent. Test media trees were removed by executed
hooks and checked in both temporary directories before the watcher refinement
and after it; the owned containers and volumes were removed. Continuation ports
62635 and 63051 both refused connections. Temporary host source/wiring and
generated authentication state were removed. Original evidence remains retained.
At worker handoff, the clean committed worktree was retained for parent
integration. Its commits contain no C1, bootstrap, runtime shutdown, D3, ledger,
approval-status or generated-index edits; the parent subsequently added only
the ledger/index updates and integration record described above.
