---
applyTo:
  - "crates/revaer-torrent-libt/**"
---

`AGENTS.md` and `rust.instructions.md` apply first. This file specializes the FFI boundary.

# FFI Boundary Rules

- Unsafe code is confined to the FFI boundary modules, build scripts, and native shims only.
- `rv lint` mechanically enforces that authored `unsafe` stays inside `crates/revaer-torrent-libt/src/ffi.rs` and `crates/revaer-torrent-libt/src/ffi/**`.
- The public surface exposed to the rest of the workspace must be safe Rust wrappers and translated domain types.
- Document safety invariants and failure translation at the boundary.
- Runtime configuration and global/per-torrent limit acknowledgements must carry
  the completed worker handler result, including alternate-speed reconciliation,
  not enqueue
  success. Preserve the original typed error and worker-origin warning/health
  reporting even when the caller drops its reply receiver. Receiver loss is not
  cancellation of admitted work or proof of rollback; a closed reply channel is
  an unobserved completion, not success. Preserve target identity and native
  error sources for per-torrent as well as global limits. Successful replies
  precede the independent event flush; handler failure retains the existing
  skipped flush and cannot immediately clear degraded health. These ADR 588 S2
  acknowledgements alone do not establish native atomicity, shutdown
  admission/settlement or revision-wide publication atomicity.

# `catch_unwind`

- `catch_unwind` is permitted only at the explicit FFI boundary where it prevents a Rust unwind from crossing a foreign ABI boundary.
- Allowed uses must satisfy all of the following:
  - the call site is immediately adjacent to the ABI boundary
  - the reason is documented in code comments or module docs
  - the panic is translated into a deterministic error or boundary-safe failure contract
  - the path is covered by tests
- `catch_unwind` is never acceptable as ordinary control flow, generic recovery, or a substitute for explicit error handling in normal Rust code.

# Native Shim Rules

- The pinned `cxxbridge-macro` is patched locally to preserve each documentation
  fragment’s source span. Rust 1.96 Clippy otherwise discards the generated
  documentation and rejects correctly documented fallible bridge methods. Keep
  the error sections and lint rules intact; the patch changes no native ABI.

- Keep C++ exception translation narrow and explicit where possible.
- Native operation replies catch expected libtorrent system errors or torrent
  authoring errors. Their CXX `Result` declarations translate unexpected foreign
  exceptions into the safe session wrapper's existing typed failure contract.
- Avoid blanket `catch (...)` handlers unless the ABI or toolchain truly requires one and the reason is documented and tested.
- Do not leak foreign exceptions or panic behavior into the rest of the Rust workspace.
- Native build changes must preserve the Sonar compilation database and staged CXX bridge headers. Hosted Linux coverage must retain a `crates/revaer-torrent-libt/src/ffi/session.cpp` llvm-cov section with at least one positive covered-line record; local macOS may omit that assertion only when the real native backend was not compiled.

- When an owned persistent Linux mount is supplied through `REVAER_NATIVE_RECOVERY_ROOT`, coverage must also run the existing native service-recovery scenario with the same instrumentation and retain the workspace measurements. This opt-in execution does not lower the per-crate floor or replace ordinary tests.

- Feed native coverage to the CFamily LLVM importer and Rust LCOV to the Rust importer. Keep complete native line/branch counts and source scope; native LCOV dash counters must not reach the Rust parser.
- Build-helper tests include the canonical manifest-root source path so LLVM merges their measurements with the build script instead of producing a duplicate `tests/../` source record. Rust LCOV omits compiler and registry files outside the analyzed checkout.
