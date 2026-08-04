# Libtorrent 2.1 Compatibility Foundation

- Status: Accepted
- Date: 2026-08-04
- Context:
  - The native torrent adapter supported libtorrent 2.0.x APIs, while current ARM Homebrew supplies libtorrent 2.1.0.
  - The existing build script could mix default Homebrew paths with pkg-config metadata and did not prove that selected macOS libraries matched the Cargo target architecture.
  - Upstream stack commits contained the needed repairs alongside unrelated media features, so this foundation extracts only native build and runtime compatibility.
- Decision:
  - Select one coherent native source in this precedence order: bundle, paired explicit include/library override, pkg-config, then one complete fallback prefix.
  - Propagate every pkg-config preprocessor define and re-run discovery when relevant pkg-config environment changes.
  - Accept libtorrent `>=2.0.10,<2.2.0` and validate the selected macOS libtorrent binary with `lipo` against the Cargo target architecture.
  - Keep compile-time C++ branches for both pre-2.1 and 2.1 APIs covering torrent loading, file layouts and strong indexes, torrent authoring, magnet generation, peer data, resume state, and metadata ownership.
  - Serialize the native test recipe because overlapping in-process libtorrent sessions have not been validated across both supported API lines.
- Consequences:
  - ARM Homebrew libtorrent 2.1 can compile and run the native adapter without deprecated API warnings, while supported 2.0 builds retain their existing API path.
  - Libtorrent 2.2 and unknown macOS architectures fail closed pending dedicated compatibility evidence.
- Follow-up:
  - Run the shared full gates when released.
  - Add a CI-native matrix with a real libtorrent 2.0.x installation so both compile-time branches execute remotely rather than relying on source guardrails for the unavailable local 2.0 path.

## Task Record

- Motivation:
  - Establish native libtorrent compatibility independently of the media-target PR stack.
- Design notes:
  - Extracted source selection and define propagation from `22d1f559` without its supply-chain/vendor changes.
  - Extracted pkg-config invalidation and macOS architecture validation from `56f0d7a9`, retaining a single coherent fallback and extending the upper boundary to libtorrent 2.2.
  - Extracted only `.github/instructions/rust.instructions.md`, `justfile`, `crates/revaer-torrent-libt/src/ffi/session.cpp`, and `crates/revaer-torrent-libt/src/session/native.rs` compatibility concepts or hunks from `38fbf569`; no media, API, schema, or target-model files were imported.
- Test coverage summary:
  - Added deterministic build-script tests for source coherence, pkg-config definitions, version bounds, architecture parsing, and both C++ API branches.
  - Ran warning-free native compile and test validation against installed ARM Homebrew libtorrent 2.1.0.
  - Ran a warning-free x86_64 C++ syntax compile against installed Intel Homebrew libtorrent 2.0.11 headers and verified that the ARM build rejects its x86_64 library before link.
  - The 2.0 Rust/native runtime suite cannot execute on this workstation because the x86_64 Rust target is not installed; runtime coverage for that line remains a CI matrix follow-up.
- Observability updates:
  - Build failures now identify missing native libraries, unsupported target architectures, failed `lipo` probes, and architecture mismatches before link or runtime.
- Status-doc validation:
  - Updated the ADR index and mdBook summary; no media status or operator-facing media documentation changed.
- Risk & rollback plan:
  - The remaining risk is unexecuted local 2.0 native coverage and behavioral variation in future 2.1 patch releases. Revert this commit to restore the 2.0-only native path if stack integration regresses.
- Dependency rationale:
  - No dependency was added; build guardrails use `std` and the crate's existing `tempfile` development dependency.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, `.github/instructions/ffi.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Updated Rust instructions for coherent native selection, supported versions, architecture validation, and serialized native tests. No contradictory policy was retained.
