# Media Profile Create Rejection

- Status: In progress
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Bounded profile-create fixture and API regression tests;
  `work/media3-profile-create-proof`, unpublished, parent integration pending.
- Approved authority: [Specification](../../MEDIA_TRANSCODING.md),
  [ADR 419](../adr/419-pr76-media-data-review-remediation.md), and the existing
  [ADR 557](../adr/557-root-persistence-contract.md) /
  [ADR 559](../adr/559-media-approval-delta.md) cutover boundary. The operator's
  [ADR 588](../adr/588-first-release-decision-package.md) approval remains intact.
- Requirement ledger: [Media completion ledger](../adr/564-media-completion-ledger.md),
  configuration and discovery evidence; no ledger or approval-status edits here.

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
- Correct only the create fixture to disable scheduling and omit the interval.
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
  That request and all later discovery/readiness/job assertions remain intact.
- `just fmt`, focused policy guardrails, local schema generation and TypeScript
  `--noEmit` checks passed.
  `just api-test-client` failed at npm audit because sandbox DNS could not
  resolve the registry; no successful dependency audit is claimed.
- `just instruction-drift` failed because E2E edits require a scoped
  instruction update. Instructions and generated indexes are outside this
  delegated write scope; the parent must integrate those changes explicitly.
  `just ci` and `just ui-e2e` remain pending parent integration/revalidation.
  No full UI run, coverage waiver, upload, push, or release claim was made.
- Durable local evidence, including the host source, both configs, exact commands,
  JSON test counts and original failure logs:
  `/Users/vanna/Source/revaer/artifacts/media-verification/2026-09-11-profile-create-proof/`.

## Observability And Status Docs

No runtime metric, event, log, API schema or support-claim changes. Assertion
messages now expose the existing error body. The following PATCH and later
automation expectations still require their approved workflow integration;
this correction is not a completed discovery or media vertical slice. Parent
integration must update routine-task navigation through the existing Just
surface, without changing any acceptance criterion.

## Risk, Rollback And Dependencies

Test-only change, no new dependency. Roll back the scoped commit to restore the
previous fixture. Neither relaxing the identity guard nor activating a legacy
automation path is authorized or necessary for this bounded correction.

## Stale-Policy Check And Cleanup

Reviewed `AGENTS.md`, scoped Rust/data/UI instructions, the task template,
the Just command surface, specification and cited ADRs. The stale create
fixture is corrected; later profile-automation fixture drift is retained and
documented. No policy contradiction is removed by broadening scope. Both owned
API hosts terminated and their child databases were verified absent. Test
media trees were removed by the executed hooks; the owned container was removed.
The worktree is retained for parent integration, with no C1, bootstrap, runtime
shutdown, D3, ledger, approval-status, instruction or generated-index edits.
