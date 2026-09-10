# Single-file directory authoring regression

- Status: Recorded
- Date: 2026-09-10
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - Integrated torrent-authoring E2E returned HTTP 400 for a fresh directory
    containing only the ten-byte `seed.txt` payload `revaer e2e`. Native tests
    previously covered a direct file and a two-file directory, not this shape.
- Decision:
  - Add the missing native regression with the default author request: no
    piece-length override, trackers, private flag, or filters. Preserve all
    existing assertions and native authoring behavior.
  - Stop the bounded hash-path investigation after the exact case succeeds on
    the local native backend. Do not change C++ on the basis of the hypothesis.
- Consequences:
  - The one-file directory now has exercised default-piece-size coverage and
    must preserve its torrent root name and relative file path.
  - This is regression evidence, not a fix for the reported HTTP failure or
    proof that libtorrent 2.0 and supported Linux packages behave identically.
- Follow-up:
  - Parent captures the actual failing `TorrentError::OperationFailed` source.
    Distinguish `create_torrent.validate_path` from engine `create_torrent` and
    inspect `LibtorrentError::NativeFailure.message` when that variant exists.
    The pre-engine allow-path check can reject the request independently of
    native hashing; that is a diagnostic branch, not a diagnosed root cause.
  - Parent integrates the regression and runs full CI/UI. No API, allow-path,
    filesystem, bootstrap, piece-size, or native-contract change is selected.

## Task Record

- Motivation:
  - Verify the singleton-directory normalization hypothesis before modifying
    native hashing and close the specifically identified test-shape gap.
- Design notes:
  - The existing native harness creates fresh temporary roots and removes them
    through `TempDir`. The new test writes exactly `seed.txt` and ten bytes.
  - Assertions require metainfo, magnet URI, one relative `seed.txt` result,
    exact file and total size, a supported effective piece size, no warnings,
    the directory name in metainfo, and the one-file directory path record.
  - The FFI request mapping preserves `piece_length: None` as an unset override.
    No C++ code, public API, or dependency changed.
- Test coverage summary:
  - Base commit: `1f8bc6575f1dbce7caa88fcf7a1020a1e48c7420`.
  - Before any production edit, `just test-native` passed 95 library tests
    (94 existing plus this regression), seven build guardrails, and three
    native integration tests with `REVAER_NATIVE_IT=1` and serial execution.
  - Build output selected the real macOS arm64 native backend and the coherent
    Homebrew libtorrent-rasterbar `2.1.1_1` headers/library; `pkg-config`
    reported `2.1.1`. The new test executed, rather than being compiled out.
  - There is no failure-before/pass-after result: unchanged production code
    passed the exact reproduction. No `NativeFailure` was produced to inspect.
  - `just fmt` and `just lint` passed, including policy and both strict Clippy
    passes. A repeat `just test-native` passed the same 95 + 7 + 3 tests.
    Logs are preserved outside the disposable worktree at
    `/Users/vanna/Source/revaer-reviews/2026-09-10/torrent-author-single-file-570`.
- Observability updates:
  - None. Do not expose structured native errors through a changed HTTP surface
    merely to investigate this bounded regression.
- Status-doc validation:
  - The original HTTP failure remains unresolved by this slice. No integrated
    E2E, full CI, Linux package, or published Sonar completion claim is made.
- Risk & rollback plan:
  - Test-only runtime delta; revert this regression if its documented existing
    authoring expectations prove incorrect. No production rollback is needed.
- Dependency rationale:
  - None added. Reuse the existing native harness and metainfo assertions.
- Stale-policy check:
  - Reviewed root `AGENTS.md`, Rust/FFI scoped instructions, Sonar guidance, and
    canonical native/quality recipes. No recipe, instruction, accepted constant,
    quality threshold, or architectural approval changed.

## Harness Hypothesis

- Source inspection at this task's base confirms that
  `crates/revaer-app/src/orchestrator.rs:167` checks the authoring root before
  invoking the engine. `ensure_authoring_path_allowed` at line 201 reads the
  filesystem policy; `enforce_allow_paths` at line 1209 rejects an unmatched
  lexical or canonical root with `allow_paths` / `root_not_permitted`.
- The frozen seed in
  `crates/revaer-data/migrations/0067_factory_reset_seed_defaults.sql:159`
  supplies only `.server_root/downloads` and `.server_root/library` as allowed
  roots. `tests/support/paths.ts:16` resolves `E2E_FS_ROOT`, and
  `tests/specs/api/torrents.spec.ts:23` places the one-file authoring fixture
  beneath that root.
- Parent reports that the last three failing runs used `target/e2e-root` or
  the repository root. Those fixture locations are outside the seeded roots;
  an allowlist mismatch is therefore a likely pre-engine failure, not evidence
  of incorrect native hashing. This slice has not inspected the live policy or
  proved that this rejection caused the observed HTTP 400.
- Parent will rerun `just ui-e2e` with
  `E2E_FS_ROOT="$PWD/.server_root/library"` after its already-running gate.
  Preserve the allowlist and all assertions. The earlier 46-pass run does not
  prove its root selection, and the proposed rerun is not yet API evidence.

## Validation Boundaries

- `just docs-index` generated 512 entries. `just docs-build` succeeded with a
  large-search-index warning; this is not a warning-free documentation claim.
  `just docs-link-check` passed all 1,066 links. `just instruction-drift` and
  `just clean-test-fixtures` passed. No test media is retained.
- Full-file MAIN-scope Sonar secrets analysis of the modified `native.rs`
  returned zero issues. The available snippet analyzer has neither Rust nor
  C++ semantic analysis; this is not a native C-family or full Sonar gate pass.
  Full repository Sonar remains with parent integration, without suppression
  or criteria changes.
- No database was provisioned for this native investigation. Parent owns
  integrated `just ci` and `just ui-e2e`. Its concurrent coverage run against
  `fe227810` does not include this new test; integration and fresh validation
  are required before making that claim. The HTTP 400 remains undiagnosed.
