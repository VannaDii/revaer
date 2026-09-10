# Asset sync failure-path coverage

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - The exact failed PR #131 coverage log identifies only `asset_sync` below the
    unchanged 90% per-package line threshold. The failure is at lines 4027-4045
    of the parent-retained `pr131-coverage.log`; its timestamp is 2026-08-16.
  - The assigned base is `c487a899317c9ed70aa4a90e648a2342883d32f7`.
    A local comparison confirms no `asset_sync` diff from PR head `4681b5c4`.
    This is historical failure evidence, not a new GitHub check result.
  - The parent owns active full-workspace CI. This task is limited to this
    crate's tests and this record, without database, browser, publishing,
    dependency, production-behavior, or gate-criteria changes.
- Decision:
  - Add nine deterministic tests in the existing inline test module. Exercise
    actual validation and filesystem errors and assert their paths, reasons,
    preserved bytes, and refusal to write a lock after invalid references.
  - Retain the production implementation, error model, Rust edition, dependency
    graph, threshold, scanner scope, and canonical recipes unchanged.
- Consequences:
  - Missing validation and error paths now execute under real tests. Coverage
    diagnostics do not authorize merging or substitute for the parent gates.
  - Non-UTF-8 path construction is Unix-specific; no filesystem encoding or
    privilege assumptions are needed to execute that path-validation branch.
- Follow-up:
  - The parent must obtain Linux runner proof and run integrated `just ci` and
    `just ui-e2e` before repository completion. The historical Linux failure
    was not reproduced locally: the unmodified macOS baseline already passed.

## Task Record

- Motivation:
  - Repair the earliest confirmed stack coverage failure with meaningful tests,
    without weakening the package threshold or changing unrelated stack work.
- Design notes:
  - Cover empty, NUL-containing, missing, malformed, incomplete, self-closing,
    unnamespaced, and unclosed SVG envelopes, plus both valid namespace quote
    forms and surrounding whitespace.
  - Exercise case-insensitive raster rejection, extensionless and non-text
    assets, non-UTF-8 extensions, missing runtime-text reads, invalid UTF-8
    DataTables reads, legacy and missing final avatar references, already
    canonical JavaScript, and parentless copy destinations.
  - A full fixture-driven sync proves the missing last avatar reference returns
    the emitted JavaScript path and exact error reason without creating a lock.
  - Reuse existing private fixture roots and automatic cleanup. Do not construct
    error variants merely to claim execution of the production failure paths.
- Test coverage summary:
  - Fresh execution details and remaining gaps are recorded below.
- Observability updates:
  - None. Tests verify existing error classifications, paths, reasons, and the
    underlying I/O source; no logging or runtime telemetry is changed.
- Status-doc validation:
  - No user-facing capability or operator workflow changes. README/status claims
    remain unchanged. Register this record in the ADR index, mdBook summary,
    and regenerated documentation catalogs.
- Risk & rollback plan:
  - Test-only additions cannot change shipped behavior. Revert this commit if
    a test portability issue is confirmed, keeping the original failing gate
    visible until an equivalent deterministic regression replaces it.
- Dependency rationale:
  - No new or changed dependencies. Tests use `std` and existing fixture helpers.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust/UI/devops scoped instructions, the ADR template,
    and the canonical quality, UI, and documentation recipes.
  - No scoped policy relaxation or stale reference was introduced or removed.
    Existing quality recipes still contain database defaults that differ from
    the scoped Rust instructions; this unrelated drift is left to the parent.
  - Per the explicit task boundary, full CI/E2E remain parent-owned, not waived.

## Fresh Validation

- Host: Darwin arm64, Rust 1.96.0, cargo-llvm-cov 0.8.7. All Cargo diagnostics
  use this worktree's absolute `CARGO_TARGET_DIR`, with two build jobs.
- No focused package recipe exists. Use bounded `just --command` diagnostics
  without editing recipes or claiming that they replace canonical `just cov`:

  ```sh
  CARGO_TARGET_DIR="$PWD/target" CARGO_BUILD_JOBS=2 RUSTFLAGS=-Dwarnings \
    just --command cargo llvm-cov -p asset_sync --all-features \
      --show-missing-lines --fail-under-lines 90
  ```

- Baseline: 33 library tests plus one CLI test pass; 881/967 lines, 91.11%.
  Final: 42 library tests plus one CLI test pass; 1060/1108 lines, 95.67%,
  with zero failures, ignored tests, or filtered tests.
- Forty previously uncovered lines in the unchanged library body execute after
  the additions. Before adding the new test lines to the denominator, the
  original library's missed-line count fell from 86 to 46. This is not merely
  an increase in covered test scaffolding.
- An initial sidecar was omitted by cargo-llvm-cov's test-directory heuristic;
  move the tests into the existing inline module so they remain in reported
  scope. No exclusion or scanner setting was added or changed.
- The initial non-UTF-8 filename fixture failed on macOS with `Illegal byte
  sequence` before reaching the validator. The final test passes the Unix
  byte-valued path directly: extension validation occurs before any file read.
  It executes the real rejection without requiring the host to create that name.
- Both package-scoped Clippy passes matching the existing canonical flags pass
  with warnings denied. No lint suppressions were added.
- The uninstrumented package tests also pass with `RUST_TEST_THREADS=1` and
  warnings denied: 42 library tests, one CLI test, and zero doctests.
- Remaining unexecuted production paths include invalid compile-time manifest
  ancestry, deletion failures during directory replacement, canonicalized-JS
  write failures, runtime directory traversal failures, and traversal metadata
  failures. Privilege changes, permission races, and injected production hooks
  were deliberately not added. Some remaining uncovered lines are existing
  fallible test-helper and assertion-diagnostic paths.
- Stable branch counters are not enabled by the existing coverage toolchain;
  these are executed failure-path assertions and line/region evidence, not a
  claim of complete branch coverage. Linux runner evidence is still required.
- `just fmt`, `just instruction-drift`, `just docs-index` (504 entries), and
  `git diff --check` pass. Reviewed the entire diff: only inline tests, this
  record, its two index entries, and its two generated catalog entries change
  (plus the generated catalog timestamp).
- `just clean-test-fixtures` passes; the private `.server_root` fixture parent
  is empty. No generated test media remains. Parent-owned worktrees and media
  directories are untouched.
- Retained the baseline/final JSON reports, final LCOV, and final test/coverage
  log outside the disposable worktree in the parent's
  `revaer-reviews/2026-09-10/asset-coverage-562/` evidence directory.
- Full CI, database tests, UI E2E, browsers, GitHub writes, pushes, and media
  acquisition are not run. Integrated parent gates and Linux proof remain open.
