---
applyTo:
  - "Cargo.toml"
  - "rust-toolchain.toml"
  - ".clippy.toml"
  - ".secignore"
  - "deny.toml"
  - "justfile"
  - "just/**"
  - "crates/**/*.rs"
  - "crates/**/Cargo.toml"
  - "tests/**/*.rs"
  - "scripts/**/*.rs"
---

`AGENTS.md` is the root contract. This file tightens Rust-specific guidance for the paths in `applyTo`.
If any Rust-path rule in this file conflicts with `AGENTS.md`, the root contract wins.

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
- `#[allow(...)]` and `#[expect(...)]` are not permitted in authored code. Split or redesign the code instead.
- Committed vendored Rust is part of the Sonar source gate. Fix reported vendor findings with behavior-preserving private refactors and upstream-compatible public APIs; vendored ownership is not permission to exclude paths, suppress issues, or accept open findings.
- If custom cfgs are introduced, register them with `cargo::rustc-check-cfg` in `build.rs` or the manifest lint configuration. Do not silence `unexpected_cfgs`.
- Prefer `#[must_use]` for important return values and `pub(crate)` for internal APIs.
- FFI crates may omit a crate-wide `forbid(unsafe_code)` if necessary, but unsafe code must stay isolated to the documented boundary modules and shims. Do not use lint suppressions to permit unsafe.
# CI And Recipe Maintenance

- Preserve ADR 501's inherited-pipe cleanup and error contract when repairing
  native supervision. Observe EOF on both owned captures in the existing
  bounded verification phases, without a fresh grace period. Keep read failures,
  deadlines and final unclosed-pipe evidence intact; missing captures on a
  failed setup path are not proof of EOF. This does not qualify full S2.
- Torrent orchestrator refresh must retain the previous `engine_profile` through
  preparation and until both engine application and global limit updates have
  completed successfully. Pass the current candidate explicitly to blocklist
  metadata preparation; never publish it early to supply that helper's input.
  Preserve the existing startup-before-watcher and awaited-refresh sequencing.
  This ADR 588 S2 field-publication boundary does not serialize arbitrary new
  callers or make native mutations, best-effort metadata changes, filesystem
  policy or the complete configuration revision atomic.
- ADR 588 S2's application shutdown authority must retain the first monotonic
  origin and only shorten its shared absolute deadline within the unchanged
  30-second application budget. Bootstrap must latch it before background-task
  waits; media-task cooperative waits must observe subsequent shortening and
  must not allocate fresh per-task grace. Preserve warnings and real join
  classification. This local timer authority does not qualify independent
  PID1 enforcement, post-abort/native/configuration settlement or recovery.
- Bootstrap's configuration watcher shares that same authority with the media
  tasks. Stop polling and recheck drain before admitting a snapshot; once
  admitted, await its existing serial apply and completed limits result. Join
  cooperatively before the same shrinking cutoff, with no unbounded post-abort
  wait or fresh grace. Preserve WARN for deadline/abort, unconfirmed settlement
  and genuine join failure. A watcher join does not prove listener/pool, native
  worker/destructor, revision-wide or PID1 quiescence; those S2 obligations remain.
- ADR 586/588's E2E serving entry and its launch selection belong only in
  `cfg(test)` app bootstrap code. It must invoke the shared typed compliance
  preflight with an explicit test loader, then run the real application without
  changing setup/auth configuration or substituting runtime dependencies.
  Ordinary Rust runs must exercise its launch guards rather than starting an
  unrequested server. Preserve production-entry missing/error regressions even
  when the test-only launch variable is present; do not add a production feature,
  environment override, fixture fallback or packaged compliance artifact.
- Bootstrap test children that clear their environment must preserve a supplied
  `LLVM_PROFILE_FILE` exactly so their executed coverage reaches the collector.
  Keep other inherited variables cleared and test supplied, empty and absent
  instrumentation values. Never change production environment handling or
  discard child profiles to manufacture a cleaner coverage result.
- Database-backed Rust gates must follow the explicit disposable-database input
  contract in `devops.instructions.md`. Do not restore connection defaults,
  password-bearing URI literals, or helper-composed credential fallbacks.
  `just db-start` must keep local endpoint normalization between `localhost` and
  `host.docker.internal`, use explicit `REVAER_DB_MANAGED=1` provenance when a
  caller synthesized a local `DATABASE_URL` and expects Docker lifecycle
  ownership, honor an already reachable explicit non-Docker `DATABASE_URL` when
  managed mode is not requested, reject managed mode for non-local endpoints,
  use portable TCP probes that do not require Python-only environments, support
  `REVAER_DB_DATA_DIR` for isolated local test data directories, provision local
  Docker Postgres containers with enough shared memory for migration-heavy test
  runs, wait for local containers to exit recovery before migrations, and retry
  transient Postgres recovery/startup/not-yet-accepting-connection errors around
  `sqlx migrate run` and `sqlx database reset` before treating the database as
  mismatched. Pass the resolved database inputs and required managed-database
  provenance directly into every dependent child as required by the canonical
  contract; recipe lines do not share shell state. When one recipe line chains
  database setup and tests, join dependent commands with `&&` so an earlier
  failure cannot be hidden by a later command.
  `just test-features-min` runs library, binary, integration, and doctests for
  `revaer-api` and `revaer-app` under their minimal feature configurations. Full
  workspace doctests remain covered by `just test` with all features. Preserve
  both feature-configuration checks; do not describe the current minimal-feature
  recipe as skipping rustdoc. Keep recipes and docs aligned with that
  admin-connection workflow when test database bootstrapping changes.
- Tool-version comparisons in `justfile` must stay portable across GNU and BSD userspace; do not rely on GNU-only `sort -V`.
- Rust CLI tools invoked by required recipes must be installed at an exact reviewed version with the published lockfile. Floating or unlocked `cargo install` commands are forbidden because registry resolution drift can make identical commits fail before their gates run. Required-recipe Cargo tool installs must use `scripts/ensure-exact-cargo-tool.sh` or a pinned prebuilt installer path so missing and mismatched versions are replaced through the bounded retry installer without weakening version, lockfile, or gate requirements. Documentation recipes pin mdBook `0.5.0`, mdbook-mermaid `0.17.0`, and Lychee `0.24.2`, and must propagate documentation build or link-check failures. The shared `trunk-install` recipe owns Trunk `0.21.14`; UI recipes must depend on it, and harness code must only consume the provisioned binary instead of installing Trunk ad hoc.
- `just cov` installs exact cargo-llvm-cov 0.8.7 through `scripts/ensure-exact-cargo-tool.sh`, records coverage once with `cargo llvm-cov --workspace --all-features --include-ffi --no-report`, retains native llvm-cov text, and then enforces the 90% per-package line threshold with `cargo llvm-cov report --package ...` against that shared workspace dataset. Keep the coverage gate workspace-sourced so library crates receive credit for lines exercised by downstream crates and integration tests; do not accept Cargo warning output or remove FFI instrumentation to make the run pass.
- The `revaer-torrent-libt` build script must select exactly one native source per build: bundle, paired explicit include/library override, pkg-config, or one coherent fallback prefix. It must propagate pkg-config defines, select Boost and OpenSSL headers from the same installation family ahead of ambient system headers, accept libtorrent `>=2.0.10,<2.2.0`, and verify that the selected macOS libtorrent library contains the Cargo target architecture before enabling the native backend. When pkg-config omits `TORRENT_ABI_VERSION`, query the configured C++ preprocessor for the effective packaged-header ABI and fail closed if it cannot be proven. Manual bundle, override, and fallback sources must provide their complete build definitions through `LIBTORRENT_DEFINES`, including `TORRENT_ABI_VERSION`; missing native ABI metadata fails closed.
- `just test-native` serializes native libtorrent tests with `--test-threads=1`; keep all native tests enabled and do not reintroduce overlapping native sessions without task-recorded evidence across the supported 2.0.x and 2.1.x API lines.

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
- Root-catalog file evidence must normalize native Unix mode widths losslessly and retain file-type as well as permission bits. Keep the exact trust mask, ownership checks and supported-target boundary unchanged when fixing platform-specific compilation or lint failures; permission-only evidence is not equivalent.
- ADR 557 root-readiness responses must remain path-free and validate coherent source/attestation reasons, a canonical positive decimal generation, and exactly five ordered bounded count rows. Transport validation is not database snapshot, filesystem, authentication, or destructive-readiness proof; do not activate the legacy workflow with these DTOs before the coordinated cutover.
- Target roughly 300-400 non-test LOC per production file. Split large files instead of silencing `too_many_lines`.
- `lib.rs` should stay limited to crate docs, module declarations, light re-exports, and tiny crate-boundary glue.
- `main.rs` must remain a thin bootstrap entry point.
- Name modules for what they own. No grab-bag files mixing domain logic, transport DTOs, adapters, and orchestration.

# Performance

- Measure before optimizing. Do not land “performance” refactors based on taste or folklore.
- For performance-driven changes, record the command, benchmark, trace, or timing report that justified the change in the task record or ADR.
- Prefer simpler, more explicit code unless measurement shows a real hotspot.
