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

- Keep workspace lint posture aligned with `AGENTS.md`, the active `rv` tasks, and crate-root attributes.
- Keep `.secignore` and the `[advisories].ignore` list in `deny.toml` empty. An exception requires exact operator consent, an expiry, and a matching guardrail change; an agent-authored ADR is not consent.
- `rv lint` includes `tools/src/revaer_tooling/tasks/policy.py`. Keep that guardrail aligned with the root policy when the lint posture changes.
- `tools/src/revaer_tooling/tasks/policy.py` currently enforces empty advisory-ignore lists, no source-level lint suppressions, no authored stubs, FFI-only `unsafe`/`catch_unwind`, and the stored-procedure-only runtime SQL boundary. The `sqlx::query*` scan allows only `crates/revaer-data/src/**` plus the disposable test-database provisioning helper at `crates/revaer-test-support/src/postgres.rs` and its integration test. The inline DDL/DML scan is case-insensitive, excludes test-only sidecar modules at `crates/**/src/**/tests.rs`, and must keep working when `rg` is unavailable by falling back to the tracked Rust file list.
- `rv lint` also runs a production-target Clippy pass on workspace libs, bins, and examples that forbids `panic!`, `unwrap()`, `expect()`, `unreachable!()`, `todo!()`, and `unimplemented!()` without applying those restrictions to test targets.
- Keep repo-level Clippy exceptions in `rv lint`, not in crate source. Today that includes the ADR-backed `clippy::multiple_crate_versions` exception and the workspace `pub(crate)` style exception for `clippy::redundant_pub_crate`. The owning `clippy::cargo` and `clippy::nursery` groups are enforced from the task registry for the same reason.
- `#[allow(...)]` and `#[expect(...)]` are not permitted in authored code. Split or redesign the code instead.
- Committed vendored Rust is part of the Sonar source gate. Fix reported vendor findings with behavior-preserving private refactors and upstream-compatible public APIs; vendored ownership is not permission to exclude paths, suppress issues, or accept open findings.
- If custom cfgs are introduced, register them with `cargo::rustc-check-cfg` in `build.rs` or the manifest lint configuration. Do not silence `unexpected_cfgs`.
- Prefer `#[must_use]` for important return values and `pub(crate)` for internal APIs.
- FFI crates may omit a crate-wide `forbid(unsafe_code)` if necessary, but unsafe code must stay isolated to the documented boundary modules and shims. Do not use lint suppressions to permit unsafe.

# CI And Recipe Maintenance

- Runtime and test database endpoints come from injected typed settings. In ADR 591 feature-development mode, CI/E2E initialize owned disposable databases with the complete initializer; existing databases are read-only baseline verified. Historical migration/reset/seed operations must fail closed in that mode. Rust fixtures explicitly initialize and seal runtime roles, preserve negative empty-database tests, and report owned cleanup failures. Native tests remain serial; minimal-feature tests retain both API/app integration targets and no-default-feature selection.
- Tool-version comparisons in `justfile` must stay portable across GNU and BSD userspace; do not rely on GNU-only `sort -V`.
- Rust CLI tools invoked by required recipes must be installed at an exact reviewed version with the published lockfile. Floating or unlocked `cargo install` commands are forbidden because registry resolution drift can make identical commits fail before their gates run. The shared `trunk-install` recipe owns Trunk `0.21.14`; UI recipes must depend on it, and harness code must only consume the provisioned binary instead of installing Trunk ad hoc.
- `rv cov` records coverage once with `cargo llvm-cov --workspace --all-features --no-report`, then enforces the 90% per-package line threshold with `cargo llvm-cov report --package ...` against that shared workspace dataset. Keep the coverage gate workspace-sourced so library crates receive credit for lines exercised by downstream crates and integration tests.
- The `revaer-torrent-libt` build script must select exactly one native source per build: bundle, paired explicit include/library override, pkg-config, or one coherent fallback prefix. It must propagate pkg-config defines, select Boost and OpenSSL headers from the same installation family ahead of ambient system headers, accept libtorrent `>=2.0.10,<2.2.0`, and verify that the selected macOS libtorrent library contains the Cargo target architecture before enabling the native backend. When pkg-config omits `TORRENT_ABI_VERSION`, query the configured C++ preprocessor for the effective packaged-header ABI and fail closed if it cannot be proven. Manual bundle, override, and fallback sources must provide their complete build definitions through `LIBTORRENT_DEFINES`, including `TORRENT_ABI_VERSION`; missing native ABI metadata fails closed.
- `rv test-native` serializes native libtorrent tests with `--test-threads=1`; keep all native tests enabled and do not reintroduce overlapping native sessions without task-recorded evidence across the supported 2.0.x and 2.1.x API lines.
- During the accepted `rv` migration, preserve that Rust-only 90% threshold while
  adding native instrumentation and complete Rust/C++ evidence. Per-package JSON
  must retain native measurements even though the existing numeric gate measures
  Rust. Published reports must not hide authored files. Install `rust-src` through
  rustup and use LLVM path equivalence to render compiler-mapped standard-library
  sources; do not suppress missing-source diagnostics or filter those files away.

# Documentation

- Public crates need crate-level rustdoc that explains purpose, invariants, and a realistic usage example.
- Externally consumed public items should document:
  - behavior
  - invariants and assumptions
  - error cases
  - panic behavior, if any exists
  - copy-pasteable examples when the item is meant to be used directly
- Prefer examples that use `?` and explicit error handling over `unwrap()`/`expect()`.

# Maintainability And Layout

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

# Accepted media runtime contracts

- During discovery, a vanished candidate or descendant parent is expected
  absence only after the retained declared root revalidates. Record the skipped
  candidate and continue the batch; never downgrade declared-root loss or other
  filesystem failures to absence.
- Discovery directory inventory counts every raw filename byte before decoding
  or filtering, rejects partial/over-limit inventories, and compares retained
  descriptor observations before and after enumeration. A changed inventory
  cannot supply stable aggregate membership or a clean absence census.
- ADR 593 supersedes ADR 588/589's custom owner/controller and fingerprint
  registration requirements. Restart unfinished steps and discard or overwrite
  only their Revaer-owned temporary outputs. Preserve originals until verified
  final replacement; reconcile an interrupted replacement before continuing.
  Stop previous work before replay, retain ordinary cancellation/deadlines, and
  record failures. Do not rebuild a custom ownership or control protocol.
- Accepted ADR 594 replaces ADRs 512/513 and the named discovery coordination
  contracts. Implement the full workflow locally; distributed workers and their
  scaffolding are not approved. Rescan interrupted discovery without duplicate
  jobs, reuse completed job checkpoints, and rebuild unfinished temporary outputs.
  Budget peak scratch across concurrent jobs, remove obsolete intermediates after
  their last consumer, and retain only needed recovery artifacts. Failed cleanup
  stays charged until files are removed; retry it individually. Keep the existing
  Revaer root lock and replacement safety, not distributed leases, recovery
  leadership, rolling debit ledgers or an independent cleanup-owner protocol.
- ADR 557 root startup precedes media facade and worker construction. Load the
  approved source once, retain source descriptors and root locks through service
  shutdown, and revalidate immediately before atomic activation. Missing or
  rejected proof must persist the closed unavailable state without substituting
  old readiness; a persistence failure prevents startup. Successful capability
  probes alone never prove deployment ownership or persistent durability.
  ADR 592 removes native external-writer assertions, not active-file change
  handling. Unqualified filesystem/deployment evidence remains unavailable;
  focused tmpfs tests are not package qualification or permission to drop the
  required persistent-storage and Kubernetes paths.
