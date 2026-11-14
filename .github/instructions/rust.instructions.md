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

`AGENTS.md` is the root contract. This file specializes Rust implementation for the paths in `applyTo`; root policy defines all acceptance gates, coverage targets and required deliverables.

# Rust Quality Rules

- Test workspace helpers use the common repo-local `.test-tmp/` root. Temporary file builders inherit the private `TMPDIR` provided by `rv`; trusted catalog tests retain their ancestry and owner checks.
- Native admission uses descriptor/mount identity, ownership and real capability probes for private directories. Filesystem names are identity metadata, not an admission allowlist; no different device from `/` is required.

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

- Trusted root catalogs retain a bounded content digest across load and retained source revalidation; equal-size writes must not depend on timestamp resolution.

- Keep workspace lint posture aligned with `AGENTS.md`, `rv lint` and crate-root attributes.
- Keep `.secignore` and the `[advisories].ignore` list in `deny.toml` empty. An exception requires exact operator consent, an expiry, and a matching guardrail change; an agent-authored ADR is not consent.
- `rv lint` composes source, workflow, dependency and compiler policy through `tools/src/revaer_tooling/tasks/policy.py`; update the owning check when its contract changes.
- Source policy preserves the stored-procedure boundary and the narrowly scoped disposable database provisioning exception. Use `rust_findings` in `tasks/policy.py` for the exact paths and checks; do not maintain a second shell implementation as an enforcement authority.
- Preserve both compiler passes in `rv lint`: all-target validation and production-target panic restrictions. The latter must not apply production panic rules to test targets.
- Keep operator-approved compiler configuration in the canonical `rv` Cargo adapter and manifests, not authored source suppressions. Preserve the owning lint groups and approved exceptions.
- `#[allow(...)]` and `#[expect(...)]` are not permitted in authored code, except for the exact generated CXX documentation exception in `ffi.instructions.md`. Split or redesign authored code instead.
- Vendor analysis and patching follow root policy. Preserve first-party wrappers and dependency security audits; report upstream dependency findings without taking ownership of vendor-source refactors.
- If custom cfgs are introduced, register them with `cargo::rustc-check-cfg` in `build.rs` or the manifest lint configuration. Do not silence `unexpected_cfgs`.
- Prefer `#[must_use]` for important return values and `pub(crate)` for internal APIs.
- FFI crates may omit a crate-wide `forbid(unsafe_code)` if necessary, but unsafe code must stay isolated to the documented boundary modules and shims. Do not use lint suppressions to permit unsafe.

# CI And Recipe Maintenance

- The existing reqwest client enables its dependency-free SOCKS feature for configured SOCKS routes. Proxy authentication remains in private client configuration; never fall back to direct routing when a configured proxy is unavailable.

- Manual Torznab execution uses `roxmltree` for standards-compliant XML parsing instead of an authored parser. Keep DTD resolution disabled and bound response bytes, XML nodes and release counts; never format credential-bearing upstream errors or URLs. Bootstrap constructs and injects HTTP clients, and shutdown awaits owned database writes.

- ZIP extraction uses the non-yanked 9.x release with only its flate2 adapter, sharing the existing Rust deflate backend. Align fsops' Unix dependency with ZIP's nix version instead of introducing a duplicate-crate exception; retain archive traversal, content and permission regression coverage. Restore ZIP directory modes after extracting their children, deepest first; recorded modes must not depend on the caller's process umask.

- Source inspection retains the bounded ffprobe frame sample and attributes frame side data only to its reported stream index; unindexed frames are ignored. Attachments use their stable kind marker when ffprobe omits or varies the codec name; other missing codecs still fail graph or sidecar validation.

- The application's default `compat-qb` feature forwards the API compatibility feature explicitly. Disable API dependency defaults so the application's no-default-feature gate exercises the actual compatibility-free configuration.

- Native discovery must invalidate Cargo when `PATH`, pkg-config search variables, or target-qualified pkg-config variables change.

- Keep the workspace MSRV aligned with the pinned Rust toolchain. Release build dependencies retain symbols (`strip = "none"`) for the pinned Rust 1.96 LLVM loader workaround; this setting applies only to build dependencies.
- Cardigann import uses `serde_norway` to retain its Serde YAML data model without the archived `unsafe-libyaml` dependency. Preserve valid import normalization and malformed-input rejection when updating the parser.

- Use the canonical `rv` database and test tasks. Preserve injected credential-safe database URLs, explicit managed-service provenance, isolated data directories, bounded readiness/recovery handling and failure propagation. Use `tasks/database.py` and the test-task registry for exact behavior; never copy password-bearing URLs or legacy shell invocation bodies into instructions.
- Canonical setup installs Rust CLI tools from the exact reviewed pins and published lockfiles in `tools/versions.toml`. Tasks consume provisioned executables; never install floating or unlocked Cargo tools ad hoc.
- `rv cov` measures the workspace with all features and applies the existing per-package threshold to the targets designated in Section 3 of `AGENTS.md`. Keep measurements workspace-sourced so libraries receive credit for execution by downstream crates and integration tests. Test-only support and examples have no independent coverage targets; use the coverage tool's supported classification, including its documented limitations for inline test code. Preserve collected records and required production/native measurements; do not add tests merely to execute test scaffolding.
- The per-package gate delegates its 90% Rust line threshold to `cargo-llvm-cov report --fail-under-lines` using supported source exclusions. Do not parse coverage counters or implement another percentage gate. Keep complete diagnostic reports separate from that scoped threshold report.
- The dev-dependency-only `revaer-test-support` fixture is an internal utility: under the approved production-only coverage requirement, retain its full validated measurements without applying the production percentage threshold.
- The `revaer-torrent-libt` build script must select exactly one native source per build: bundle, paired explicit include/library override, pkg-config, or one coherent fallback prefix. It must propagate pkg-config defines, select Boost and OpenSSL headers from the same installation family ahead of ambient system headers, accept libtorrent `>=2.0.10,<2.2.0`, and verify that the selected macOS libtorrent library contains the Cargo target architecture before enabling the native backend. When pkg-config omits `TORRENT_ABI_VERSION`, query the configured C++ preprocessor for the effective packaged-header ABI and fail closed if it cannot be proven. Manual bundle, override, and fallback sources must provide their complete build definitions through `LIBTORRENT_DEFINES`, including `TORRENT_ABI_VERSION`; missing native ABI metadata fails closed.
- Native recovery nextest invocations append `-Dwarnings` to the effective plain or encoded compiler flags; Cargo config alone must not permit warnings through the external subcommand. Preserve native test serialization in `rv test-native` and the nextest resource configuration; do not introduce overlapping native sessions outside their supported contracts.
- Preserve the production Rust threshold and complete native measurements defined by root policy. Install matching `rust-src` and retain LLVM source-path equivalence; never hide missing-source failures or authored native records.

# Documentation

- The media fixture catalog retains all 30 declared fixtures and their expected probe snapshots so runtime integration tests use the complete qualified inputs.

- Public crates need crate-level rustdoc that explains purpose, invariants, and a realistic usage example.
- Externally consumed public items should document:
  - behavior
  - invariants and assumptions
  - error cases
  - panic behavior, if any exists
  - copy-pasteable examples when the item is meant to be used directly
- Prefer examples that use `?` and explicit error handling over `unwrap()`/`expect()`.

# Maintainability And Layout

- Public Torznab handlers reuse the manual-search dispatcher. Their operator-approved five-second wait observes sealed pages or terminal state without retaining a database transaction or owning an attempt. Deadline expiry returns available sealed results and leaves unfinished work running; external offset/limit slices append order independently of internal page size.

- Media execution retains checkpoint replay, verified replacement, cooperative shutdown and protected scratch cleanup. Filesystem capacity probing is shared by media admission and filesystem post-processing.

- The application media facade and discovery runtime receive the existing media store, native capability detector, catalog source and watcher. `notify` provides the native watcher, `rustix` retains descriptor identity, `sha2` computes aggregate identities, and `yaml-rust2` validates portable configuration before persistence.

- `revaer-media-runtime` owns injected native process, capability and inspection adapters. Its restored `rustix` dependency provides safe Unix descriptor and process operations without authored unsafe code.
- Root catalogs retain descriptor, mount, source identity and root-lock checks. Their restored `sha2` dependency computes canonical identities; `tempfile` isolates functional filesystem regression fixtures.
- Command execution and managed scratch workspaces retain cooperative cancellation, live resource limits, capacity admission and terminal cleanup policy.
- Job planning, output verification and replacement retain source-identity and recovery checks. The `media_fixtures` integration target executes real conversions and verifies their resulting media graphs.
- `rv test-media-recovery` exercises the production worker H.264 replacement regression with real FFmpeg/FFprobe and all features. Its synthetic catalog is distinct from the Linux mounted-service test.
- `rv cov` retains matching all-feature instrumentation across the production worker and mounted-recovery collections without clearing earlier profiles. Require one passing production replacement test and all nine selected mounted-recovery scenarios; empty selections are failures. Use `tasks/coverage.py` for the exact task sequence.

- `revaer-media-core` owns deterministic media graph compilation, planning, compliance and verification. Keep filesystem, process and database access in injected runtime adapters.

- Keep files single-purpose and cohesive.
- Target roughly 300-400 non-test LOC per production file. Split large files instead of silencing `too_many_lines`.
- `lib.rs` should stay limited to crate docs, module declarations, light re-exports, and tiny crate-boundary glue.
- `main.rs` must remain a thin bootstrap entry point.
- Name modules for what they own. No grab-bag files mixing domain logic, transport DTOs, adapters, and orchestration.

# Performance

- Measure before optimizing. Do not land “performance” refactors based on taste or folklore.
- For performance-driven changes, measure the actual behavior and report the relevant command and result briefly in chat. Root artifact policy applies: no routine task record, ADR or saved proof file is required.
- Prefer simpler, more explicit code unless measurement shows a real hotspot.

- `rv test-runtime-shutdown` and `rv lint-runtime-shutdown` retain both all-feature and no-default-feature configurations. Each test selection must execute positive passing tests; preserve both existing Clippy passes.

- The reusable database initializer has its own success-path integration test in the foundation. It verifies exact initializer digest sealing, restricted runtime identity, owner login disablement, repeated-init rejection and owned database/role cleanup; do not rely on downstream media consumers for coverage.

- Coverage executes independent tests in nextest processes. Keep native libtorrent tests in the serial resource group, database-heavy tests in the bounded group, and the nine recovery scenarios independently selectable. Private test state must not become a shared fixture merely to reduce execution time.

- Media directory observations compare bounded raw-name hashes alongside metadata; resumable scans must detect name changes even when filesystem timestamps collide.

- Scheduled RSS dispatches Torznab XML and public Cardigann HTML explicitly under the existing deployment job loop, shares bounded transport and private connection resolution with search, and awaits claim settlement during cooperative shutdown. Cardigann recent listings reuse persisted static GET rules with empty keywords; never fabricate manual-search runs or claims. RSS identifiers remain separate from canonical search ingestion and acquisition. Its real database executor regressions use the existing bounded database test group.

- Public Cardigann HTML execution uses `scraper` for HTML5 tree construction and CSS selection; do not replace it with a second XML dialect or a handwritten HTML parser. The operator-approved MPL-2.0 allowances are limited to the four CSS dependencies named in `deny.toml`.

- Media fixture inspectors receive the actual private workspace root. Compare path ancestry against that root; checkout parent names such as `workspace` must not classify source files as conversion outputs.
