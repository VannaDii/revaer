---
applyTo:
  - "Cargo.toml"
  - "rust-toolchain.toml"
  - ".clippy.toml"
  - ".secignore"
  - "deny.toml"
  - "justfile"
  - "crates/**/*.rs"
  - "crates/**/Cargo.toml"
  - "tests/**/*.rs"
  - "scripts/**/*.rs"
---

`AGENTS.md` is the root contract. This file tightens Rust-specific guidance for the paths in `applyTo`.
Root policy takes precedence; these scoped rules may only tighten or specialize it.

# Rust Quality Rules

- Production and bootstrap Rust must be deterministic and panic-free.
- `panic!`, `unwrap()`, `expect()`, and `unreachable!()` are forbidden in authored production and bootstrap code.
- In this file, "bootstrap code" means startup/wiring runtime code, not test setup helpers.
- `todo!()` and `unimplemented!()` are forbidden in authored Rust. Split the work or delete the dead path instead of leaving stubs behind.
- Tests should prefer `Result`-returning flows and explicit assertions over `unwrap()` and `expect()`. Use panic-based helpers only when the behavior under test is itself a panic boundary.
- `Option<T>` is valid only for expected absence or partial-function semantics. Do not use it to hide I/O, validation, persistence, network, or parsing failure.
- `Result<T, E>` is required for recoverable failure, including `Result<(), E>` for side-effecting operations that can fail.
- `catch_unwind` is forbidden outside the FFI boundary shims covered by `ffi.instructions.md`.
- Silent suppression is forbidden. `let _ = expr;` is forbidden when `expr` returns a `Result` or `Option` that represents a failure mode.
- Discarding non-error values is acceptable when intentional; add a brief comment when the intent is not obvious.
- Errors are logged once at the origin point, then propagated as data.

# Authoring And Lint Hygiene

- Keep workspace lint posture aligned with `AGENTS.md`, the active `just` recipes, and crate-root attributes.
- Keep `.secignore` and the `[advisories].ignore` list in `deny.toml` empty. An exception requires exact operator consent, an expiry, and a matching guardrail change; an agent-authored ADR is not consent.
- `just lint` includes `scripts/policy-guardrails.sh`. Keep that guardrail aligned with the root policy when the lint posture changes.
- `scripts/policy-guardrails.sh` currently enforces empty advisory-ignore lists, no source-level lint suppressions, no authored stubs, FFI-only `unsafe`/`catch_unwind`, and the stored-procedure-only runtime SQL boundary. The `sqlx::query*` scan allows only `crates/revaer-data/src/**` plus the disposable test-database provisioning helper at `crates/revaer-test-support/src/postgres.rs` and its integration test. The inline DDL/DML scan is case-insensitive, excludes test-only sidecar modules at `crates/**/src/**/tests.rs`, and must keep working when `rg` is unavailable by falling back to the tracked Rust file list.
- `just lint` also runs a production-target Clippy pass on workspace libs, bins, and examples that forbids `panic!`, `unwrap()`, `expect()`, `unreachable!()`, `todo!()`, and `unimplemented!()` without applying those restrictions to test targets.
- Keep repo-level Clippy exceptions in `just lint`, not in crate source. Today that includes the ADR-backed `clippy::multiple_crate_versions` exception and the workspace `pub(crate)` style exception for `clippy::redundant_pub_crate`. The owning `clippy::cargo` and `clippy::nursery` groups are enforced from the Justfile for the same reason.
- `#[allow(...)]` and `#[expect(...)]` are not permitted in authored code, except
  for the exact generated CXX documentation exception in `ffi.instructions.md`.
  Split or redesign authored code instead.
- Committed vendored Rust is part of the Sonar source gate. Fix reported vendor findings with behavior-preserving private refactors and upstream-compatible public APIs; vendored ownership is not permission to exclude paths, suppress issues, or accept open findings.
- If custom cfgs are introduced, register them with `cargo::rustc-check-cfg` in `build.rs` or the manifest lint configuration. Do not silence `unexpected_cfgs`.
- Prefer `#[must_use]` for important return values and `pub(crate)` for internal APIs.
- FFI crates may omit a crate-wide `forbid(unsafe_code)` if necessary, but unsafe code must stay isolated to the documented boundary modules and shims. Do not use lint suppressions to permit unsafe.

# CI And Recipe Maintenance

- Native discovery must invalidate Cargo when `PATH`, pkg-config search variables,
  or target-qualified pkg-config variables change.

- Keep the workspace MSRV aligned with the pinned Rust toolchain. Release build
  dependencies retain symbols (`strip = "none"`) for the pinned Rust 1.96 LLVM
  loader workaround; this setting applies only to build dependencies.

- `just test`, `just test-features-min`, `just test-native`, `just db-migrate`, `just cov`, and `just validate` default `REVAER_TEST_DATABASE_URL` through `scripts/local-postgres-url.sh` or explicitly supplied environment values. Do not document or reintroduce password-bearing database URI literals in Rust instructions, recipes, workflows, tests, or examples; local defaults may use non-production credentials only when the connection string is composed from variables or helper scripts. `just db-start` must keep local endpoint normalization between `localhost` and `host.docker.internal`, use explicit `REVAER_DB_MANAGED=1` provenance when a caller synthesized a local `DATABASE_URL` and expects Docker lifecycle ownership, honor an already reachable explicit non-Docker `DATABASE_URL` when managed mode is not requested, reject managed mode for non-local endpoints, use portable TCP probes that do not require Python-only environments, support `REVAER_DB_DATA_DIR` for isolated local test data directories, provision local Docker Postgres containers with enough shared memory for migration-heavy test runs, wait for local containers to exit recovery before migrations, and retry transient Postgres recovery/startup/not-yet-accepting-connection errors around `sqlx migrate run` and `sqlx database reset` before treating the database as mismatched. Because `just` recipe lines do not share shell state, recipes that compute database URLs must pass `REVAER_TEST_DATABASE_URL`, `DATABASE_URL`, and any required managed-database provenance directly into every command that depends on them. When one recipe line chains database setup and tests, join dependent commands with `&&` so an earlier failure cannot be hidden by a later command. `just test-features-min` intentionally runs library, binary, and integration tests for `revaer-api` and `revaer-app` without invoking rustdoc; full workspace doctests remain covered by `just test`, while the minimal-feature gate stays focused on feature-configuration behavior. Keep recipes and docs aligned with that admin-connection workflow when test database bootstrapping changes.
- Tool-version comparisons in `justfile` must stay portable across GNU and BSD userspace; do not rely on GNU-only `sort -V`.
- Rust CLI tools invoked by required recipes must be installed at an exact reviewed version with the published lockfile. Floating or unlocked `cargo install` commands are forbidden because registry resolution drift can make identical commits fail before their gates run. The shared `trunk-install` recipe owns Trunk `0.21.14`; UI recipes must depend on it, and harness code must only consume the provisioned binary instead of installing Trunk ad hoc.
- `just cov` records coverage once with `cargo llvm-cov --workspace --all-features --no-report`, then enforces the 90% per-package line threshold with `cargo llvm-cov report --package ...` against that shared workspace dataset. Keep the coverage gate workspace-sourced so library crates receive credit for lines exercised by downstream crates and integration tests.
- The `revaer-torrent-libt` build script must select exactly one native source per build: bundle, paired explicit include/library override, pkg-config, or one coherent fallback prefix. It must propagate pkg-config defines, select Boost and OpenSSL headers from the same installation family ahead of ambient system headers, accept libtorrent `>=2.0.10,<2.2.0`, and verify that the selected macOS libtorrent library contains the Cargo target architecture before enabling the native backend. When pkg-config omits `TORRENT_ABI_VERSION`, query the configured C++ preprocessor for the effective packaged-header ABI and fail closed if it cannot be proven. Manual bundle, override, and fallback sources must provide their complete build definitions through `LIBTORRENT_DEFINES`, including `TORRENT_ABI_VERSION`; missing native ABI metadata fails closed.
- `just test-native` serializes native libtorrent tests with `--test-threads=1`; keep all native tests enabled and do not reintroduce overlapping native sessions without task-recorded evidence across the supported 2.0.x and 2.1.x API lines.
- During the accepted `rv` migration, preserve that Rust-only 90% threshold while
  adding native instrumentation and complete Rust/C++ evidence. Per-package JSON
  must retain native measurements even though the existing numeric gate measures
  Rust. Published reports must not hide authored files. Install `rust-src` through
  rustup and use LLVM path equivalence to render compiler-mapped standard-library
  sources; do not suppress missing-source diagnostics or filter those files away.

# Documentation

- The media fixture catalog retains all 30 declared fixtures and their expected
  probe snapshots so runtime integration tests use the complete qualified inputs.

- Public crates need crate-level rustdoc that explains purpose, invariants, and a realistic usage example.
- Externally consumed public items should document:
  - behavior
  - invariants and assumptions
  - error cases
  - panic behavior, if any exists
  - copy-pasteable examples when the item is meant to be used directly
- Prefer examples that use `?` and explicit error handling over `unwrap()`/`expect()`.

# Maintainability And Layout

- Media execution retains checkpoint replay, verified replacement, cooperative
  shutdown and protected scratch cleanup. Filesystem capacity probing is shared
  by media admission and filesystem post-processing.

- The application media facade and discovery runtime receive the existing media
  store, native capability detector, catalog source and watcher. `notify` provides
  the native watcher, `rustix` retains descriptor identity, `sha2` computes aggregate
  identities, and `yaml-rust2` validates portable configuration before persistence.

- `revaer-media-runtime` owns injected native process, capability and inspection
  adapters. Its restored `rustix` dependency provides safe Unix descriptor and
  process operations without authored unsafe code.
- Root catalogs retain descriptor, mount, source identity and root-lock checks.
  Their restored `sha2` dependency computes canonical identities; `tempfile`
  isolates functional filesystem regression fixtures.
- Command execution and managed scratch workspaces retain cooperative cancellation,
  live resource limits, capacity admission and terminal cleanup policy.
- Job planning, output verification and replacement retain source-identity and
  recovery checks. The `media_fixtures` integration target executes real conversions
  and verifies their resulting media graphs.

- `revaer-media-core` owns deterministic media graph compilation, planning,
  compliance and verification. Keep filesystem, process and database access in
  injected runtime adapters.

- Keep files single-purpose and cohesive.
- Target roughly 300-400 non-test LOC per production file. Split large files instead of silencing `too_many_lines`.
- `lib.rs` should stay limited to crate docs, module declarations, light re-exports, and tiny crate-boundary glue.
- `main.rs` must remain a thin bootstrap entry point.
- Name modules for what they own. No grab-bag files mixing domain logic, transport DTOs, adapters, and orchestration.

# Performance

- Measure before optimizing. Do not land “performance” refactors based on taste or folklore.
- For performance-driven changes, record the command, benchmark, trace, or timing report that justified the change in the task record or ADR.
- Prefer simpler, more explicit code unless measurement shows a real hotspot.

- The migrated `rv test-runtime-shutdown` and `rv lint-runtime-shutdown` retain
  both all-feature and no-default-feature configurations. Each test selection
  must execute positive passing tests; preserve both existing Clippy passes.

- The reusable database initializer has its own success-path integration test in
  the foundation. It verifies exact initializer digest sealing, restricted
  runtime identity, owner login disablement, repeated-init rejection and owned
  database/role cleanup; do not rely on downstream media consumers for coverage.
