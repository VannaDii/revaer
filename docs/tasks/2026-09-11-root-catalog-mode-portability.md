# Root Catalog Mode Portability

- Status: Blocked
- Date: 2026-09-11
- Operator approval: Not applicable: nonarchitectural task record
- Scope / owner / PR: Integrating agent; root-catalog mode conversion and its
  existing evidence regression; unpublished child of governance `d5662941`.
- Approved authority: ADR 550 source trust contract and ADR 559 narrow G1.
  No new architecture, platform support, failure classification or trust rule.
- Requirement ledger: [L2](../adr/564-media-completion-ledger.md#outside-in-ledger).

## Motivation And Design Notes

Canonical Linux `just ci` on the governance tree reached Clippy, then failed
three `useless_conversion` diagnostics in `root_catalog/local_file.rs` at the
file evidence and directory/file trust checks. The native `st_mode` is `u32`
on Linux but narrower on macOS. Blindly removing widening would break the
development build; filtering through permission-only flags would lose evidence.

A single private generic lossless conversion normalizes the native value at
the existing three boundaries. `Into<u32>` accepts both native widths without
casts, truncation, new I/O, extra failure states, cfg-dependent behavior or lint
suppression. Strengthen the existing real-file load test to compare the entire
recorded mode with `MetadataExt::mode()`, retaining the permission assertion.
The trust mask remains `0o022`; ownership, symlink, race and platform gates are
unchanged. No dependency or public API changes.

## Test Coverage Summary

Retained evidence: `artifacts/media-verification/2026-09-11-delivery-governance`
in the primary checkout, outside completed temporary worktrees.
Pre-fix `ci-volume.log` records exit 101 from the three Linux lint errors.
The earlier `ci.log` stopped at disposable bind-mount database ownership; an
owned Docker volume resolved startup without changing the gate. Initial
governance policy checks pass with a private GPG home.

Source revision: `c8d83bf18317740f5cff759fe990235d4850da2d`, clean at the full
CI/UI starts. Final result-record edits are documentation only.

- `just test-media-root-catalog`: 59 passed on Linux arm64, exit zero; the
  canonical focused recipe does not run unrelated tests or the fixture suite.
- The same recipe on macOS arm64 compiled without warnings, then initially
  failed 19 of 59 cases because the sandbox denied private fixture creation.
  The unchanged permission-approved rerun passed all 59 with zero warnings or
  ignored tests, including the full-mode assertion. The fixture inventory was
  empty before and after. Both logs and manifests are retained.
- Full Linux `just ci`: exit zero, both strict workspace Clippy passes,
  full/minimal-feature tests, Rust dependency checks, UI build, all 18 package
  coverage gates, script coverage and release build completed. The log retains
  eight `config watcher task aborted during bootstrap shutdown` WARN lines;
  S2 remains held and this is not a warning-free acceptance claim.
- Rust LCOV: 238 source records, 101,449 line records, 94,432 covered. Full native
  text and script-coverage reports are retained. These are local measurements,
  not published Sonar metrics.
- Full Linux `just ui-e2e`: exit one; 46 passed, one failed, 61 not run. Profile
  creation at `tests/specs/api/media.spec.ts:130` expected 201 and received 400.
  Teardown also lacks `GET /v1/media/jobs/{media_job_public_id}/phases` and
  `GET /v1/media/profiles/{media_profile_public_id}/readiness` coverage. Retained
  JavaScript LCOV has 56 source records, 5,185 lines and 3,864 covered; positive
  records do not make the suite or teardown pass.
- Linux validation used existing image
  `sha256:64bc48fceaca573e4fdd6173ebb7a9113216d51b0ecd4c2fe2320b463dad4992`,
  Rust 1.96.0, Just 1.49.0 and NVM-selected Node 24.19.0. This Ubuntu validation
  image on Docker Desktop is not either supported production package.
- Instruction drift, formatting, whitespace and canonical size passed. The
  governance boundary is 555/9,999 changed lines; this source boundary is
  118/9,999 before final documentation-only result updates.

The correction is implemented and its focused regressions pass. Handoff remains
blocked by the required full E2E result and retained warnings; no assertions,
coverage scope or gates were relaxed. No amd64 package, dedicated full fixture
suite, published Sonar, remote-check, review or merge result follows from this
correction. No held C1, D4, D5, E1, S2 or other decision is released.

## Observability And Status Docs

No runtime telemetry or diagnostic change. The full mode evidence is unchanged.
Link this record from the task index, book navigation and completion ledger;
leave the specification's scope and architectural approvals intact.

## Risk, Rollback And Dependencies

Risk is accidentally changing the evidence or trust predicate across platforms;
the full-mode comparison and existing rejection/race tests are required. Revert
only this bounded correction to restore the original conversion call sites and
Linux lint failure. No dependency, gate, migration or package input changes.

## Stale-Policy Check And Cleanup

Reviewed AGENTS.md, Rust/FFI scoped instructions, source/evidence types and
existing root-catalog tests. Added the exact cross-platform preservation note
to Rust guidance. No architecture/policy contradiction is reinterpreted as
approval. Work is isolated from the conflicted primary checkout. Completed
agent worktrees, their private build output, owned test containers and database
volumes were removed; `just clean-test-fixtures` removed ignored fixture media.
The temporary integration worktree is removed after retaining these commits
and the non-media reports. No unrelated container, worktree or user edit is
removed. No source was pushed from this task.
