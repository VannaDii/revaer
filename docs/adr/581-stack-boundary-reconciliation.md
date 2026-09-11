# Stack boundary reconciliation

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - GitHub's displayed PR sizes and branch names do not prove a linear,
    reviewable stack. ADR 564 identified an unverified PR 99 transition.
- Decision:
  - Audit all current media PR boundaries and correct the isolated PR 99
    reconstruction. Preserve original refs and remote metadata pending full
    descendant reconciliation. Do not change review or quality criteria.
- Consequences:
  - The local PR 99 schema regression is corrected, but no remote repair,
    complete stack validation, source push or merge is claimed.
- Follow-up:
  - Propagate the existing dependency/security and instruction corrections at
    their earliest owning boundaries, repair the phase read-model descendant,
    reconstruct the approved init slices, and reconcile formal membership only
    after verifying the complete ordered branch mapping.

## Verified Boundaries

Read-only CLI/API snapshots on 2026-09-10 cover all 104 open PRs with
`stack/media3-` heads. Their base names form one unbranched chain from `main`;
all 104 titles are conventional. Exact commit ancestry holds at 103 boundaries,
with PR 98 the sole exception. GitHub has two formal stacks: stack 204 contains
41 PRs from 195 through 97, and stack 202 contains 63 PRs from 98 through 194.
No PR is missing formal membership. The local extension does not track the
primary checkout's current branch; that does not mean remote membership is
absent. No browser was used for monitoring or inspection.

The canonical `just stack-changed-lines` gate was run for all 104 current
base/head pairs. It rejects three boundaries:

| PR | Exact Base / Head | Required Repair |
| --- | --- | --- |
| 98 | `54157ef8` / `7ed723d5` | Non-ancestor base. GitHub's comparison shows 131,057 changed lines; the direct endpoint no-rename comparison is 14,940. Neither is a permitted review boundary. |
| 130 | `6471074e` / `a3797174` | 213 binary deletions, 21,064,113 original bytes. The guard fails closed on uncountable entries despite GitHub showing 2,481 text changes. No binary exception is approved or implemented here. |
| 186 | `6eb56bc2` / `adb64ca5` | 97,202 no-rename changed lines, although GitHub reports 9,906. Moving the init path contributes 43,694 deleted plus 43,752 added lines. Reconstruct using the approved stable-path assembly/cutover contract; do not exploit rename detection. |

Other size gates pass at the inspected SHAs. This is not a claim that their
content, approvals, checks or reviews are complete. The private audit retains
full API snapshots, ordered edges, each guard's real status/output, and an
immutable old-blob/byte inventory for the binary deletions.

## PR 99 Correction

The isolated reconstruction at `ccf8b3ef` mistakenly restored the expected
`MediaJobPhaseAppendRequest` schema after repaired base `f87bea0b` removed it.
The focused schema test reproduced the failure. Corrected commit `377a082e`
removes the stale expectation and asserts absence without changing any runtime
schema or route. Its diff against `f87bea0b` is 1,368 changed lines.

Independent review found no other semantic mismatch: 14 of 17 changed files
have identical changed lines to the original PR; the remaining differences are
the corrected schema expectation, necessary import context and documentation
ordering. Descendant `28f72711` reintroduces the append schema while exposing
read-only phases; reconcile that intermediate boundary rather than remove the
new regression assertion. A later removal in `1a0b63e7` is not evidence that
every intermediate PR passes.

## Phase Boundary Replay Checkpoint

The corrected PR 99 boundary now has 20 reconstructed descendants through
`ef84c954` on retained local ref `work/media3-phase-boundary-20260910`.
The first nine patches replay byte-for-byte. At PR 110, `f99fec1b` retains the
single schema catalogue and the absence assertion, and moves the already
existing phase-schema correction from `1a0b63e7` to its first read-only owner.
Both generated API artifacts were regenerated and compared structurally.
An independent review confirmed exact preservation of PR 110's other 28 files
before this deliberate regeneration and task-record update.

All ten remaining descendants replayed. The readiness artifact conflict
preserves both schema additions; the later worker-owned-phase artifacts match
their original commit exactly. Comparing reconstructed tip `ef84c954` against
original `eb31c3f5` yields only the absence assertion and ADR 467/475 updates.
All 20 canonical changed-line guards pass, as does range whitespace validation.

At PR 110, all eight OpenAPI tests pass under both minimal and all features;
strict API Clippy, workspace formatting, API export and artifact checks pass.
Both eight-test OpenAPI configurations also pass at the reconstructed tip.
Full `just ci` at that tip reaches audit and fails on `h2` 0.4.12
(`RUSTSEC-2026-0258`) and yanked `chacha20` 0.10.0. Full `just ui-e2e` stops
before browsers at four high-severity npm dependency findings. Initial sandbox
Docker/network failures were retried with explicit isolated database ownership
and the required network access. No passing full-gate claim is made.

The existing dependency correction `1ee5f2ba` still needs propagation at its
earliest owning stack boundary. Original refs remain intact; no remote branch,
formal stack, review state or required check was changed. D4, D5 and S2 remain
held. The private phase-boundary evidence directory retains complete gate logs,
range comparison and final tree delta; temporary media is cleaned.

## Dependency And UI Checkpoint

The unchanged dependency inputs at `ef84c954` match `1ee5f2ba`'s parent.
Applying that existing correction yields `cf9d0456`: Rust audit and deny,
tests npm audit/API-client generation, and release npm audit all pass. The
lockfiles and test manifest exactly match the original correction's output.
Only branch-specific task indexes/catalogues were reconciled. The resulting
355-line boundary passes the canonical changed-line guard.

Full UI validation exposed the existing undefined reporter `envDir` regression.
Existing correction `654ed33b` was replayed as `d90b3c80`, with identical
Playwright configuration and a 98-line boundary. A later full run reproduced
the stale expectation that worker-owned POST routes should not return 405.
Existing correction `91ea0f9d` was replayed as `bef72ac9`, preserving the exact
test file and separately requiring 405 for those retired writes. Its boundary
is 115 lines; supported-route assertions and coverage remain intact.

Full `just ci` exits zero: all-feature/minimal tests, strict lint, dependency
gates, all 18 package coverage gates, script coverage and release build pass.
The run began at `cf9d0456`; only the two existing TypeScript test/configuration
corrections and their documentation were added while it ran. No Rust, build,
workflow, scanner or threshold input changed during validation. Rust LCOV has
218 source records, 99,028 line records and 92,060 covered lines. Complete
native, Rust and script coverage reports are retained privately. There are
eight held-S2 watcher-abort warnings among 40 WARN records, so exit zero does
not establish warning-free acceptance or published Sonar coverage.

The final full UI run at `bef72ac9` has 46 passes, one failure and 61 not run.
Media profile creation returns 400 instead of 201 at
`tests/specs/api/media.spec.ts:130`; coverage teardown remains incomplete.
Earlier runs retain distinct evidence for the undefined reporter, occupied
UI port 8080, an agent configuration error using unsupported API
port 17070, and the retired-route failure. The successful test-starting setup
uses UI port 18080 and the historical harness's API port 7070. The unrelated
service on port 8080 was untouched. No assertion, scheduling feature or
coverage requirement was disabled to hide the remaining profile failure.

The first-PR ownership audit identifies PR 195 as the earliest common owner,
with later dependency overwrites in PRs 197, 203, 73 and 90 that must be
reconciled. The initial bounded backport fits at 9,997 total changed lines,
but its older release graph still has 11 findings including one critical;
that was not an acceptable first-PR baseline. Compatible remediation in
`6c229aa5` now clears every Rust, tests and release advisory, including an
installed-release-graph audit and plugin-import smoke check. It retains the
real release plugin, parent versions, manifest and configuration: published
npm 11.19.1 clears its bundled findings, with compatible Undici 6.28.0/7.29.0
updates for the two unbundled clients. No alias, stub, plugin replacement,
force update or exception was used. The existing semver-diff 5.0.0 deprecation
warning remains; no newer release satisfies the current parent's ^5.0.0
constraint. This is not warning-free installation or complete validation.

The complete backport initially raised PR 195 to 10,184 changed lines. The
previously requested task-record consolidation is now committed as
`15e47d6e`: routine records 404, 407, 462, 479, 480 and 481 point to their
combined motivation, decisions, historical test evidence, risks and policy
checks in ADR 478. Original paths and historical Git records remain available;
architectural ADRs 461/477 and all non-documentation files are byte-unchanged
from the dependency commit. Their accepted decisions are not reclassified or
weakened. The canonical no-rename guard passes at 9,392 additions plus 585
deletions, or 9,977/9,999 changed lines. Link validation reports 683 OK, zero
errors and one unsupported pre-existing PostgreSQL example URI.

Full CI/UI at this exact first-PR revision and propagation through subsequent
overwrites remain required; reconstructed-tip results are not substituted for
that evidence. No remote source or stack mutation, merge, new architectural
approval or criteria relaxation is claimed. Completed validation worktrees,
owned database/container storage and generated test media are removed; refs
and private evidence remain. The primary checkout's changes are preserved.

## Exact Foundation Validation And Immediate Replay

At first-PR candidate `15e47d6e`, both canonical gates were attempted in an
isolated worktree using NVM Node 24.19.0 and a dedicated PostgreSQL container
on port 5455. Neither passed. These results do not replace or invalidate the
separately recorded reconstructed-tip results above.

- `just ci` stops at native compilation during lint: pkg-config selects the
  installed x86_64 libtorrent 2.0.11 while Rust targets arm64, and its Boost
  headers are unavailable. The installed arm64 libtorrent is 2.1.1, outside
  this historical boundary's supported range. Existing later commits
  `294adb46` and `8ab54c8d` implement 2.1 compatibility and ABI coherence;
  they were not copied into the foundation or bypassed with native skips.
  Exact-boundary validation needs a compatible native environment. A bounded
  check found no already-equipped local Linux validation image.
- `just ui-e2e` installs the exact tests graph, passes its zero-finding audit,
  generates the API client and reaches setup. The historical harness requires
  port 8080, already owned by an unrelated application, so setup refuses to
  proceed. Teardown correctly fails for absent API coverage. No tests ran;
  the unrelated application was not stopped, and no port/coverage assertion
  or quality criterion was relaxed.
- The immediate two descendants replay cleanly apart from one manifest
  conflict: `75cb6635` becomes `b7e29ef9`, preserving the incoming exact
  brace-expansion scopes alongside patched fast-uri 3.1.6/js-yaml 4.3.2;
  `ebddb420` becomes patch-identical `79f23836`. All other differences from
  the original tip are inherited foundation corrections. Original refs,
  accepted ADRs 461/477 and the consolidated task-record pointers remain.
- Both descendant npm lock/manifest dry-runs and zero-finding audits pass,
  as do instruction drift, whitespace and canonical changed-line guards:
  627/9,999 for the two-commit replay and 657/9,999 for the inherited delta.
  These are not full CI, installed-graph or lifecycle-validation claims.

Private evidence is retained in the dated foundation-validation and
foundation-descendants directories under `/private/tmp`. No remote source,
PR, formal-stack, release, criteria or architectural-approval mutation was
performed. D4, D5 and S2 remain held. The first foundation still needs its
own passing gates before publication; later-tip success is not a substitute.

## Linux Foundation And Eight-Commit Replay

Exact first-PR candidate `15e47d6e` now has Linux arm64 evidence from an
isolated Ubuntu 24.04 environment with Rust 1.91.0, NVM Node 24.19.0 and
libtorrent 2.0.10. The source tree remains unchanged. The first attempt lacked
jq in the tool image; installing that prerequisite and rerunning the unchanged
gate resolved the infrastructure failure without a repository or criteria edit.

- Full `just ci` exits zero, including all-feature/minimal tests, strict lint,
  audits, all 16 package coverage gates and the release build. Its 40 runtime
  shutdown WARN records remain defects, not warning-free acceptance.
- Explicit `just test-native` exits zero: 90 unit tests and three native
  integration tests pass with `REVAER_NATIVE_IT` enabled. A mounted real Docker
  socket and successful `docker info` preflight prevent environment-based
  native-test skipping. No native implementation or ABI criterion was changed.
- Full `just ui-e2e` exits zero: all 101 tests pass, including API and UI
  coverage teardown. Container-local ports avoid the unrelated host service
  on 8080. No assertions, browser projects or coverage checks were disabled.
- Local LCOV retains 168 source records, 57,985 executable lines and 54,144
  covered lines. This is neither published Sonar coverage nor validation of
  the complete later media feature, Linux amd64, or either release package.

Eight more existing descendants replay from `79f23836` through `27130e90`.
The replay preserves the patched real semantic-release npm plugin and its
installed dependency graph instead of importing the incoming no-op replacement.
Root-over-scoped instruction precedence, the security fixes and the consolidated
task-record pointers remain intact. Incoming API-generator graph changes are
retained with patched js-yaml; no new production behavior is invented.

Both installed npm graphs, zero-finding audits, workflow regression/live
guardrails, instruction drift and whitespace checks pass. Seven individual
changed-line gates pass; the asset-conversion boundary still rejects its 213
binary deletions, exactly as original PR 130 does. This is the existing PR 130
blocker, not an additional distinct PR. Full compiler/UI gates were not run
for these eight descendants. Known intermediate asset URL defects still need
their existing later correction moved to the first responsible boundary.

ADR 585 inventories that fixed binary deletion set and proposes an explicit,
one-deliverable exception; it remains Proposed and changes no executable rule.
D4/D5 approval was requested but not received, and S2 remains held. The frozen
database policy prevents persisting the new root catalog before its approved
single-init cutover; disabling media automation assertions is not a repair.

Complete nonmedia logs and coverage are retained in the dated Linux-foundation,
foundation-descendants and binary-deletion evidence directories under
`/private/tmp`. No remote source, formal-stack, review, required-check or approval
state was changed. The primary checkout's staged changes and conflicts remain
untouched. These are exact-revision checkpoints, not completion of the goal.
All Linux validation processes have terminated; their run containers, dedicated
database and completed validation/replay worktrees are removed. Canonical media
fixture cleanup passes. Committed refs, the reusable tool-only image and nonmedia
evidence remain. The documentation checkpoint passes link, drift and whitespace
checks; no full-gate result is attributed to this documentation-only revision.

## Corrected Asset Owner And Next Stack Segment

The local asset deliverable now spans `945e81d6..0d725dce`. Commit `3892b126`
consolidates the existing conversion and hardening commits; `0d725dce` adds
three regression tests and their task note. Runtime UI bytes match the existing
hardening output, while the root logo and equality gate bring forward the exact
approved `1462d18e` correction. No new runtime architecture, dependency, feature
removal or scanner criterion is introduced.

The unchanged canonical guard still rejects the exact 213 binary deletions.
Their prior blobs, SHA-256 values and byte lengths match ADR 585's inventory;
the full text subtotal is 3,191 changed lines. Neither historical nor corrected
representation has an approved binary exception. ADR 585's original manifest
is not silently extended to cover these new heads.

Trunk release-output HTTP verification passes for 22 SVG URLs, exact source
bytes/MIME types, icon/manifest/tile references, DataTables paths and root-logo
equality. Full UI at `3892b126` passes 101 tests, including route-coverage
teardown. Desktop dashboard inspection confirms rendering of the logo and queue
images, not comprehensive visual, mobile or platform-icon acceptance.

Initial CI found a dedicated database startup race, then an E0463 dependency
artifact failure during coverage. The database became ready and a clean
coverage rebuild completed all tests without that compiler error. It exposed
asset-sync coverage below 90%; the new tests assert nine malformed SVG cases,
both valid namespace quote styles and legacy/incomplete DataTables URL sets.
All 36 library tests, the binary test and formatting pass. Full CI at
`0d725dce` exits zero, including all 16 package coverage gates and release build.
LCOV has 169 source records, 58,808 executable lines and 54,904 covered lines;
the asset-sync library covers 951/1,030 lines. The 40 runtime shutdown WARN
records remain; this is not warning-free or published-Sonar acceptance.
Final exact-revision `just test-native` and `just ui-e2e` both exit zero:
94 native-library unit tests, seven build-contract tests, three enabled native
integration tests and all 101 end-to-end tests pass. Their nonmedia logs and
route-coverage records are retained separately from the earlier revision.

Twelve following commits are reconstructed through `cd39407f`: the two prior
quality/database descendants and ten originals through managed-workspace
lifecycle. The incoming Sonar refactor now preserves the already-established
run-derived database bindings and one 1 GiB allocation per service. Original
release-JavaScript coverage steps are restored against the retained real release
configuration recipe; only rejected stub-specific instrumentation remains absent.
The recipe produces positive LCOV for 71/71 lines (63 real configuration lines,
eight smoke-script lines). This is local coverage, not a published Sonar result.

All twelve individual size gates pass (largest 6,980/9,999), as do workflow
regression/live policy, instruction drift, shell syntax and whitespace checks.
The final rebase preserves every patch and adds only the parent's 64 test lines
and four ADR lines. Installed npm graph/audit evidence remains applicable to
byte-identical dependency inputs. Full compiler/UI gates for the descendants
are not claimed. Original and intermediate refs and exact maps are retained.

The completed agent, intermediate replay and asset validation worktrees are
removed; all validation processes have terminated. Dedicated database/run
containers, ignored database storage and test media are removed; canonical
fixture cleanup passes. Private nonmedia evidence is under the dated asset-owner and
next-stack directories in `/private/tmp`. D4/D5/S2 remain held, the frozen
database authority is unchanged, and no GitHub source, review, stack or check
state has been mutated. The primary checkout remains untouched.

## Managed-Workspace Validation And Recipe Ownership

The managed-workspace test delta from original `ba74b3b2` is preserved at
`00a1bac5`. Full policy exposed a retained exact-tool test reading the root
Justfile after recipe modularization. Correction `b5ab8eec` reads the effective
`just --show udeps` recipe, preserving all version, toolchain, invocation and
rejection assertions. Its three-file delta is 23/9,999 lines; the DevOps clause
and ADR 478 record accompany it. No criteria or production behavior changes.

The correction is folded into its first owner, `e830333b`, and all 19 following
commits are replayed through `011da317`. All rewritten per-commit size, drift,
whitespace and changed-shell checks pass, together with workflow regression,
live policy/workflow/stack guards and the exact-tool tests. The largest remains
9,945/9,999. All twelve runtime patches remain exact. The rewritten workspace
boundary `63d1e258` and tested `b5ab8eec` share tree
`0813c6dac26a69cd34268795e6a5b32231a92ddd`; history-sensitive checks were run
separately, not inferred from tree equality. Original refs remain preserved.

Full isolated Linux arm64 `just ci` at `b5ab8eec` exits zero, including all
18 package coverage gates and the release build. Rust LCOV contains 186 source
records, 63,976 line records and 59,905 covered lines; generic script coverage
is retained. The 40 held shutdown WARN records remain. Explicit native tests
pass: 94 unit, seven build-contract and three enabled native integration tests.
Full `just ui-e2e` passes all 101 tests and its JavaScript coverage assertions.
The initial browser installation reported missing optional-browser libraries;
the isolated tool image is corrected without changing source or test criteria.
The final full rerun again passes all 101 tests with no missing-library warning.
JavaScript LCOV contains 54 sources, 4,382 lines and 3,866 covered lines. These
are the configured Chromium and two API projects, not an all-browser claim.

PR 195 remains assigned to VannaDii, with 30 successful checks on its old
published head `37062266`, not this local reconstruction. A Copilot request
returns CLI success but subsequent API read-back has no pending or submitted
review; fulfillment remains unconfirmed. This request is the only remote
mutation in this checkpoint. No source, stack or required-check state changes.
Published Sonar coverage, package acceptance, D4/D5/S2 and ADR 585 remain open.

Private nonmedia evidence is retained in the dated workspace-owner and
runtime-stack evidence directories under `/private/tmp`. Root and scoped
DevOps instructions were reviewed; the stale root-only recipe assertion was
corrected without relaxing it. No new architectural decision or approval is
recorded. The completed replay agent and its clean worktree are removed.
All validation processes have terminated; the dedicated database and its
storage are removed, and canonical fixture cleanup passes. Nonmedia coverage
and route evidence are retained before removing ignored test outputs. Document
indexing, instruction drift and whitespace checks pass; link validation reports
1,139 OK and zero errors. The book builds with its existing large-search-index
WARN (11,961,608 bytes), not warning-free documentation acceptance.

## Execution And API Boundary Validation

Full isolated Linux arm64 `just ci` at `011da317` exits zero, including all
18 package coverage gates and the release build. The added execution,
replacement, verification, persistence, API and UI sources are included in this
boundary; this does not imply that the later application job-runtime wiring
is present. Rust LCOV has 207 source records, 81,763 line records and 75,986
covered lines. Generic script coverage is retained. Forty held shutdown WARN
records remain, so this is not warning-free acceptance or published Sonar proof.
Explicit native tests pass: 94 unit tests, seven build-contract tests and three
enabled native integration tests, with none ignored or filtered.

Full E2E is not passing at this boundary. All 101 individual tests pass, but
teardown rejects 39 uncovered media API operations. The original recipe returns
zero anyway because later LCOV assertions mask the Playwright failure. This is
a false-green gate, not acceptance. The route list and full output are retained;
no route or assertion is removed. The application does not yet inject the real
media provider at this boundary, so moving later success-only route smoke tests
backward without reconciling the service boundary is not a proven fix.
Application startup also regenerates both tracked OpenAPI artifacts; their
exact output and drift are retained, then only these owned generated changes
are restored. The later schema commits are not silently assumed present.

Two earlier startup attempts failed from Docker storage exhaustion and a
nested-cache ownership error. Host-backed Cargo registry and complete home
cache mounts resolve those environment failures without pruning unrelated
Docker resources. Their failed logs remain separate from the completed test
run. The earlier managed-workspace E2E log was checked for the newly discovered
teardown failure markers and has none; its result is not retroactively treated
as this boundary's result.

Correction `cee9c667`, validated locally as `13da9c59`, makes every existing
LCOV assertion blocking and propagates the captured c8/Playwright status after
checking coverage. The 28-case regression executes the resolved recipe with
isolated command fixtures: two shard modes, success/failure test statuses and
seven valid/invalid coverage shapes. Its original-recipe negative control
reproduces the false success. Workflow regression, instruction drift, shell
syntax and whitespace checks pass; the four-file correction is 89 changed
lines. A real complete E2E rerun at `13da9c59` still has 101 passing tests and
39 uncovered operations, but now correctly exits one. It is not a passing E2E
gate. No test, route requirement, coverage assertion or threshold was weakened.

The next eleven originals are reconstructed through `232a81ea`, retaining
their exact code and historical migration deltas. Two missing per-diff E2E
instruction updates caused real drift failures. The owning commits now include
one maintenance instruction each; ADR 430 has two corresponding record lines.
These four additive documentation lines are isolated in preservation evidence;
they do not change executable behavior or create operator approval. All eleven
size gates pass (largest 9,938/9,999), together with instruction drift,
whitespace, workflow regression/live checks and patch preservation. There are
no changed shell files, so that syntax-check input set is empty. Full compiler
and E2E gates for these later eleven commits are not inferred from the earlier
boundary's results. Original and pre-amend refs remain preserved.

The next quality commit, original `f5f22fa1`, requires separate reconciliation:
its rejected release-plugin alias, dependency downgrades and loss of working
Playwright result paths must not overwrite the repaired baseline. No new
duplicate tolerance, architectural approval or criteria relaxation is inferred
from the original commit. D4/D5/S2, ADR 585, package acceptance and published
Sonar remain open. No remote mutation was performed in this checkpoint.

The corrected quality candidate `0fc991d5` preserves the real release graph and
working Playwright configuration, retains inherited patched RNG/WASI versions,
and removes only two unused duplicate tolerances. The original downgrade
candidate's four duplicate-family failures are resolved without new exemptions.
Full locked metadata, warnings-denied Cargo audit, canonical deny, workflow and
preservation checks pass; its diff is 1,768/9,999. No full compiler or E2E result
is inferred for this later candidate. Exact deviations and retained package
records are recorded in the private quality report.

The E2E correction is folded into its first owner `b0e8def9`, and all 31
descendants are replayed through `d540f67c`. All 32 mapped size, drift,
whitespace and changed-shell checks pass (largest 9,945/9,999), as do the
28-case exit regression and workflow/live guards. Thirty descendant deltas
remain exact; the original job-runtime commit's identical `set -euo pipefail`
line is already present from the earlier fix and is retained once. This is
accepted as a mechanical overlap, not a new architectural decision or literal
patch-identity claim. Independent application of the correction to `0fc991d5`
produces the exact final tree. Cargo/npm graphs and deny criteria are unchanged.
The mapped API-service boundary `d26bc15b` has the exact tree of tested
`13da9c59`; it retains the known E2E failure, not a pass.

All completed agents and owned validation/reconstruction worktrees are removed,
their refs and nonmedia evidence retained. Dedicated database/run containers
and database storage are removed; canonical fixture cleanup passes. The user's
primary checkout remains unchanged. Host-backed dependency caches are retained
outside worktrees for future validation; no test media is retained. Private
evidence lives in the dated execution-owner, job-stack and E2E-exit directories
under `/private/tmp`. The next required work is the failing API/provider/schema
boundary and full validation of the later application-wiring/quality segment,
not a source push based on the earlier false success.
Documentation indexing, instruction drift and whitespace checks pass; link
validation reports 1,139 OK and zero errors. The book builds with the existing
large-search-index WARN, not warning-free documentation acceptance.

## Task Record

- Motivation:
  - Make stack reconstruction evidence-based and preserve each reviewable
    deliverable without forcing an oversized integration history onto GitHub.
- Design notes:
  - Keep original and corrected local refs separate. All remote inspection is
    read-only and exact-SHA based; no stack linking, bypass, force-push or
    required-check mutation occurred. The API assertion preserves the earlier
    API boundary, not a new architecture choice.
- Test coverage summary:
  - Eight OpenAPI tests pass under minimal and all features. Strict API
    all-target Clippy and workspace formatting pass. Full `just ci` reaches
    audit and fails on vulnerable `h2` 0.4.12 and yanked `chacha20` 0.10.0.
    Full `just ui-e2e` stops at the old npm graph's security audit before
    browsers start. These gates remain failed; no criterion was relaxed.
  - Initial local system-Ruby and Node selection failures were corrected using
    the available modern Ruby and NVM-managed exact Node, then both gates were
    rerun. The independent existing-data proof is recorded in ADR 582.
- Observability updates:
  - Retain private guard, GitHub, compiler, test and audit output. No production
    logging, telemetry or scanner settings change.
- Status-doc validation:
  - Update ADR indexes, the generated catalogue and the completion ledger.
    Keep implementation evidence separate from remote and package readiness.
- Risk & rollback plan:
  - A local reconstruction could erase descendant changes if published without
    reconciliation. Keep it unpublished, retain original refs, and validate
    exact repaired boundaries before any conditional push. Discard only the
    owned reconstruction ref to roll back this preparation.
- Dependency rationale:
  - No dependency added. The audit uses existing Git, GitHub CLI, Ruby standard
    libraries and the canonical changed-line gate.
- Stale-policy check:
  - Reviewed root AGENTS, Rust/data/devops scoped instructions, ADRs 522/559/564,
    the task template and the historical PR 99 instructions. Historical scoped
    precedence drift is already corrected in the integration and remains a
    restack propagation requirement. No consent or clean-gate claim is inferred
    from the historical task records.

## Integrated Validation And Cleanup

- At `f0a17970`, full `just ci` exits zero in the retained integration worktree.
  All-feature/minimal tests, strict lint, dependency audits, package coverage,
  script coverage and release build complete. Eight existing held-S2 watcher
  WARN events remain; this is not warning-free acceptance.
- Full `just ui-e2e` exits one: 46 pass, one fails, 61 do not run. The unchanged
  media profile creation assertion receives 400 instead of 201 at
  `tests/specs/api/media.spec.ts:130`; teardown also rejects missing job-phase
  and profile-readiness route coverage. No assertion or feature was removed.
- Fresh CI Rust LCOV contains 238 source records, 101,422 line records and
  94,332 covered lines. CI-only generic coverage retains 116/163 executable
  existing-data-helper lines and 123/206 ingestion-proof lines. ADR 582's
  separate live-run coverage is retained as separate evidence, not silently
  merged with these counts. No authoritative Sonar scan or published metrics
  were obtained; the previously recorded service/credential limitations remain.
- The first temporary-worktree integration attempt encountered an asset helper
  cached with a removed worktree path. Only workspace-package build artifacts
  were cleared; the authoritative successful run rebuilt them at the stable
  integration path. This did not change source, tool pins or criteria.
- Documentation links pass: 1,126 OK, zero errors. The book builds with the
  existing 11,722,153-byte search-index WARN. The closeout-only record is
  reindexed and checked after this checkpoint.
- Both completed worker/reconstruction worktrees are removed, their committed
  refs retained, and evidence copied to the private stack-audit evidence
  directory. Both dedicated validation databases and their bind-mounted
  storage were removed. Canonical fixture cleanup passes, and no application
  or UI development server remains. The primary checkout's staged changes and
  conflicts are unchanged. No remote source or formal-stack mutation occurred.
