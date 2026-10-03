# Tooling migration inventory

Initial foundation: `54641c63`. Shared prerequisite baseline: `ec8adc14`.
Current disposable media integration starts from `f4b80bf7` plus its recorded dirty-tree snapshot (2026-09-21).
The operator paused media work and authorized merge/restack on 2026-10-02. The
pushed operator checkpoint is `71596802`; the older disposable validation
snapshot above remains historical evidence, not current acceptance.

## Status and acceptance

The uv bootstrap, installed launcher, typed task dispatch, process handling, and
Python quality gates are implemented. Existing Just, shell, Node, and workflow
entry points remain until their replacements have passed the required comparisons.

A fixture result proves the stated wrapper or behavior; it does not claim that
the application CI or browser suite has passed. Final acceptance still requires
`rv ci` and `rv ui-e2e` on this foundation and a disposable media integration.

### Current merge preparation (2026-10-02)

All PR and Sonar workflow gates now invoke the shared uv-managed CLI. The full
policy check and GitHub Actions syntax checks pass. The required-check snapshot
is preserved from the media stack. A targeted urllib3 2.8.0 update passed the
current advisory audit. Container qualification exposed replaced Alpine OpenSSL
packages; their exact pins were updated to the available 3.5.9-r0 release. Full
CI, E2E and hosted analysis remain required before merge. Recovery refs and a
dependency-ordered plan preserve all 104 media PR heads plus the operator tip.

## Integration dependencies

- The tooling branch now follows the existing shared prerequisite commits through
  `ec8adc14`, including advisory-clean dependencies and libtorrent 2.1 ABI support.
  Those commits retain their original history. Proofs below explicitly described
  as the initial foundation refer to `54641c63`; complete acceptance must be rerun
  on the new baseline. The active media checkouts remain untouched.
- Media adds split Just recipes, database rebaseline proofs, native helpers, runtime
  E2E coverage, compliance checks, and stricter Sonar evidence requirements.
- A later 2026-09-15 refresh found additional uncommitted media association/root
  readiness UI scenarios and database ingestion closure proofs. Their active
  checkout remains untouched; include those changes in the next integration
  inventory refresh before selecting a disposable validation snapshot.
- These additions must receive task mappings and parity checks before integration
  is complete. Do not call the simpler foundation policy checker equivalent to them.
- Rust quality-gate pins and the native ABI inputs must come from the selected
  checkout. Never overwrite the active stack or silently downgrade its compiler.

## Recipe mapping

Rows marked Pending have no replacement yet. Media rows require comparison
against their newer implementation even where a foundation command already exists.

| Recipe | Source | Python status |
| --- | --- | --- |
| `fmt` | Foundation: `justfile` | Verified on foundation |
| `fmt-fix` | Foundation: `justfile` | Verified with real tools in fixture |
| `policy` | Foundation: `justfile` | Foundation verified; media guardrails pending |
| `instruction-drift` | Foundation: `justfile` | Foundation verified; media mappings pending |
| `lint` | Foundation: `justfile` | Full foundation policy, Python lint/types, and both Clippy passes succeeded on Linux |
| `check` | Foundation: `justfile` | Passed on Linux with supported libtorrent 2.0.10 |
| `test` | Foundation: `justfile` | Passed full foundation suite in the supported Linux environment |
| `test-native` | Foundation: `justfile` | Passed foundation suite in the supported Linux environment |
| `test-features-min` | Foundation: `justfile` | Passed foundation suite in the supported Linux environment |
| `build` | Foundation: `justfile` | Verified with real Cargo in fixture |
| `build-release` | Foundation: `justfile` | Verified with real Cargo in fixture |
| `release-artifacts` | Foundation: `justfile` | Verified source manifest and configured Cargo target in fixture |
| `udeps` | Foundation: `justfile` | Pinned compiler and unused-dependency detection verified; repository gate pending |
| `sqlx-install` | Foundation: `justfile` | Installation moved to setup; real SQLx database tasks verified |
| `db-migrate` | Foundation: `justfile` | SQLx/PostgreSQL replay, checksum rejection and rollback verified |
| `audit` | Foundation: `justfile` | Rust and locked Python audits passed on the shared prerequisite baseline with four targeted lockfile updates |
| `deny` | Foundation: `justfile` | Advisories, bans, licenses, and sources passed on the shared prerequisite baseline |
| `cov` | Foundation: `justfile` | Full foundation collection and every Rust crate gate passed; complete native LCOV/text/HTML retained |
| `sonar-compile-db` | Foundation: `justfile` | Full supported Linux native build and compilation database passed; repeated generation/failure checks verified |
| `sbom` | Foundation: `justfile` | Verified with real Cargo in fixture |
| `licenses` | Foundation: `justfile` | Verified with real cargo-deny in fixture |
| `api-export` | Foundation: `justfile` | Verified with real Cargo in fixture |
| `helm-lint` | Foundation: `justfile` | Verified on foundation |
| `helm-package` | Foundation: `justfile` | Signed and unsigned packaging verified |
| `helm-publish` | Foundation: `justfile` | Published to a local TLS/authenticated registry; retrieved signed chart and metadata match packaged bytes |
| `release-dev` | Foundation: `justfile` | Mapped to release publish; local service and post-merge/tag workflow contracts verified; hosted acceptance pending |
| `release-lock` | Foundation: `justfile` | Real uv resolver, unchanged graph, and invalid-manifest failure verified |
| `validate` | Foundation: `justfile` | Ordered gates implemented; full repository acceptance pending |
| `ci` | Foundation: `justfile` | Validation plus release build implemented; full repository acceptance pending |
| `docker-build` | Foundation: `justfile` | Real single-platform load and multi-platform OCI export verified; full application image acceptance pending |
| `docker-scan` | Foundation: `justfile` | Real Trivy finding and report retention verified; full application image acceptance pending |
| `sync-assets` | Foundation: `justfile` | Verified with real Cargo in fixture |
| `check-assets` | Foundation: `justfile` | Verified clean, staged, changed and new assets |
| `ui-serve` | Foundation: `justfile` | Implemented; lifecycle validation pending |
| `ui-build` | Foundation: `justfile` | Built in the supported Linux application E2E run |
| `ui-e2e` | Foundation: `justfile` | Passed 35 anonymous + 35 authenticated API checks and 13 Chromium scenarios |
| `ui-e2e-coverage` | Foundation: `justfile` | Complete foundation API operation and configured UI route evidence passed |
| `runbook` | Foundation: `justfile` | Full application run and archive passed; failure/interrupt/archive-lock behavior verified |
| `zombies` | Foundation: `justfile` | Implemented; native Cargo/Trunk, 87 focused checks, the complete 1137-test suite and 32 Linux process/watch checks passed; full application acceptance pending |
| `dev` | Foundation: `justfile` | Implemented; native Cargo/Trunk, 87 focused checks, the complete 1137-test suite and 32 Linux process/watch checks passed; full application acceptance pending |
| `docs-install` | Foundation: `justfile` | Installation moved to setup |
| `docs-build` | Foundation: `justfile` | Real foundation build passed without warnings |
| `docs-serve` | Foundation: `justfile` | Implemented; lifecycle validation pending |
| `docs-index` | Foundation: `justfile` | Real foundation index passed with 355 entries |
| `docs-link-check` | Foundation: `justfile` | Implemented; validation pending |
| `docs` | Foundation: `justfile` | Verified with real tools in fixture |
| `db-start` | Foundation: `justfile` | Ownership lifecycle and application E2E composition verified |
| `db-reset` | Foundation: `justfile` | Explicit managed reset verified; ownership checks protect other databases |
| `db-seed` | Foundation: `justfile` | Idempotent transactional seed verified against real PostgreSQL |
| `sqlx-install` | Media: `just/database.just` | Foundation implementation exists; media comparison pending |
| `db-migrate` | Media: `just/database.just` | Application default aligned; SQLx migration semantics verified |
| `db-rebaseline-freeze` | Media: `just/database.just` | Verified exact 167-file frozen corpus; current feature-development phase accepted without rewriting historical pins; mutation/phase/path failures covered |
| `db-rebaseline-candidate` | Media: `just/database.just` | Native SQLx/PostgreSQL replay reproduced candidate, statement map and evidence byte for byte; owned container/volume removal verified |
| `db-init-prefix-check` | Media: `just/database.just` | All 532 routine classifications and reviewed final initializer matched; prefix and forbidden-control failure checks covered |
| `test-database-baseline-read` | Media: `just/database.just` | Native Rust fixture verifies workspace/all-feature baseline selection and explicit database propagation; actual media acceptance pending |
| `db-pristine-catalog-generate` | Media: `just/database.just` | Native Python generation reproduced the complete 8,456-line reviewed snapshot byte for byte |
| `db-pristine-catalog-validate` | Media: `just/database.just` | Exact reference comparison, retained mismatch evidence and failed-read cleanup verified against the pinned server |
| `db-pristine-catalog-test` | Media: `just/database.just` | All 27 catalog mutation cases, identity/TOAST/statistics normalization, security changes and malformed evidence checks passed |
| `db-init-finalize` | Media: `just/database.just` | Regenerated reviewed initializer bytes exactly; unique marker, exact delta and unchanged-pin checks covered |
| `db-init-final-proof` | Media: `just/database.just`; accepted ADR 591 | Historical parity acceptance superseded for v0; do not complete the obsolete D3 proof project. Reuse baseline, extension and reset checks for required fresh-init/security/recovery qualification. Historical components are not a completed public command; retirement and final workflow integration remain pending. |
| `db-init-pool-probe` | Media: `just/database.just` | Native Cargo fixture verifies exact library selector, input propagation, failure/no-match rejection and shown output; full database producer pending |
| `db-init-cancellation-probe` | Media: `just/database.just` | Native Cargo fixture verifies exact library selector, input propagation, failure/no-match rejection and shown output; full database producer pending |
| `stack-changed-lines` | Media: `just/database.just` | Exact complete text accounting and all 213 original binary blobs verified with real Git; identity/history/expiry/unpublished-head failures covered; provider acceptance is synthetic |
| `db-init-assembly-changed-lines` | Media: `just/database.just` | Exact reviewed text limit and unconditional binary rejection passed native/failure tests |
| `db-start` | Media: `just/database.just` | Explicit ownership/overrides supported; media application integration pending |
| `db-reset` | Media: `just/database.just` | Explicit managed reset supported; administrative databases protected |
| `db-seed` | Media: `just/database.just` | Selected connection and transactional seed verified; media application integration pending |
| `docs-install` | Media: `just/docs.just` | Restored pinned Cargo installation and native Mermaid integration; native fixture verified |
| `docs-build` | Media: `just/docs.just` | Compared media recipe; exact pins and native book output verified; full media book acceptance pending |
| `docs-serve` | Media: `just/docs.just` | Compared media recipe; native adapter preserves serve --open; interactive media acceptance pending |
| `docs-index` | Media: `just/docs.just` | Compared media recipe; release-mode native indexer fixture verified; current media index acceptance pending |
| `docs-link-check` | Media: `just/docs.just` | Restored pinned Lychee installation; native broken/resolved local link checks passed |
| `docs` | Media: `just/docs.just` | Restored install/build/index composition; native fixture passed; current media book acceptance pending |
| `docker-build` | Media: `just/images.just` | Compared one-platform load and multi-platform OCI export; existing native Buildx tests cover bytes and builder selection; full application images pending |
| `docker-scan` | Media: `just/images.just` | Compared HIGH/CRITICAL failing scan against default revaer:ci; existing native Trivy tests retain findings and reject missing images; application image scan pending |
| `image-build-push` | Media: `just/images.just` | Typed Buildx coordination and digest mismatch fixtures pass; actual publication pending |
| `image-build-verify` | Media: `just/images.just` | Native Buildx fixture verifies source/version/architecture labels; full application image pending |
| `image-inventory` | Media: `just/images.just` | Explicit source/platform and output contracts verified; actual published image inventory pending |
| `image-scan` | Media: `just/images.just` | Real Trivy SARIF findings retained and evaluated by the separate gate |
| `image-sign-attest` | Media: `just/images.just` | Complete bundle validation precedes injected signing; keyless publication pending |
| `image-attestation-verify` | Media: `just/images.just` | Exact identity/issuer/predicate and retained output fixtures pass; native keyless verification pending |
| `image-manifest-create` | Media: `just/images.just` | Immutable source resolution, output digest agreement and failure fixtures pass |
| `image-manifest-verify` | Media: `just/images.just` | Verification-only inputs and workflow ordering checked; no remote image result claimed |
| `image-manifest-sign` | Media: `just/images.just` | Common immutable digest selection and injected signature failure checks pass |
| `test-fixture-scripts` | Media: `just/media.just` | Python source/probe/generation/task checks pass, including native tools and 70 F1 cases |
| `download-test-fixtures` | Media: `just/media.just` | Actual CLI acquired and hash/size-verified all 22 locked media sources |
| `generate-test-fixtures` | Media: `just/media.just` | Actual FFmpeg generated all eight declared media derivatives; foundation subtitle fix preserved |
| `verify-test-fixtures` | Media: `just/media.just` | Actual CLI matched all 30 snapshots; real bounded/F1 emissions retained under the exact existing contracts |
| `test-media-conversion` | Media: `just/media.just` | Actual media command passes all six tests including prepared fixtures: 30 pipeline actions, 8 video and 6 audio transcodes, zero failures; locked source and reviewed snapshot checks pass |
| `test-media-root-catalog` | Media: `just/media.just` | Actual disposable media integration passed 80 selected tests through `rv`; logs retained in `artifacts/media-native-contracts/`. Linux-only cases still require Linux qualification. |
| `test-media-root-contract` | Media: `just/media.just` | Actual disposable media integration passed 95 selected tests through `rv`; logs retained in `artifacts/media-native-contracts/`. Linux-only cases still require Linux qualification. |
| `test-media-broker-codec` | Media: `just/media.just` | Actual disposable media integration passed 20 selected tests through `rv`; logs retained in `artifacts/media-native-contracts/`. Linux-only cases still require Linux qualification. |
| `update-test-fixture-probes` | Media: `just/media.just` | Whole-set validation precedes ordinary updates; tests prove F1 is never replaced |
| `clean-test-fixtures` | Media: `just/media.just` | Tracked/link guards and owned directory cleanup verified |
| `clean-test-media` | Media: `just/media.just` | Cleanup restricted to checkout-owned conversion temporary files; unrelated global temporary data preserved |
| `fmt` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `fmt-fix` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `policy` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `workflow-guardrails-test` | Media: `just/quality.just` | Structured policy cases and shared task wiring passed; workflow cutover pending |
| `trivy-sarif-verify` | Media: `just/quality.just` | Python validator passes original fixtures; workflow conversion pending |
| `trivy-sarif-policy-test` | Media: `just/quality.just` | Cases included in tooling tests; media acceptance pending |
| `image-compliance-generate` | Media: `just/quality.just` | Native legacy comparison matched all five artifact contents; actual image pending |
| `image-compliance-validate` | Media: `just/quality.just` | v2 checks and failure fixtures pass; actual image pending |
| `image-compliance-test` | Media: `just/quality.just` | Bundle mutations and native parity verified; media acceptance pending |
| `media-compliance-guardrails-test` | Media: `just/quality.just` | Exact pins, declarations, source evidence, labels and nonfree guard checked; integrated into `rv policy` |
| `stack-check-contract-test` | Media: `just/quality.just` | Replaced by tooling-check workflow mutations; conversion/upload/cleanup/build dependencies preserved; real workflow cutover pending |
| `supply-chain-results-test` | Media: `just/quality.just` | Included in tooling tests; caller conversion pending |
| `advisory-exception-guardrails-test` | Media: `just/quality.just` | Replaced by tooling-check advisory mutations at policy/audit/deny entry points |
| `sonar-script-tests` | Media: `just/quality.just` | Python scanner/input/SCM/installer/result cases and measured bootstrap passed; full producer integration pending |
| `instruction-drift` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `lint` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `check` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `test-runtime-shutdown` | Media: `just/quality.just` | Implemented with both feature modes; real Cargo/Clippy fixture validation; application integration pending |
| `lint-runtime-shutdown` | Media: `just/quality.just` | Implemented with both feature modes; real Cargo/Clippy fixture validation; application integration pending |
| `test` | Media: `just/quality.just` | Actual complete macOS `rv test` passed with `RUST_TEST_THREADS=1` and owned database cleanup; Linux-only and separately ignored conversion qualification remain pending |
| `test-native` | Media: `just/quality.just` | Compared media all-feature, single-threaded libtorrent selection and native flag; real Cargo fixture passed; actual native application suite pending |
| `test-features-min` | Media: `just/quality.just` | Compared both API/app no-default-feature invocations and database propagation; real Cargo fixture passed; application acceptance pending |
| `udeps` | Media: `just/quality.just` | Foundation implementation exists; media comparison pending |
| `audit` | Media: `just/quality.just` | Actual Rust and locked all-group Python dependency audits pass in both checkouts |
| `deny` | Media: `just/quality.just` | Actual advisories, licenses, bans and sources pass in both checkouts |
| `cov` | Media: `just/quality.just` | Native instrumentation and tool evidence implemented; integrated gate pending |
| `cov-report` | Media: `just/quality.just` | Complete native/Rust outputs verified; Sonar integration pending |
| `script-coverage` | Media: `just/quality.just` | Real macOS/Linux kcov and official uv cases passed; at most one uncovered bootstrap line approved; workflow integration pending |
| `sonar-compile-db` | Media: `just/quality.just` | Shared-baseline native build passed; generated headers survive task completion |
| `sonar-verify-inputs` | Media: `just/quality.just` | Complete input and failure fixtures passed; full producer integration pending |
| `sonar-prepare-sources` | Media: `just/quality.just` | Tracked-file protection, generated cleanup, and browser evidence retention passed |
| `sonar-prepare-scm` | Media: `just/quality.just` | Real shallow clones, exact event ancestry, dirty work preservation, and stale-evidence rejection passed |
| `sonar-scan` | Media: `just/quality.just` | Process/warning/override fixtures passed; full analysis acceptance pending |
| `sonar-package-report` | Media: `just/quality.just` | Complete binary archive, private permissions, and missing/linked evidence checks passed |
| `sonar-verify-result` | Media: `just/quality.just` | Real local API retries, exact analysis binding, PR/main scope, hotspots, and failure evidence passed |
| `verify-supply-chain-results` | Media: `just/quality.just` | Python task requires all three exact successes; caller conversion pending |
| `js-release-coverage` | Media: `just/quality.just` | Replaced by real Python tooling coverage; Python/E2E merge fixtures passed |
| `js-coverage-merge` | Media: `just/quality.just` | Real classic/module browser fixtures and complete source parsing passed; application collection pending |
| `validate` | Media: `just/quality.just` | Compared media gate ordering; ownership-lock and stop-on-failure composition passed; complete application gates pending |
| `ci` | Media: `just/quality.just` | Compared validation-before-release ordering; native task and failure composition checks passed; full CI acceptance pending |
| `build` | Media: `just/release.just` | Compared asset sync and workspace/all-target/all-feature selection; native build fixtures passed; actual media build pending |
| `build-release` | Media: `just/release.just` | Compared release/all-target/all-feature selection; native release fixture passed; actual media release build pending |
| `release-artifacts` | Media: `just/release.just` | Native source binding, custom Cargo target directory and packaged bytes verified; current media artifact acceptance pending |
| `sbom` | Media: `just/release.just` | Compared locked all-feature Cargo metadata; native metadata fixture passed; current media inventory pending |
| `licenses` | Media: `just/release.just` | Restored pinned cargo-deny installer prerequisite; native JSON license report passed; current media report pending |
| `api-export` | Media: `just/release.just` | Compared revaer-api generate_openapi binary invocation; native exporter fixture passed; media API acceptance pending |
| `helm-annotation-test` | Media: `just/release.just` | Static CLI task and Helm lint prerequisite implemented; multiline rendering, exact marker rejection and retained previous output verified |
| `compliance-chart-test` | Media: `just/release.just` | Python port of all 100 cases plus YAML integrity checks; selected-chart CLI and Helm lint composition implemented |
| `helm-package-test` | Media: `just/release.just` | Static CLI task and Helm lint prerequisite run native package/signing/registry and version regressions; no recursive lint invocation |
| `helm-lint` | Media: `just/release.just` | Actual foundation and media `rv helm-lint` pass; media includes 6 annotation, 102 compliance and 47 package/version tests plus strict lint and packaging |
| `helm-package` | Media: `just/release.just` | Strict lint and synthetic media bindings ported; current-media unsigned package verified with unchanged defaults/source; remaining annotation/signing comparisons pending |
| `helm-publish` | Media: `just/release.just` | Foundation implementation exists; media comparison pending |
| `release-dev` | Media: `just/release.just` | Foundation implementation exists; media comparison pending |
| `release-lock` | Media: `just/release.just` | CLI maps to uv lock for the unified Python dependency graph; supersedes the removed Node release lock operation |
| `sync-assets` | Media: `just/ui.just` | Foundation implementation exists; media comparison pending |
| `check-assets` | Media: `just/ui.just` | Foundation implementation exists; media comparison pending |
| `trunk-install` | Media: `just/ui.just` | Foundation implementation exists; media comparison pending |
| `ui-serve` | Media: `just/ui.just` | Foundation implementation exists; media comparison pending |
| `ui-build` | Media: `just/ui.just` | Foundation implementation exists; media comparison pending |
| `api-test-client` | Media: `just/ui.just` | Generated Node client replaced by typed Python requests and native JSON Schema response validation; 57 media-integrated API cases selected per auth mode, with one retained automation contract failure in each. |
| `ui-e2e-app-build` | Media: `just/ui.just` | Internal typed Cargo serving-artifact selection; real application library-test executable built and used by five-phase media validation. |
| `ui-e2e-app-test` | Media: `just/ui.just` | Implemented as `rv ui-e2e-app-test` with typed Cargo groups and unchanged default-feature selections; seven native fixture cases pass for success, failures and zero-test filters. Actual media launch/compliance/bootstrap groups pass all nine tests with explicit initialized runtime fixtures; foundation cleanup and strict Clippy also pass. |
| `ui-e2e-bootstrap-test` | Media: `just/ui.just` | Python tooling checks cover phase selection, owned services, single-init lifecycle, endpoint validation, cleanup and failure paths; final original bootstrap-case audit remains. |
| `ui-e2e` | Media: `just/ui.just` | All media API/UI scenarios ported. Nine media plus thirteen foundation UI cases pass on Chromium/Firefox/WebKit with an integration-only selector correction. Expanded API run: 56 pass/one retained automation failure in each auth mode. |
| `ui-e2e-coverage` | Media: `just/ui.just` | Python operation/UI route gate implemented; complete media acceptance remains blocked by the retained automation scenario and requires full route evidence. |
| `ui-e2e-shard-coverage` | Media: `just/ui.just` | Python exact-assignment/completed-phase gate and native shard fixtures implemented; final media workflow/sharded acceptance pending. |
| `runbook` | Media: `just/ui.just` | Python suite/coverage/archive composition implemented with failure evidence and locking; current-media complete acceptance pending. |
| `zombies` | Media: `just/ui.just` | Implemented; native Cargo/Trunk, 87 focused checks, the complete 1137-test suite and 32 Linux process/watch checks passed; full application acceptance pending |
| `dev` | Media: `just/ui.just` | Implemented; native Cargo/Trunk, 87 focused checks, the complete 1137-test suite and 32 Linux process/watch checks passed; full application acceptance pending |

### Single-init commands added by the current media work

| Source recipe | Python replacement | Evidence and remaining work |
| --- | --- | --- |
| `db-test-init`, `db-test-drop` | Static tasks using injected lifecycle settings and typed Docker/psql operations | Native lifecycle fixture covers sealing, roles, collisions and failure cleanup; full media initializer sealed and exercised through both auth modes and all three browsers with owned storage removal confirmed. |

### Current integration blockers from real application runs

- The fully ported API suite runs 57 scenarios per auth mode: 56 pass and the
  retained positive legacy automation case fails against the verified-identity
  admission rule. Keep this failure visible until the media contract is resolved.
- `rv ui-e2e-app-test` is implemented and passes seven native orchestration
  fixtures. Explicit initialized runtime fixtures now pass all nine actual media
  launch/compliance/bootstrap tests. Three media fixture checks and two foundation
  cleanup checks pass, with strict Clippy and owned-storage removal. Evidence and
  the integration patch are in `artifacts/runtime-fixture-qualification/`.
- The complete Python tooling check passed 1,698 tests before adding the focused
  app-regression task; the locked dependency audit also passed. This is tooling
  evidence, not complete Rust CI, media acceptance, coverage or Sonar completion.

### Workspace and documentation qualification

- Combined Python/Rust formatting passes. The full media `rv test` run fails in
  application tests that require initialized fixtures. Five positive bootstrap
  fixtures have been corrected in the disposable integration and all 28 selected
  bootstrap tests pass. Subsequent fixture corrections passed all 352 application
  library tests and all thirteen configuration integration tests. The broader
  workspace diagnostic exposed additional data/runtime/filesystem fixture gaps.
  Those corrections pass their targeted checks; full `rv test` qualification
  with serial scheduling passed on macOS and confirmed owned database cleanup.
  Complete CI, Linux-only cases, conversion fixtures and other release gates remain unproven.
- The foundation book builds warning-free with the pinned mdBook/Mermaid tools.
  The integrated book retains a large-search-index warning; do not call that gate
  clean or disable search to hide it. The API contract page is now `contract.md`
  to avoid its previous output collision with `index.md`.
- Logs and the integration fixture patch are retained under
  `artifacts/workspace-bootstrap-qualification/`.
