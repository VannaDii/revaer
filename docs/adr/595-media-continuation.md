# Media continuation on the merged rv foundation

- Status: Recorded
- Operator approval: Nonarchitectural task record; explicit task-scoped unqualified handoff approval recorded below
- Implementation status: Committed continuation checkpoint; unqualified handoff approved for this task only
- Date: 2026-10-03
- Context:
  - The operator requested an isolated continuation from pushed checkpoint `71596802`, integrated with current main and the merged rv foundation.
  - Open media PRs and unfinished checkouts contain both retained requirements and implementations superseded by accepted ADRs 591, 593 and 594.
- Decision:
  - Create `work/media-continuation` in an isolated checkout, retain checkpoint first-parent history, and merge main `d413f0c6` without modifying existing checkouts or recovery references.
  - Use main's locked uv-managed rv executor. The Justfile contains direct compatibility aliases, including the explicitly required `just ci` and `just ui-e2e`; no second implementation of either gate exists.
  - Retain the checkpoint's immutable profiles, logical roots, source/replacement safeguards, normalized persistence, constrained runtime baseline verification and recovery authority. Old path-profile, migration/bootstrap and distributed takeover implementations are not replacements for these contracts.
  - Port independently useful fixture, SQL qualification, root selection, document-indexer isolation, native discovery and factory-reset retry changes with their tests. Reconcile Cargo workspace metadata offline without registry upgrades.
  - Preserve all other source/test heads as specifically accounted requirements until adaptation and qualification finish. This branch is an integration checkpoint, not evidence that every old requirement has been implemented or release-qualified.
- Consequences:
  - One committed continuation supplies a shared source base and an explicit disposition for every open media/validation PR and unfinished checkout.
  - Later container policy/technical-constraint work and discovery/recovery implementation still require adaptation to the current contracts. Legacy comparison material remains until its safeguards and tests have a validated replacement.
- Follow-up:
  - Adapt the four remaining service/portable-import fixtures, then port Python profile fixtures to complete immutable requests and logical root bindings. Preserve negative admission and source/replacement assertions; rerun both required gates.
  - Finish the required CI gates and the remaining unique behavior/test transplants recorded in the accounting. Do not delete recovery refs or abandon dirty checkouts before that work is complete.

## Task Record

- Motivation:
  - Replace competing unfinished integrations with one reviewable continuation while retaining operator recovery and distinguishing missing requirements from superseded implementations.
- Design notes:
  - Merge main rather than reset or restack the operator checkpoint. Use existing typed native collaborators and task dispatch. CI/E2E use the existing owned single-init lifecycle; existing databases are read-only baseline verified. Migration/reset/seed commands fail closed in feature-development mode.
  - PR workflow guardrails require the initialized application-fixture gate in single-init mode and reject historical migration execution. Historical transition fixtures still exercise their original phase. This implements ADR 591 rather than changing approved architecture or granting a gate exception.
  - Scanner scope expands to enumerate every retained tracked source and YAML fixture. No scanner criteria, coverage threshold, advisory ignore or remote quality setting is weakened.
- Test coverage summary:
  - Preserve initialized-runtime and cleanup regressions from the unfinished integration, missing-executable behavior, native discovery assertions, isolated document-indexer roots and transient-reset retry tests.
  - Add native Cargo baseline endpoint/failure verification and composition tests for both database phases; mutation tests reject missing single-init fixture gates and migration replay.
  - Reconcile media service and worker fixtures with explicit sealed runtime initialization. Assert factory-seeded cost snapshots rather than appending duplicate defaults. Preserve the missing-cost/source-preservation regression with a privileged corruption script restricted to the fixture-owned database; production snapshot procedures and triggers remain unchanged. These corrections require fresh gate evidence below.
  - Required gate attempts and final results are recorded below. Fixture passes do not establish full application or release acceptance.
- Observability updates:
  - Canonical baseline verification names success/failure without exposing endpoints or raw database errors. Retain existing bounded factory-reset retry diagnostics. No new product telemetry surface.
- Status-doc validation:
  - Reviewed root/scoped instructions, tooling inventory, workflow execution, container inputs and SPDX inventory against the selected sources. Added the current accounting and corrected the tooling inventory's stale integration pointer.
- Risk & rollback plan:
  - The integration is not qualified while gates fail or unique behavior remains unported. Return to the preserved pushed checkpoint `71596802` or main `d413f0c6`; the original checkouts and recovery refs remain intact. No production database, image publication, PR disposition or remote gate state was changed.
- Dependency rationale:
  - No new dependency. Retain main's Python/uv/package pins and exact SPDX package evidence, and the checkpoint's Rust dependency graph and native proof pins. Offline workspace lock reconciliation removes unused macro-error dependencies introduced by the divergent vendored manifests; no registry upgrade occurred.
- Stale-policy check:
  - Reviewed `AGENTS.md` and all seven `.github/instructions/*.md` files, Justfile/split recipes, PR/CI/Sonar/image workflows, tooling migration inventory and scanner properties.
  - Drift found: Just/shell ownership claims, historical database replay in feature-development, old integration baseline pointers, incomplete scanner source/YAML inventories, foundation-only tooling fixture assumptions and missing merged ADR index entries.
  - Updated live executor references, retained accepted checkpoint specialization and V0 authority, replaced migration gate execution with initialized application fixtures, expanded source inventories, and recorded required legacy parity/behavior work rather than declaring it complete.

## Reconciliation evidence

- [Human-readable accounting](support/595-continuation-accounting.md)
- [Exact PR heads, changed paths, checkout deltas and preserved references](support/595-continuation-inventory.json)
- Qualification remains incomplete. The source integration was committed as `b9fb1d1f`; CI regenerated the current API schema, retained as `e89b16a9`. The following evidence update changes documentation only.

## Gate evidence (2026-10-03)

- `just tooling-check`: passed, 1,749 Python tests, formatting, Ruff and strict typing.
- `just lint`: passed both workspace/all-feature and production-target Clippy checks. `just policy`: passed. `just instruction-drift --base 71596802 --head e89b16a9`: passed across the complete continuation.
- `just ci`: exit 101. Baseline verification, formatting, lint, Helm checks (6 annotation, 102 compliance and 47 package tests), instruction drift, asset checks, unused-dependency checks, audit, deny and UI build passed. The Rust application suite reported 354 passed and 37 failed. Thirty-one runtime failures report the duplicate `media_policy_operation_cost_media_policy_profile_id_sort_or_key` constraint; six media service tests return service errors requiring diagnosis. Later minimal-feature, coverage, tooling coverage and release-build steps were not reached.
- `just ui-e2e`: failed in the first anonymous API phase, 28 passed, 25 failed and four fixture errors. Profile/lifecycle fixtures still submit retired path fields and partial writes; other request/response assumptions also require reconciliation. Authenticated API and browser phases were not reached.
- Qualification ran on macOS arm64 against an explicitly owned disposable server using the pinned PostgreSQL image. Restricted runtime baseline verification succeeded. The owned databases/roles and server were removed after both runs; existing operator databases were not reset. Native root catalog reported the unsupported host platform, so these runs provide no positive Linux root-attestation evidence.
- CI generated `crates/revaer-app/docs/api/openapi.json` from the current typed contracts; the schema update is retained rather than leaving stale public documentation.
- No tests, advisory policy, coverage criteria or scanner settings were weakened. [Gate manifest](support/595-continuation-gates.json) records result summaries and local log checksums. Failed qualification is not a design-change decision or release approval.

- Subsequent E2E diagnosis found that the checkpoint's embedded base schema omitted current main's indexer routes and used stale foundation responses. Retain main's non-media path and schema inventory alongside the generated current media contracts; an API generator run cannot recreate omitted embedded foundation routes by itself. Required API validation remains enabled.

- Follow-up CI on the sealed-fixture correction (`a6898202`) passed formatting, both Clippy passes, the preceding validation steps and 356 application tests, but failed 35 application tests. Thirty-one now fail on denied access to the deliberately private legacy `media_discovery_job_enqueue_v3` writer; four service/portable-import tests still need adaptation. This is evidence that canonical costs are no longer the immediate setup failure, not evidence that the worker regressions ran successfully. Do not broaden runtime grants.
- Both the embedded `docs/api/openapi.json` and the crate's generated copy need main's foundation inventory. The first schema correction updated only the latter; the next E2E run therefore retained the same 25 failures/four fixture errors. Correct the embedded source and export through the existing canonical task before revalidation.

- After correcting the embedded inventory and running `just api-export`, `just ui-e2e` on `d5f00def` reported 41 passed, 12 failed and four fixture errors in the anonymous API phase. All 13 foundation API failures are resolved. Remaining failures use retired media path-profile/PATCH contracts or depend on those fixtures. Later authenticated/browser phases were not reached. The owned database/roles and server were removed. `just policy` and full checkpoint-to-branch instruction drift passed on this revision. Updated local log copies and checksums are in the gate manifest.
- At that revision, required qualification remained unproven. Native worker admission was subsequently reconciled below; remaining work is portable import/service contract adaptation, then complete immutable profile requests and preconditions in Python media scenarios. Preserve the source/change/cancellation/replacement assertions and keep private legacy writer grants closed.

- Native worker-fixture adaptation creates exact profile/association versions through runtime procedures and admits real descriptor-observed source fingerprints through the serializable guarded writer. Policy versions explicitly select dry-run or atomic replacement, and no-op tests use a source-matching desired target. Owned fixture SQL supplies synthetic catalog state only; it is not Linux attestation, persistent-storage or package evidence. Worker success cleanup is asserted; negative legacy automation fixtures use sealed init instead of replayed migrations. Required gates are pending on this correction.

- First native-fixture CI run (`db00179b`) reported 365 passed and 26 failed application tests. Legacy enqueue denials are gone and nine additional cases pass. Non-dry-run cases now reach the workspace boundary and reject its default directory permissions. Prepare the fixture workspace as owner-only, preserving the existing production safeguard. Four media service/portable-import cases remain unadapted.

## Approved checkpoint handoff

On 2026-10-03, after reviewing the committed continuation and accounting, the operator directly answered **“Approve unqualified checkpoint handoff”** to the explicit choice between continuing suite adaptation and a task-scoped exception to the `AGENTS.md` requirement that `just ci` and `just ui-e2e` pass before handoff. This approval permits completion of this continuation-checkpoint task only and expires at this handoff. It does not approve a release, establish product qualification, change architecture, relax either gate, or authorize any scanner, advisory, coverage, runtime-grant or remote-setting exception. Future implementation handoffs remain subject to both required gates.

The final source CI attempt on `21e89f1b` exited 101: 387 application tests passed and four failed. All 31 native worker cases passed, including source identity, cancellation, missing costs, replacement recovery, workspace permissions and media verification safeguards. Remaining failures are `media_discovery_creation_rejects_concurrent_target_append`, `media_discovery_run_queues_accepted_profile_paths_and_skips_rejected_sources`, `media_service_round_trips_profile_job_yaml_and_capability_paths`, and `portable_yaml_moves_complete_configuration_between_isolated_instances`. Later CI steps were not reached. The owned qualification server was removed.

The latest E2E evidence remains `d5f00def`: 41 passed, 12 failed and four fixture errors in the anonymous API phase; authenticated/browser phases were not reached. Subsequent changes affect Rust test fixtures and their owned test support; E2E was not rerun for this documentation-only handoff. The gate manifest identifies exact tested revisions and retained log checksums. Neither gate is represented as passing.

The continuation task is complete under this explicit exception: a committed, conflict-free branch integrates the pushed checkpoint and current main, with retained/superseded/required-unported work accounted. Product reconciliation remains unfinished. The next implementation action is to adapt the four named service/portable-import tests to current guarded association admission and logical-root import contracts, retaining concurrency and portable-import rejection assertions, and rerun `just ci`. Then update the Python media fixtures and run both phases of `just ui-e2e`. All recovery references and existing unfinished checkouts remain preserved until reconciliation finishes.
