# Strict Sonar analysis foundation implementation

- Status: Implemented task record; no new architecture decision
- Date: 2026-08-15
- Approval trace:
  - This implementation is bounded by the operator-approved ADRs 001, 002, 442,
    443, 444, and 446 through 452 as written on 2026-08-15, plus accepted ADR 482
    for tooling module boundaries and accepted ADR 488 for one scanner invocation.
  - ADRs 442 through 452 are supplied by the global stack and are intentionally
    referenced rather than recreated in this sparse prerequisite base.
  - This record closes implementation work only. It does not introduce or claim
    approval for another architectural choice.
- Context:
  - The prerequisite branch inherited broad Sonar exclusions, incomplete coverage,
    partial SCM behavior, duplicate scanner execution, missing required contexts,
    floating analysis tools, and workflows that could hang or conceal failed gates.
  - The live `main` inventory at the start of this work reported 0.0% coverage,
    1,344 lines to cover, 185,119 NCLOC, 126 unresolved issues (45 critical,
    39 major, and 42 minor; 91 in `session.cpp`), and six current hotspots.
    Those findings are remediation work, not accepted debt or suppression targets.
  - The UI/runtime asset cleanup is implemented separately by signed commit
    `f01380f90e2cac1eeb78e2ba0a8776a8052c516d`. Integration must place that layer
    before this foundation, preserve its ADR consolidation, and change the final
    exact source inventory from deleted `revaer-logo.png` to top-level
    `revaer-logo.svg`.
- Implemented boundary:
  - `sonar.sources` exactly enumerates tracked authored top-level entries, including
    `just`; `.sonar-test-scope/.gitkeep` is a committed empty sentinel; all source,
    test, coverage, duplication, issue, analyzer-default, and SCA filters are empty.
  - Hidden-file, text, YAML, JSON, JavaScript/TypeScript, Rust, native C-family, and
    SCA analysis remain enabled. PostgreSQL files are not misrouted to PL/SQL.
  - `sonar.filesize.limit=100` and `sonar.javascript.maxFileSize=100000` align the
    scanner-wide 100 MB limit with the JavaScript analyzer's KB property. Sonar's
    current documentation describes that property as configurable and documents no
    lower hard maximum.
  - `sonar.scm.forceReloadAll=true` is supported and loads blame for all files.
    Sonar documents the option as expensive and generally not permanent; the
    approved strict policy intentionally retains it, with bounded job timeouts and
    scanner `WARN` rejection exposing its operational cost instead of narrowing SCM.
  - SCA is enabled and fail closed. The documented
    `sonar.sca.sbomImportPaths` property is deliberately absent on this early tree
    because `release/media-compliance/media-runtime-inventory.spdx.json` does not
    exist yet. The structured property guard requires both together after the
    matching inventory layer is integrated.
  - Rust coverage installs exact cargo-llvm-cov 0.8.7, exercises the workspace with
    all features and `--include-ffi`, emits positive LCOV, and retains native
    llvm-cov text. JavaScript/TypeScript and authored shell/Ruby reports are merged
    into their Sonar inputs. The native build emits a compilation database and
    stages required CXX bridge headers.
  - Coverage toolchain compatibility is behavioral: CI uses pinned clang-19 and
    Rust-bundled llvm-cov/profdata. Hosted Linux fails unless the authored
    `crates/revaer-torrent-libt/src/ffi/session.cpp` section has a positive covered
    record. macOS does not require that record when it compiled the platform stub.
  - A local macOS Rust 1.91.0 / LLVM 21.1.2 / Homebrew clang-19 run previously
    passed Rust coverage with 163 sources, 55,774 `DA` records, and 52,052 hits but
    no `session.cpp`; the hosted Linux PR 71 artifact did contain positive native
    records. The section-aware verifier covers positive, absent, and zero-hit forms.
  - Both Sonar workflows use full-history exact-head checkout and reject a base SHA
    that is not the exact ancestor of the head. Stale or divergent stack branches
    fail with a restack requirement.
  - Setup installs SonarScanner CLI 8.1.0.6389 only. Every supported platform archive
    has an exact SHA-256 pin and is also verified with its detached signature against
    the committed SonarSource key and pinned fingerprint. Setup never scans.
  - Per ADR 488, `just sonar-scan` is the sole scanner invocation. It retains one
    complete log, rejects every ANSI-normalized `WARN` token, and admits exactly one
    safe task ID. The same task produces the retained report archive and API evidence.
  - Result verification waits for the exact task and quality gate, rejects ignored
    conditions, requires positive coverage and lines to cover, and permits zero
    unresolved issues and zero current hotspots only. PR calls use PR new-code
    semantics; main calls omit leak-period narrowing and inspect the complete issue
    and hotspot inventory. Hotspot status is never a filter.
  - ADR 482 is implemented as an import-only root `justfile` with seven exact domain
    modules and a five-owner structured Ruby workflow guard. Non-login `bash -c`
    preserves NVM selection, and `.nvmrc` plus the wrapper require Node 24.19.0.
  - Cargo analysis tools remain exact: cargo-udeps 0.1.57 on
    `nightly-2026-06-13`, audit 0.22.0, deny 0.18.9, llvm-cov 0.8.7, sqlx 0.8.6,
    and trunk 0.21.14. Install warning output, including yanked-lock warnings, fails.
  - The PR trigger has no narrowing. All 21 operator-supplied required contexts are
    statically derived and reachable on same-repository stack PRs. UI aggregation
    requires nonempty API and UI records from all three exact shard artifacts.
  - Image verification and any explicitly authorized image or Helm publication wait
    for UI, feature, native, media-conversion, coverage/Sonar, supply-chain, and
    matrix gates. Ordinary pull requests build and scan verification images without
    pushing or signing them. The exact amd64/arm64 analyses both run, Trivy high or
    critical findings return exit code 1 while SARIF remains uploadable, and build,
    scan, manifest, signing, and release operations run only through canonical
    `just` recipes.
  - Every direct checked-in job has a bounded timeout, setup steps have a 20-minute
    bound, required jobs and steps reject concealed failures, and external actions
    use exact commits; any `docker://` action must use an exact image digest.
- Consequences:
  - Scanner or analyzer warnings, missing reports, partial shards, stale SCM bases,
    absent required contexts, vulnerable images, and incomplete evidence now block.
  - Full SCM blame, complete coverage, two image analyses, and retained evidence add
    predictable CI cost. Bounded timeouts turn hangs into visible failures.
  - The strict foundation does not make the existing 126 issues and six hotspots
    disappear. Dedicated behavior-preserving cleanup layers must reduce them to zero
    before the final service can satisfy the complete main gate.
  - The 2026-08-15 local handoff also found repository-wide dependency failures that
    this nondependency layer is prohibited from absorbing: exact cargo-audit and
    cargo-deny reject current RustSec findings in `postgres-protocol`,
    `tokio-postgres`, `quinn-proto`, `anyhow`, `cxx`, and `event-listener`, plus the
    existing yanked `spin` lock entry. These findings remain mandatory fixes.
  - On this macOS host, Homebrew libtorrent 2.1.1 is incompatible with the unchanged
    `session.cpp` 2.0 API surface. Native `check`, `udeps`, and therefore full `just
    ci` stop there; the no-default-feature Rust lane passes, and hosted Linux with its
    pinned 2.0 package remains the required native proof.
- Follow-up:
  - Replay the asset commit first and switch the final source inventory to
    `revaer-logo.svg`; run `just check-assets` against
    `crates/revaer-ui/static/nexus` and retain the asset-owned UI instruction and ADR
    consolidation updates.
  - Add the SBOM import property in the same layer that introduces its exact matching
    SPDX inventory, then prove the retained scanner log contains no `WARN`.
  - Run one remote PR analysis to prove hosted Linux native coverage, all 21 required
    contexts, one scanner task, positive published coverage, zero warnings, and the
    strict issue/hotspot result. This local task does not mutate GitHub or Sonar.
  - Land separate behavior-preserving dependency and native compatibility repairs;
    do not suppress the audit findings or weaken native coverage to unblock this
    prerequisite.

## Task Record

- Motivation:
  - Establish a mergeable, reviewable strict-analysis prerequisite before behavior
    layers add further media transcoding surface.
- Design notes:
  - Implementation follows accepted ADR 482 and ADR 488 boundaries exactly. Parser
    checks use structured YAML, JSON, and Java-properties inputs; shell checks remain
    narrowly responsible for operational evidence.
- Test coverage summary:
  - Added fixtures for YAML duplicate keys and step names, literal and concealed
    `continue-on-error`, trigger narrowing, dependency graphs, all required contexts,
    exact UI shards, image matrix cardinality, direct gate commands, digest-pinned
    container actions, scanner checksums/signatures/traversal, ANSI warnings, task ID
    correlation, PR/main result scope, hotspot dispositions, SCM ancestry, native
    report sections, exact Cargo tools, NVM selection, image scans, and shard records.
  - The complete focused policy suite, workflow/Sonar recipes, shell and Ruby syntax,
    formatting, instruction drift, exact scanner installation, exact Node selection,
    npm install/audit, TypeScript compilation, generated API client, JavaScript
    coverage, media-conversion fixture, and no-default-feature FFI tests pass locally.
    Full `just ci` and `just ui-e2e` were not launched after their prerequisite lanes
    failed or required the explicitly prohibited shared-port services. Remote scanner
    and hosted-Linux native proofs remain explicit integration checks.
- Observability updates:
  - Retains the scanner log, SCM evidence, one task record, submitted scanner report,
    result API JSON, Rust/native/JavaScript/script coverage, compilation database,
    CXX headers, and cargo-udeps toolchain evidence.
- Status-doc validation:
  - Updated ADR 100's invocation supersession, accepted ADR 488's implementation
    pointer, this ADR index, and the documentation summary. Product capability and
    release claims are unchanged.
- Risk & rollback plan:
  - Revert this prerequisite as one layer if strict mechanics regress. Do not roll
    back by weakening scanner criteria, neutralizing findings, restoring duplicate
    scans, or bypassing required contexts. Stop with exact evidence if the official
    scanner rejects an approved property.
- Dependency rationale:
  - Adds no runtime dependency. CI uses exact SonarScanner CLI 8.1.0.6389, setup-trivy
    v0.3.1 with Trivy v0.69.3, cargo-llvm-cov 0.8.7, c8 12.0.0, TypeScript 5.9.3,
    and `@types/node` 26.1.1. Versions are exact because these tools create merge and
    release evidence; signature and checksum verification protect scanner provenance.
- Stale-policy check:
  - Reviewed `AGENTS.md`, Rust, FFI, DevOps, Sonar, and UI scoped instructions; root
    `justfile`, all seven modules, workflows, setup action, scanner properties, and
    instruction-drift mappings.
  - Removed stale PostgreSQL-to-PL/SQL mapping, authored test/generated exclusions,
    decoration-only quality-gate guidance, tolerated backlog hotspots, floating
    Cargo/nightly tools, login-shell Node behavior, permissive shard uploads, direct
    workflow release/security commands, and the DevOps prohibition on blocking
    scanner-side quality-gate waiting.
  - Added UI-scoped non-asset rules for exact NVM selection and fail-closed three-shard
    E2E coverage. Asset-specific UI instruction updates remain owned by the separately
    integrated asset layer; this prerequisite does not edit that worker's asset scope.
