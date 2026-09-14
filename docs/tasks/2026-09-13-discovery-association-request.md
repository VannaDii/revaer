# Logical Discovery Association Request

- Status: In progress
- Date: 2026-09-13
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Shared association create/replace request and focused
  wire tests; integration owner, local and unpublished.
- Approved authority: [ADR 557](../adr/557-root-persistence-contract.md#profiles-and-discovery-associations),
  [ADR 559 G1](../adr/559-media-approval-delta.md#g1-architecture-versus-implementation-detail),
  and [ADR 588](../adr/588-first-release-decision-package.md) retain their exact scopes.
- Requirement ledger: L2/L4/L9 in the
  [completion ledger](../adr/564-media-completion-ledger.md#outside-in-ledger).

## Motivation And Design Notes

The full UI gate still fails at schedule activation because the legacy
path-taking profile cannot establish trusted filesystem identity. The approved
replacement uses a separate logical discovery association, not a relaxed root
check or a different expected HTTP status.

Add the exact eight-field create/replace request in the shared API models.
Construction and map-only JSON decoding require both logical keys, an explicit
relative prefix, the profile UUID and positive PostgreSQL integer version, and
all three boolean modes. Empty prefix explicitly selects the whole attested
root; absent/null prefix is rejected. Unknown fields, duplicate fields,
positional arrays, absolute roots, invalid types and invalid bounds fail with
one value-free error. Debug output omits caller values. Immutable requests
serialize exactly the approved fields without defaults or normalization.

The existing map-only decoder is shared with readiness responses; their wire
shape and checks are unchanged. Requested automatic modes remain intent, not
admission. This does not add routes, conditional-header handling, stored
procedures, root attestation, UI controls or scheduling authority. The 1 MiB
decompressed HTTP bound belongs to the route, not this standalone body decoder.
The coordinated init/root cutover and complete operator workflow remain open.

## Test Coverage Summary

At base `2865b1b9` plus this source delta, `just test-media-root-contract`
passes all 72 tests: 11 association tests and 61 existing input/readiness tests,
zero failures/ignored and 24 unrelated tests filtered by the unchanged recipe.
Vectors exercise all required and duplicate fields, all eight mode combinations,
key/prefix/version boundaries, strict types, explicit whole-root selection,
constructor parity, redacted diagnostics and exact JSON round trips.

The initial focused log is `target/discovery-association-contract-tests.log`.
The first `just lint` run failed on a test's manual empty-string construction;
it is corrected to `String::new()` without changing lint criteria or assertions.
The focused rerun again passes 72 tests, including an additional UUID-array
rejection vector; strict all-target/all-feature model Clippy passes. Formatting,
instruction drift and documentation indexing pass. The private disposable gate
runner now checks final TCP readiness instead of the temporary initialization
socket, with the same 60 attempts and database settings; no child-PGDATA or
unproven ownership workaround is adopted.
### Source-Pinned Full Verification

The implementation is committed at `fa43f2612ffb2387134e6be3f366ad9fc542b77b`.
The host is macOS arm64; the disposable database uses the pinned PostgreSQL
16.14 image recorded with each gate. This is not Linux service-package proof.
The canonical no-rename size check from `2865b1b9` is 679 additions and 36
deletions, 715/9,999. That is a local pair, not a check of every GitHub PR.

- `just ci` exits 0, including all 18 package coverage gates and the complete
  release build. New association production code has 75/75 covered LCOV lines;
  its tests have 253/254. The gate log contains no compiler-warning or runtime
  WARN line. The disposable PostgreSQL server still emits its locale and
  local-auth initialization warnings, which remain unresolved and retained.
- The first UI attempt fails before tests because overlapping UI and release
  coverage preparation both reinstall `tests/node_modules`, leaving TypeScript
  libraries unavailable. This is an integration-owner coordination failure,
  not a product test result. It is retained; subsequent gates run sequentially.
- The sequential `just ui-e2e` rerun reaches the full configured suite: 57 pass,
  one fails with `media_profile_filesystem_identity_required` at scheduling
  activation, and 72 dependent tests do not run. UI coverage remains missing.
  No assertion, dependency, suite, route coverage or readiness gate is removed.
- `just sonar-compile-db` succeeds. JavaScript release coverage initially fails
  at npm audit because sandboxed DNS access is unavailable; the unchanged gate
  succeeds with network access. After UI completes, the final
  `just js-coverage-merge sonar-verify-inputs` succeeds on settled inputs.
- `just --command sonar verify --file
  crates/revaer-api-models/src/media_root_contract/association.rs --project
  VannaDii_Revaer` fails with organization-entitlement HTTP 403; it is not a
  Rust-analysis pass. The transfer was authorized under the operator-approved
  ADR 588 scope after verifying the reviewed permission text. No scope or
  account setting changes. `SONAR_TOKEN` remains absent, so no canonical scan
  or positive published coverage is claimed.
- Sonar's separate `analyze secrets` command completes successfully on all 12
  files in the implementation diff, with no findings reported, using installed
  engine `2.41.0.10709`. This does not replace Rust analysis or the full scanner.

The private `artifacts/media-verification/2026-09-13-association-and-cancellation/`
batch retains failed and passing runs, server/runner diagnostics, raw and
rendered coverage, Sonar-input evidence, the independent native cancellation
experiment and the bootstrap investigation. The incremental source bundle
requires `2865b1b9`, whose full source is retained in the preceding sealed batch.
No implementation is pushed, package qualified, PR accepted or feature merged
by this checkpoint. Full root binding and operator-workflow gates remain open.

## Observability And Status Docs

No live log, metric, health, HTTP, SSE or support claim changes. The completion
ledger identifies this as a request representation, not usable discovery.

## Risk, Rollback And Dependencies

Risk is disagreement between the accepted request and its decoder. Independent
wire vectors and existing readiness regressions cover the shared helper move.
Rollback removes the association model/tests and returns that unchanged helper
to readiness; no database or media state changes. No dependency is added.
ADR 589 remains pending and is unrelated to this request representation.

## Stale-Policy Check And Cleanup

Reviewed root, Rust, UI, DevOps and Sonar instructions and ADR 557's HTTP and coordinated-cutover
sections. Runtime compatibility and quality criteria are unchanged.
Root readiness is unchanged. New routine records follow the current task-record
policy rather than creating an architectural approval event. The conflicted
operator checkout remains untouched. DevOps instructions now require sequential
execution for gates sharing test dependency/coverage directories; no gate is
weakened. Focused model tests create no media. Every owned CI/UI container,
anonymous volume and host database directory is removed, both UI ports are
verified closed, and `just clean-test-fixtures` removes downloaded/generated
fixture media. Both completed investigation worktrees and the temporary
debugger image are removed after evidence retention.
