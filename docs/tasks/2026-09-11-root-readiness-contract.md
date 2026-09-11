# Root Readiness Response Contract

- Status: Blocked
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Shared API-model readiness representation and independent
  contract tests; local unpublished continuation from `bb302c03`.
- Approved authority: [ADR 557](../adr/557-root-persistence-contract.md#root-administration)
  R1-R4 and [ADR 559](../adr/559-media-approval-delta.md)'s narrow G1. The
  [approval register](../media-approval-register.md) retains every pending hold.
- Requirement ledger: [L2 and L9](../adr/564-media-completion-ledger.md#outside-in-ledger).

## Motivation And Design Notes

The accepted outside-in sequence starts with the HTTP contract. The existing
root input helpers establish grammar but cannot represent the path-free
readiness result. Add that result under `media_root_contract` without installing
a route, changing a provider, or replacing legacy database/root authority.

Construction and deserialization enforce format version 1, the exact closed
source and attestation states/reasons, and a positive decimal generation bounded
to PostgreSQL `bigint`. A missing source is distinct from a ready zero-slot
catalog. Each result contains exactly five ordered kinds, with
`destructive <= binding <= attested <= 256`. Unavailable generations have zero
counts. Optional fields serialize as absent, never as invented empty values.
Unknown top-level and per-kind fields are rejected; no paths, logical keys, slot
ids, filesystem identities, owner values or digests exist in the response.

The public model is immutable after validation. Deserialization errors translate
to one bounded diagnostic without caller data. This validates reported values,
not the truth of filesystem probes or the consistency of future SQL reads.
Authentication, real routes, root administration, stored procedures, final init,
UI and packaged evidence remain separate work. No accepted or held runtime
behavior is changed.

## Test Coverage Summary

Independent JSON vectors cover exact state mappings, wire shape, omission,
generation precision, count bounds and ordering, malformed/duplicate/unknown
fields, and privacy. Constructor tests separately exercise the typed entry
point and its failure boundary. Validation used existing
`just test-media-root-contract` and the unchanged full gates. Local evidence is
retained under `artifacts/media-verification/2026-09-11-root-readiness-contract/`
in the primary checkout, including a command/result report and checksums. The
record remains blocked because the complete handoff criteria are not satisfied.

- Initial focused Linux run: exit 101, 57 passed and one failed. Independent
  tests demonstrated that Serde's derived struct decoder accepted positional
  JSON arrays. Retained the exact dirty implementation patch over `9edbfb6e`
  and the failure log; no negative expectation was removed.
- Fixed the representation using a map-only Serde visitor, preserving the
  derived field, type and duplicate checks without a second JSON parser.
  Added raw-JSON duplicate-field regressions and standalone row checks.
- Focused rerun: exit 0, 60 passed, zero failed or ignored, 24 unrelated tests
  filtered by the existing recipe. This includes 38 readiness tests and the
  22 existing root-input tests. Linux arm64 validation image identity is
  `sha256:64bc48fceaca573e4fdd6173ebb7a9113216d51b0ecd4c2fe2320b463dad4992`;
  this Ubuntu build environment is not a supported production image.
- `just fmt`, `just instruction-drift` and whitespace validation pass.
  Documentation links passed at the earlier draft with 1,249 checked and zero
  errors. `just docs-build` exited 0 but retained the 12,418,905-byte search-index
  warning. Final documentation validation is recorded in the retained report.
- The attempted `just --command sonar analyze secrets` file list did not run:
  sandboxed keychain/state access failed, and the permission reviewer rejected
  the broader source upload on retry. Requested operator consent for this exact
  deliverable's changed source/tests/docs under `VannaDii_Revaer`; the previous
  `final_sql.rb` consent is not reused. No clean Sonar or published coverage
  result is claimed.

### Final Executable Evidence

- Source `b65529a9` failed the first full CI attempt on two Rustdoc
  `doc_markdown` findings. Corrected the comments, without changing lint policy.
- Independent review also reproduced tagged unit-enum objects such as
  `{"source":null}` being accepted where `kind` must be a string. The isolated
  macOS regression run passed 60 tests and failed that case, for all five kinds.
  Its patch is retained against `b65529a9`; the original run output remains in
  the review task. String-only decoding fixes the finding.
- Final executable `ea4fa87a59edf172d436af418866d15f901d20cb` passes all 61
  focused root-contract tests on Linux arm64 and macOS: zero failed or ignored,
  24 unrelated tests filtered by the canonical recipe, no warnings. This
  comprises 39 readiness tests and the 22 existing root-input tests. The review
  found no other concrete issue in its bounded scope, not a whole-service or
  Copilot approval.
- `just ci` at that exact source exited 0. Both strict Clippy passes, workspace
  and minimal-feature tests, audit/dependency gates, UI build, all 18 package
  coverage thresholds, native/script coverage and final release build completed.
  It still emitted eight `config watcher task aborted during bootstrap shutdown`
  WARN lines. S2 is held; exit 0 is not warning-free acceptance.
- Rust LCOV retains 241 sources, 101,635 line records and 94,592 covered records;
  native LLVM text and script coverage are also retained. These are local
  coverage inputs, not published Sonar metrics or a Sonar gate result.
- Fresh-database `just ui-e2e` at the same source exited 1: 46 passed, one failed,
  61 not run. `tests/specs/api/media.spec.ts:130` expected profile creation HTTP
  201 and received 400. Teardown still reports unexercised
  `GET /v1/media/jobs/{media_job_public_id}/phases` and
  `GET /v1/media/profiles/{media_profile_public_id}/readiness`. Assertions,
  coverage requirements and fail-fast behavior are unchanged. JavaScript LCOV
  from the actual partial run contains 56 sources, 5,185 lines and 3,864 covered
  records; it is not a full UI pass.
- The executable diff from `bb302c03` passes the canonical changed-lines gate:
  1,738 additions and four deletions, 1,742/9,999. Final documentation adds only
  execution evidence. This is a local base/head result, not a revalidation of
  every existing GitHub PR.
- No production package, full root/API/UI workflow, init cutover, authoritative
  Sonar analysis, GitHub check or merge acceptance is established. No source
  was pushed; C1, D4, D5, E1, S2 and the other retained holds remain in force.

## Observability And Status Docs

No metrics, logs, health routes, HTTP errors or SSE behavior changes. The
bounded model error contains no supplied values. This record and the completion
ledger retain the distinction between a transport contract and a usable root
workflow; no release or support claim is advanced.

## Risk, Rollback And Dependencies

The main risk is a representation that disagrees with the accepted contract;
independent wire vectors and constructor tests target that boundary. Revert the
isolated model/tests if needed; no database, media or deployment needs rollback.

Use existing standard-library and Serde dependencies. Add only the workspace's
already locked `serde_json` as a dev dependency of `revaer-api-models`, to test
the actual JSON representation independently. No new package, version or
production dependency is introduced. C1, D4, D5, E1, S2 and other held choices
remain unapproved.

## Stale-Policy Check And Cleanup

Reviewed `AGENTS.md`, Rust, data, devops and Sonar instructions. The frozen SQL
and coordinated-cutover constraints rule out changing current root persistence
in this slice. Added a Rust contract reminder; no criteria, exception, numeric
choice, workflow or recipe was changed. Routine documentation follows ADR 587;
no new architectural decision is asserted.

The primary checkout's existing workflow changes and documentation conflicts
are preserved. The independent test and review worktrees were removed after
their files/results were retained. Linux validation containers exited and were
removed; the task-owned database containers and volumes were removed after each
run. `just clean-test-fixtures` completed, and container-local test media is gone.
The implementation worktree is removed after preserving the source and final
documentation in the retained integration; its removal also deletes build/test
output. No unrelated worktree, container, volume or media is cleanup scope.
