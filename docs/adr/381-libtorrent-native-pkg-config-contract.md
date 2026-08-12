# Libtorrent native pkg-config contract

- Status: Superseded by ADR 410
- Date: 2026-08-11
- Context:
  - Commit `e9c192d6` established the first strict native-discovery contract while local validation could encounter multiple Homebrew prefixes and libtorrent versions.
  - The rebuilt head already contains the stronger ADR 410 implementation, including coherent source selection, deterministic pkg-config definitions, dependency-header and ABI validation, macOS architecture checks, and libtorrent 2.1 compatibility.
  - Replacing that implementation with the older 2.0-only code would discard reviewed repairs.
- Decision:
  - Preserve the current ADR 410 implementation as the canonical native contract.
  - Retain the ADR 381 guarantees that pkg-config definitions are forwarded deterministically, explicit include and library overrides are paired, native libraries match the Cargo target architecture, and Cargo reruns discovery when toolchain or pkg-config inputs change.
  - Preserve the later review repair that permits only one complete fallback prefix instead of mixing headers and libraries from multiple Homebrew roots.
  - Keep the current supported libtorrent range and ABI checks governed by ADR 410 rather than restoring the historical `<2.1.0` ceiling.
- Consequences:
  - Native discovery remains fail-closed without regressing the validated 2.1 runtime path.
  - This ADR records the reconstructed provenance; ADR 410 remains the source of truth for current behavior.
- Follow-up:
  - Keep focused build-script guardrails aligned with every native discovery input and source-selection path.

## Task Record

- Motivation:
  - Reconstruct the strict pkg-config and native-toolchain portion of `e9c192d6` on the current rebuilt head without importing unrelated media changes or downgrading later repairs.
- Design notes:
  - Verified that the current build script already includes the original deterministic define propagation, pkg-config environment invalidation, paired overrides, architecture validation, and fail-closed behavior.
  - Added regression evidence for `PATH`, generic pkg-config variables, and target-qualified pkg-config variables.
  - Updated the scoped Rust instruction so native discovery invalidation is a maintained contract.
- Test coverage summary:
  - `cargo test -p revaer-torrent-libt --test build_script_guardrails --all-features`
  - `cargo clippy -p revaer-torrent-libt --test build_script_guardrails --all-features -- -D warnings -W clippy::cargo -W clippy::nursery -A clippy::multiple_crate_versions -A clippy::redundant_pub_crate`
  - `just instruction-drift`
- Observability updates:
  - No runtime logs, metrics, traces, health checks, or events changed.
  - Build-time failures and Cargo rerun directives remain the native discovery diagnostics.
- Status-doc validation:
  - Updated `docs/adr/index.md` and `docs/SUMMARY.md`; no operator or product status document required a behavior change.
- Risk & rollback plan:
  - The added assertions intentionally fail when a discovery input is removed. Revert this commit if the test itself is incorrect; do not weaken the native source, ABI, or architecture contract.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/ffi.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - The missing discovery-invalidation requirement was added to the Rust instruction; no contradiction remains.
