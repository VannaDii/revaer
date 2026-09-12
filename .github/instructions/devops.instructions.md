---
applyTo:
  - ".github/workflows/**"
  - ".github/actions/**"
  - "Dockerfile"
  - "release/**"
  - "charts/revaer/**"
  - "scripts/tests/compliance-chart-test.sh"
  - "scripts/tests/compliance-chart-test.rb"
  - "scripts/tests/helm-package-test.sh"
  - "scripts/tests/helm-package-test.rb"
  - "scripts/tests/instruction-drift-test.sh"
  - "scripts/tests/instruction-drift-test.rb"
  - "scripts/instruction-drift-check.sh"
  - "justfile"
  - "just/**"
  - "scripts/cargo-install-retry.sh"
  - "scripts/ensure-exact-cargo-tool.sh"
  - "scripts/test-exact-cargo-tool.sh"
  - "scripts/tests/exact-cargo-tool-test.sh"
  - "scripts/image-release.sh"
  - "scripts/install-sonar-scanner.sh"
  - "scripts/tests/install-sonar-scanner-test.sh"
  - "scripts/prepare-sonar-scm.sh"
  - "scripts/sonar-*.sh"
  - "scripts/verify-sonar-inputs.sh"
  - "scripts/with-node.sh"
  - "scripts/workflow-guardrails.sh"
  - "scripts/workflow_guardrails/**"
  - "sonar-project.properties"
---

`AGENTS.md` is the root contract. This file specializes workflows, release automation, container build files, and Sonar config.

# Workflow And Release Rules

- ADR 569 D1/D2 and conditional D3 were explicitly approved on 2026-09-10.
  `just db-init-final-proof` must retain the exact stock-extension inventory,
  definition/ACL mutation rejection, reset timeout scope/failure evidence, and
  independent cold/warm ingestion semantics. Reference normalization is not
  semantic proof. The final init remains inert until all cutover gates pass;
  no unrelated role, deadline, package or quality criterion is released.
  ADR 588 choice 1 approved the exact D4/D5 ingestion-family corrections on
  2026-09-11. The proof must retain frozen counterexamples, explicitly verify
  only the approved final error-to-success differences and fail on incomplete
  D3 scope; approval alone does not permit runtime cutover.
  The canonical final proof must always run the independent D4/D5 correction
  matrix before D3 comparison. The focused `--corrections-only` live harness
  is regression evidence only and must never substitute for the final proof.
  The existing D3 `warm-committed` and `existing-external-id` cases may recognize
  only their exact ADR 588 D4/D5 correction: retain `equivalent=false`, original
  SQLSTATE/native diagnostic evidence, and a named `approved_delta`. Compare
  every result, setting, identity and all 18 table images against independently
  specified expected changes. Additional differences and shared failures must
  fail; recognizing either correction must not discharge the D3 closure gate.
  The D3 compilation matrix must compare plain and disposable-observer runs
  against both exact variants. Retain complete per-operation table images,
  real persisted helper inputs, direct-role/backend/transaction provenance,
  and ordered in-call setting observations. Validate the approved D4 temporary
  table lifetime before normalizing that metadata; never normalize arbitrary
  text, result fields, settings or unobserved timestamps. Successful committed
  observer events do not establish error-path or full helper/trigger closure.
  Include the actual `search_result_ingest` application wrapper in the D3
  inventory and exercise discriminating stored scores, page boundaries and
  size-sample retention. Warm-after-rollback proof must retain the real first
  call, rollback images and same-backend committed retry; never label it as
  successful frozen committed reuse. Keep the separate approved D4/D5 matrix.
  Pin and verify read inputs as well as all 18 write-table images. Only named
  input timestamp columns equal to an observed seed-transaction clock may be
  normalized; unrelated fields or unobserved clocks must remain visible.
  Candidate/final disposable PostgreSQL cleanup must remove their anonymous
  volumes as well as containers, never prune unrelated Docker resources.
  Settings-path proof must pair uninstrumented runs with rollback-surviving
  test-only NOTICE traces, retain full native diagnostics and exact frozen
  statement/stack coordinates, and verify caller settings after savepoint and
  whole-transaction completion. Keep successful rollback/retry, late validation
  failure and nested policy-regex failure in cold/helper-first modes. These
  observations do not certify unobserved helper, native or concurrent paths.
  Validate combined NOTICE/error ordering before separating their records,
  independently verify seeded read inputs and their timestamp provenance, and
  bind nested regex errors to the selected title-rule callsite.
  Attribute/signal proof must preserve exact typed values, normalization,
  trust-bucket boundaries, identities, rollback sequence gaps and repeated
  signal rows. Retain the frozen suffix-regex and nullable-uniqueness defects;
  do not claim their intended confidence-upsert branch has executed. UUID has
  no valid observation attribute key, and D5 ID cases keep their separate proof.
  Missing-hash proof must independently specify both fixture states and every
  write-table result, distinguish durable source fill from incoming observation
  identity, and preserve competing-source conflicts and rollback sequence gaps.
  Retain wrapper source hashes before/after execution. These cases do not close
  unobserved metadata conflicts, concurrent reachability or native dependencies.

- `just test-database-baseline-read` exercises the ADR 551 read-only stored-procedure boundary against a caller-provided disposable Postgres service with the same workspace-wide all-feature selection as CI, filtering only the test names. Package-only feature resolution is not equivalent evidence for tracing behavior. It must fail when the service is missing, reject unmanaged databases without initializing them, and retain the frozen migration authority until the coordinated cutover. Baseline errors must never retain raw database messages, role names, or credentials.

- Use minimal GitHub token permissions at the workflow or job level. Only grant elevated scopes to the job that needs them.
- External GitHub actions in modified files must pin the exact upstream commit SHA. Do not use floating branch refs such as `main`, `master`, or `trunk`, and do not rely on mutable release tags alone.
- When updating an external action reference, resolve the chosen stable upstream release tag to its full 40-character commit SHA at the time of the change. Keep the originating tag in an inline comment when practical so upgrades stay auditable.
- Verify action usage against the action's current official documentation when changing its major or minor release line. Preserve documented step ordering and supported inputs.
- ORAS setup jobs in workflows must stay on a node24-capable `oras-project/setup-oras` release line and request an ORAS CLI version that the pinned action release explicitly supports.
- ORAS publish commands in release scripts must avoid absolute on-disk layer paths unless path validation is intentionally disabled; prefer running from the asset directory and pushing relative artifact names.
- Helm OCI publication defaults must target the owner-qualified GHCR namespace derived from the active GitHub repository. If a non-GitHub registry layout is needed, override it explicitly with `HELM_REGISTRY_NAMESPACE` rather than relying on an incomplete default path.
- Revaer's default public Helm OCI repository is `oci://ghcr.io/<owner>/charts/revaer`. Keep workflow defaults, install docs, and Artifact Hub registration aligned to that owner-scoped path.
- The shipped `charts/revaer/artifacthub-repo.yml` template is the source of truth for the Artifact Hub repository ID. Release packaging may append ownership data, but it must not duplicate an existing `repositoryID`.
- Trivy SARIF uploads from the reusable image workflow must set an explicit `upload-sarif` category when workflow refactors would otherwise rename the analysis identity. Keep that category aligned with the legacy `ci.yml` build-image matrix key so GitHub code scanning can compare PR scans against `main`. The exact two-entry amd64/arm64 matrix must run with fail-fast disabled, each scan must produce SARIF, and high or critical findings must fail the job with Trivy exit code 1; never neutralize PR findings to manufacture a passing context.
- Release packaging must preserve Artifact Hub ownership metadata when `ARTIFACTHUB_OWNER_NAME` and `ARTIFACTHUB_OWNER_EMAIL` are provided, even for unsigned packaging paths, because Artifact Hub ownership claim and verified-publisher flows depend on that published owner identity.
- Release packaging should publish an explicit `artifacthub.io/images` chart annotation for the Revaer image so Artifact Hub can index the runtime image and generate package security scans reliably.
- Helm release annotations must be rendered from a file through the portable renderer and covered by `just helm-annotation-test`. Do not pass multiline YAML through `awk -v` or interpolate annotation bodies into command text.
- ADR 588 C1-D requires an explicit platform-image digest, matching `amd64` or
  `arm64` selector, prepared existing PVC and manifest digest. Keep required
  values empty in the shipped chart; reject tags, repository reference
  suffixes, conflicting architecture and reserved compliance-checksum
  overrides. Both the volume and mount must be read-only, with the exact
  image/manifest subPath. Helm rendering is not image authenticity, prepared
  storage, installer completion or package-startup proof. `just helm-lint`
  must include `just compliance-chart-test` and `just helm-package-test`;
  signing and unsigned packaging
  lint must use `--strict` with explicit lint-only synthetic inputs. Never
  write those fixtures into chart defaults or published packages. Tests must
  inspect actual packaged defaults and preserve both signing-path checks.
  Keep authored Ruby test logic in tracked `.rb` files behind the canonical
  shell entrypoints so Ruby analysis and executed coverage see those sources;
  shell heredocs must not hide the test implementation from its analyzer.
- Sonar installer regression tests must inspect the real committed signing key
  with a private temporary GPG home, never the operator's keyring or trust
  database. Keep the exact fingerprint, archive and signature assertions;
  sandbox access failures do not authorize bypassing authentication checks.
- Workflows that install Rust toolchains must use the repository's configured toolchain source of truth rather than hard-coded ad hoc channels unless a documented exception is required.
- Workflow build, lint, test, coverage, security, image, signing, manifest, and release gates must call canonical `just` recipes. Workflows may install tools, authenticate, or upload artifacts, but must not execute raw Cargo gates, Docker builds or manifest publication, Trivy scans, Cosign signing, or Helm packaging/publication directly.
- Justfile recipes must run under non-login Bash so caller-selected Rust and NVM tool paths remain active inside every recipe.
- Database-backed quality recipes require a nonempty caller-supplied
  `REVAER_TEST_DATABASE_URL` or `DATABASE_URL` for a disposable test database.
  Preserve explicitly supplied distinct values and propagate the resolved pair
  to every database-backed child. Do not restore literal credential defaults or
  conceal them in URL construction helpers. Missing input and child failures
  must fail closed; the canonical policy suite exercises the real Just recipes
  with isolated command fixtures before any database-backed work is allowed.
- Media fixture acquisition must use `test-fixtures/lock.json` as the immutable source, revision, SHA-256, and byte-bound record. Cache keys must include that lock. Normal verification must create canonical probes in a private temporary tree and diff them against reviewed snapshots without modifying the worktree; snapshot replacement is allowed only through the explicit `just update-test-fixture-probes` operator recipe.
- The operator-approved ADR 578 F1 contract is test-preparation-only: bind `mkv-theora-vorbis-live-style` to its exact locked identity, unchanged snapshot, single LF-terminated ASCII diagnostic with the approved pointer/byte bounds, and exact full tool-report hashes. Reject unknown/conflicting contracts and any `allowProbeDiagnostics` field for this fixture. Retain and print the original native error, full tool report and explicit classification/count; missing evidence is fatal. Source, snapshot, diagnostic, tool or scope drift expires F1. Keep all existing exit/JSON/stderr limits, unfiltered ignored-suite execution and positive conversion-report checks; neither F1 nor the update recipe authorizes changing this snapshot or production/Sonar/GitHub criteria.
- Focused ADR 550 root-catalog parser and trusted-file validation runs through `just test-media-root-catalog`; keep that recipe scoped to `revaer-media-runtime` with all features and warnings denied.
- Focused RVB1 byte-codec validation runs through `just test-media-broker-codec` with all runtime features and warnings denied. It must retain independent known-answer vectors, exact field bounds, and rejection cases. A codec pass proves no broker lifecycle, execution closure, environment approval, or production containment.
- Focused ADR 557 root-input validation runs through `just test-media-root-contract` with all model features and warnings denied. Preserve canonical cursor bytes, exact key/path grammars, and page bounds without treating syntactic acceptance as active-catalog membership, filesystem confinement, mounted HTTP routes, or completion of the coordinated database cutover.
- Focused ADR 577 S1 shutdown-event validation runs through `just test-runtime-shutdown` and `just lint-runtime-shutdown` in both all-feature and minimal-feature app configurations, with warnings denied and real Tokio joins, scoped event capture, and cleanup assertions. Preserve warnings for panics, unexpected cancellation and grace expiry; INFO requires a cancelled join after this stop operation requested abort. ADR 588 choice 3 approved the exact S2/LIFE-1 supervisor and recovery contract on 2026-09-11; implement and qualify that contract, not the earlier unbounded watcher-join prototype. Approval is not evidence that the current shutdown path is contained. These focused gates do not replace integrated `just ci`, `just ui-e2e`, Sonar or required GitHub checks.
- Generated Playwright API schema output must remain ignored and untracked. Regenerate it from the committed OpenAPI document at test time through `just api-test-client`, using `npm ci --ignore-scripts` so `tests/package-lock.json` is the complete dependency-resolution source of truth; keep the generated-source guardrail and its fixture tests in `just policy`.
- Asset verification must run through `just check-assets`, which invokes the canonical synchronizer, compares `crates/revaer-ui/static/nexus/**` from the repository root, and fails unless repository-root `revaer-logo.svg` is byte-identical to `crates/revaer-ui/static/revaer-logo.svg`; a cwd-relative comparison that misses served assets is forbidden. Release validation must also confirm that referenced `/static/...` icon, logo, and DataTables URLs exist in the Trunk release output.
- `pr.yml` is the sole pull-request validation workflow. Its `pull_request` trigger must not narrow branches, paths, or activity types. Keep formatting, lint, test, audit, deny, coverage, E2E, media conversion, supply-chain aggregation, image, Helm, and release verification there so every same-repository stack pull request can emit all 21 contexts in `config/required-pr-checks.txt`.
- The media conversion job must invoke `just test-media-conversion` from the structurally enforced `Media conversion integration tests` step so the required context cannot degrade to fixture preparation alone.
- `just test-media-conversion` must first complete `verify-test-fixtures`, then run the unfiltered `revaer-media-runtime` `media_fixtures` Rust test binary with all features, warnings denied, and `--include-ignored`. A named filter, missing ignored-test execution, fixture-only body, or suppressed command failure is forbidden. Keep parsed Just-recipe regression coverage under the existing workflow guardrail owners, including integrity-before-Rust ordering and both commands' failure propagation. Preparation reports alone are not runtime conversion evidence.
- That recipe must delete any prior conversion report before integrity verification, route preparation evidence to the same report path plus `.preparation`, and require a newly written Rust report with a passed outcome, positive pipeline/video/audio operation counts, and zero pipeline/suite failures. Preserve `REVAER_MEDIA_CONVERSION_REPORT` for the Rust report. Missing tools, preparation, compilation, Rust execution, empty evidence, zero selected tests, or preparation-only evidence must fail; no stale report may turn an unsuccessful run into conversion proof.
- `pr.yml` must run its release-build validation job on pull requests. Keep post-merge and tag publication in `ci.yml`, but do not hide PR release-build validation behind main/tag-only guards.
- Required CI recipes must install Rust CLI tools at exact reviewed versions with `--locked`; do not let floating registry resolution decide the tool version at check time. Route cargo-audit, cargo-deny, cargo-llvm-cov, cargo-udeps, SQLx CLI, Trunk, mdBook, mdbook-mermaid, and Lychee through `scripts/ensure-exact-cargo-tool.sh`, which must replace missing, older, and newer versions while retaining an exact match, and through the bounded retry installer. The helper must force Cargo's install root to `${CARGO_HOME:-$HOME/.cargo}`, inspect and report that root's exact `bin/<binary>` executable instead of a same-named PATH entry, and pass the Cargo subcommand name when probing `cargo-*` binaries. An earlier PATH shadow must neither satisfy the check nor force reinstallation when the exact Cargo-home executable is already present. Documentation recipes pin the compatible pair mdBook `0.5.0` and mdbook-mermaid `0.17.0`, plus Lychee `0.24.2`, and must propagate documentation build or link-check failures. The canonical `just audit` gate must run Cargo audit and npm audit at `info` severity for every committed npm lockfile, with no audit exceptions. `just sqlx-install` must install SQLx CLI `0.8.6`, `just trunk-install` must install Trunk `0.21.14`, and `just udeps` must pair cargo-udeps `0.1.57` with `nightly-2026-06-13`, run `--workspace --all-targets`, and emit compiler, tool, and command evidence. Keep all pins in the PR cache key and update them only through an explicit reviewed change with a successful smoke run.
- A `cargo install` process that exits successfully but emits warning output must fail closed once with status 65; do not retry and reinstall the already-produced executable. The exact-tool helper must post-probe its owned Cargo-home path for diagnostic evidence and then preserve that status. Genuine nonzero Cargo failures retain the bounded retry policy and their final real exit status.
- The canonical UI E2E gate must install the exact `tests/package-lock.json` graph with lifecycle scripts disabled and run `npm audit --audit-level=info` before generating clients or starting browsers. Every reported npm severity is blocking; refresh the lock or dependency graph instead of adding an audit exception. The recipe must propagate Playwright, setup, teardown, and coverage command failures without allowing later artifact assertions to overwrite the failing status.
- Documentation builds must pin mdBook `0.5.0` to the protocol version used by pinned `mdbook-mermaid 0.17.0`. Browser validation must not pass conflicting `NO_COLOR` and `FORCE_COLOR` settings into Playwright; remove the inherited `NO_COLOR` setting at that process boundary instead of discarding warning output.
- Every pull request must emit `Supply Chain Checks` as a fail-closed aggregate of the independent audit, deny, and unused-dependency jobs. The aggregate must run under `if: always()` and reject every upstream result except `success` through the canonical `just` verifier.
- `pr.yml` must run the media fixture gate through the canonical fixture recipes, publish a nonempty report, and run `just clean-test-fixtures` under `if: always()` after report upload. Image and release-build jobs must depend on that fixture gate.
- `.github/build-inputs.env` is the single reviewed manifest for exact Rust, Node.js, npm, just, PostgreSQL rebaseline tools, Dockerfile frontend digest, OCI base-image digest, Alpine release, and Alpine package inputs. The setup action, Dockerfile, local image recipe, cache keys, image labels, embedded compliance data, and digest-bound evidence must consume or verify that manifest. Floating tool channels, mutable Dockerfile frontends, unqualified base-image tags, and unversioned APK resolution are forbidden.
- `ci.yml` is the post-merge and tag-release workflow. Limit it to release-artifact, publish, and image-build activity for `main` pushes and release tags; do not duplicate PR validation jobs there.
- Manual release verification belongs in dedicated `workflow_dispatch` workflows, not in `pr.yml`, and should reuse the same `just` entrypoints and pinned third-party actions as the release path they exercise.
- Manual workflows that publish PR-scoped dev Helm artifacts should encode the PR number into the default prerelease version so registry output is traceable back to the reviewed change.
- `workflow_dispatch` string inputs that flow into shell or release commands must be validated and normalized before use. Reject unsafe or malformed values instead of passing them through to `just`, Helm, or release scripts.
- Reusable image workflows may publish PR-scoped dev Helm charts only as an optional post-manifest job. Keep that publish step downstream of the multi-arch manifest job, drive it through `just helm-package` and `just helm-publish`, and derive the default prerelease chart version from the caller-provided PR number.
- PR image verification and any explicitly authorized publication must wait for all UI shards, feature, native, media-conversion, Sonar/coverage, supply-chain, and matrix-loading jobs. Ordinary pull requests build and scan verification images without pushing or signing them. The same-repository caller must set `publish_dev_helm: true` before an eligible post-manifest dev chart publication; a failed prerequisite must prevent image, manifest, signature, and Helm publication.
- Every UI shard coverage upload must use `if-no-files-found: error`. The aggregate must download all three exact named artifacts, prove nonempty API and UI records for each shard through `just ui-e2e-shard-coverage`, and only then evaluate combined route coverage.
- Canonical `just ui-e2e` requires `just ui-e2e-bootstrap-test`, prepares the
  host app with `just ui-e2e-app-build`, selects the exact completed Cargo
  library-test artifact and directly runs
  `bootstrap::runtime_tests::e2e_serving_entry`. Its explicit compliance loader
  is `cfg(test)` only; production entrypoints retain required packaged metadata.
  Preserve the real runtime, disposable database, setup/auth flows, owned process
  groups, complete suites, route assertions and coverage gates. Occupied ports
  must fail without terminating other workloads. Keep focused preparation checks
  at `just ui-e2e-bootstrap-test` and `just ui-e2e-app-test`, using the existing
  Node wrapper; these checks are neither the full UI gate nor package evidence.
- Release-tag image publication in `ci.yml` must not depend on `release-dev` or any other `main`-only job. Split dev and tag image publishing into separate jobs when their prerequisites differ.
- Stable tag activity in `ci.yml` must exclude prerelease tags consistently at the job boundary, not only in downstream publish jobs. Do not let prerelease tags build stable release artifacts that the later jobs refuse to publish.
- Reusable-workflow caller jobs must not use `secrets: inherit` unless the callee truly requires repository secrets. Prefer the default GitHub token plus explicit job permissions, and pass named secrets only when the callee consumes them.
- Helm chart validation and publication must flow through `just helm-lint`, `just helm-package`, and `just helm-publish`. Do not add ad hoc packaging or registry-push shell blocks to workflows.
- Helm packaging must render multiline annotations without passing embedded newlines through `awk -v`; keep the renderer portable across the BSD and GNU userlands used by local and hosted gates.
- The default Helm lint database URL must contain no credentials: rendering
  never needs a database connection. Exercise both the default and explicit
  override paths through real unsigned packaging; do not suppress a secrets
  finding because the hard-coded value was intended as a fixture.
- Every workflow job that invokes `just` must install it first through `./.github/actions/setup-revaer`; do not assume any hosted or self-hosted runner image already provides it. This includes each architecture job in the reusable image workflow before the Trivy verifier runs.
- PR UI E2E jobs must use the runner-provided Chrome channel, shard the `ui-chromium` project without dependency projects, and install Playwright system dependencies without downloading redundant browser bundles. Run API route coverage in a separate API E2E job, construct that job's database URLs from the exact run-scoped Postgres service credentials, upload its route-coverage artifacts, and include them in the aggregate E2E coverage gate. Keep CI video capture disabled unless Playwright's bundled ffmpeg is intentionally installed.
- `just lint` runs `scripts/workflow-guardrails.sh`, which rejects unpinned external action refs, direct `${{ inputs.* }}` interpolation inside `run:` blocks, direct workflow release/security gates, and nonempty Sonar coverage exclusions.
- Workflow guardrails must parse YAML structure rather than search whole files for policy strings. Validate real jobs, steps, conditions, permissions, action references, and `just` invocations, and retain adversarial fixtures proving comments, environment values, descriptions, and unrelated keys cannot satisfy or trigger a rule.
- Sonar property guardrails must parse Java-properties logical keys before applying the exact allowlist. Leading-whitespace forms, escaped keys, continuations, duplicate logical keys, and unknown properties are fail-closed errors; do not return to line-oriented `awk` or `grep` parsing.
- Treat `sonar-project.properties` as the versioned source of truth for Sonar analysis scope. Coverage exclusions must remain explicitly empty so Sonar imports the Rust LCOV, native LLVM coverage, JavaScript LCOV, and generic authored shell/Ruby coverage generated by the canonical coverage recipes instead of publishing zero coverage.
- PR and main-branch Sonar jobs must generate and validate every configured coverage input, including LCOV from the compiled Playwright TypeScript harness that executes the real specs, fixtures, setup, and teardown, retain the native compile database and CXX bridge headers, run the post-scan result verifier, and upload the complete nonempty scanner report. The PR verifier must receive the exact pull-request number so every measures, quality-gate, issue, and hotspot query evaluates the submitted PR analysis rather than the main branch. Remove untracked dependency-install directories, generated API clients, UI build output, and Playwright result mirrors before scanning so third-party or generated contents cannot dilute authored analysis; do not add scanner exclusions for them. Do not let the PR workflow run a smaller analysis than the main-branch workflow.
- Published architecture images must be built before compliance evidence is evaluated, independently resolved to an immutable registry digest, inventoried and scanned through that digest-qualified reference, and bound to complete generated evidence. Validate the exact digest before signing, sign the digest and compliance predicate, verify the signed attestation, and block manifest or Helm publication when any stage fails.
- Trivy must emit SARIF with `exit-code: 0` only so the report survives for `if: always()` upload; a separate mandatory verifier must fail on every HIGH or CRITICAL result. Keep deterministic vulnerable-image SARIF regression coverage for this control.
- Sonar-running jobs must prove the checkout is non-shallow, explicitly fetch the reviewed base ref, remove only an empty checkout shallow marker, and reject any remaining marker before analysis so Sonar's Git implementation cannot silently discard pull-request attribution or SCM blame.
- `just sonar-compile-db` must clean the isolated native package build before compilation and fail unless it emits a nonempty `coverage/compile_commands.json`; repeated local or CI invocations must never reuse a cached build-script result after deleting the prior database.
- External `docker://` actions require an exact `sha256` digest in addition to the exact-SHA rule for repository actions.
- Every direct checked-in workflow job requires a positive `timeout-minutes` no greater than 180. Every `setup-revaer` step requires `timeout-minutes: 20`. Required jobs and steps must not use `continue-on-error: true` or expressions that can conceal failure; literal boolean or string `false` is permitted.
- The root `justfile` is an import-only index using non-login `bash -c` and exactly the seven ADR 482 modules under `just/`. Each recipe has one module owner. Node commands run through `scripts/with-node.sh`, which selects and verifies exact Node 24.19.0 from `.nvmrc` so the NVM-managed version remains active.
- Cargo analysis tools are exact: cargo-udeps 0.1.57 on `nightly-2026-06-13`, cargo-audit 0.22.0, cargo-deny 0.18.9, cargo-llvm-cov 0.8.7, sqlx-cli 0.8.6, and trunk 0.21.14. Install them through `scripts/ensure-exact-cargo-tool.sh`; do not accept newer, older, floating-nightly, yanked-lock warning, or minimum-version substitutes. Preserve cargo-udeps cache and toolchain evidence in PR CI.
- ADR 522 database rebaseline work must use the digest-qualified PostgreSQL image and exact server/client version in `.github/build-inputs.env`. The freeze guard validates the immutable migration corpus and keeps `init.sql` absent in the freeze phase; the candidate recipe writes only ignored local evidence, applies the full candidate to a fresh database, compares its normalized schema re-dump, and verifies its pinned SHA-256 and SQL statement count. The PR Feature Matrix job must run `just db-rebaseline-candidate` before migration-backed tests on every pull request. Assembly prefixes must be exact statement-boundary prefixes, apply to an empty database, and stay within the canonical no-rename changed-line limits. The full frozen candidate is 1,624 statements, 1,593,023 bytes, and 44,630 lines. ADR 551 finalization must retain and verify that candidate before validating exact authorized deltas, the independently reviewed final digest, and live constrained-role privilege evidence through `just db-init-final-proof`. Binary or uncountable diffs fail closed. Final SQL remains inert review evidence until coordinated cutover; finalization cannot select it for ordinary runtime/tests or retire migrations.
- `scripts/workflow-guardrails.sh` composes the five ADR 482 Ruby owners for input loading, GitHub Actions, Sonar properties, required checks, and diagnostics. Do not merge them back into an unstructured parser or add a sixth policy owner without a separately approved decision.
- `just cov` must fail immediately on instrumentation/test failure before any
  report can mask it. Its separate 90% Rust package thresholds stay unchanged.
  `just cov-report` must disable cargo-llvm-cov's default test/example/benchmark
  hiding for LCOV, HTML and native exports, retain the existing Rust/native
  stream separation, and propagate every export failure. The existing required
  checks owner validates the parsed recipe contracts; test records must never
  dilute the established package gates or disappear from Sonar's input.
- Treat `sonar-project.properties` as the only versioned source of truth for Sonar scanner criteria. The exact scanner is installed only by `setup-revaer`, and `just sonar-scan` is the sole invocation in PR and main workflows. Version updates require exact per-platform SHA-256 pins, the committed fingerprint-verified SonarSource key, detached-signature verification, fixtures, and a task record.
- Sonar criteria and server settings are fail-closed under the root policy. Do not relax any property, source scope, analyzer, coverage input, quality-gate condition, issue or hotspot state, new-code definition, or required check without exact operator consent naming the change, scope, reason, and expiry.
- Release-tooling dependency changes under `release/**`, including JavaScript lockfiles such as `release/package-lock.json`, must stay manifest-scoped, avoid unrelated workflow churn, and update this instruction file in the same change so instruction-drift remains explicit.
- Database rebaseline validation must translate missing candidate files into the existing bounded failure diagnostic with a repository-relative path. Do not call platform-specific exception accessors or treat missing evidence as successful validation; keep the missing-file regression in the policy suite.
- ADR 551 pristine catalog evidence uses only `just db-pristine-catalog-generate`, `just db-pristine-catalog-validate`, and `just db-pristine-catalog-test` with the existing pinned PostgreSQL image. These fixture-only commands must read as a constrained database owner, account for every column in the 27 named catalogs, normalize identities, fail on unreadable populated catalogs, and remove their unique disposable container and volumes. They neither initialize Revaer nor change the frozen migrations or assembly file.
- ADR 569 D3 ingestion proof must retain cold-session cases and exact known-answer helper-first cases under the caller's unchanged compilation setting. Verify helper records precede ingestion, compare all retained application results and table mutations, and reject missing, duplicate or changed evidence. These cases do not replace complete helper/trigger closure or successful repeated calls across commits. Preserve the failing warm counterexample until its separately approved correction is proved; never repair the test backend, accept its error, or certify D3 from a partial matrix.
- Identity proof must bind reused canonical/source/observation IDs to independently
  selected, recorded fixture rows. Returned IDs cannot define the expected target.
  Include a coherent wrong-selection regression, distinct competing identities,
  exact refresh/hash assertions and the wrapper's best-source result.
- The canonical D3 validation matrix must exercise the 32 frozen validation
  guard sites with exact SQLSTATE, detail and native/source coordinates, not
  infer an expected guard from the observed error. Retain all 18 table images,
  read inputs, seed clocks and direct role/GUC/backend evidence across cold,
  helper-first and same-backend repeated calls. Preserve legitimate NULL/empty
  controls and the exact approved D4 third-call counterexample. Report corrected
  controls as `equivalent=false` with explicit acceptance and approved delta;
  comparison of their first two calls cannot certify full equivalence. Keep
  the matrix in the canonical final proof and its unit harness in the policy
  suite. These bounded guard cases never discharge the incomplete-D3 sentinel.
- The canonical D3 gate must run the populated policy matrix and paired
  catalog/native dependency inventory. Reject unexpected definitions, settings,
  cast/dictionary/trigger bindings and read-input mutations. Only successful
  same-process compilation/wrapper evidence may feed FK-input observations.
  Producers must register the exact serialized bytes only after successful
  validation; consumers must reject absent hashes, substituted bytes, missing
  or duplicated frames, and invalid backend identities. Independently validate
  declared catalog roots and referenced bindings, not just paired equality;
  installed callbacks, eligible DML and `pg_depend` reachability are not callback
  execution or complete PL/pgSQL late-binding proof. Preserve the explicit
  incomplete-D3 failure until all remaining conditions are independently proved.
- Existing-data ingestion proof must create application state through real recorded fixture ingestion, then use a distinct cold tested backend. Retain both stages' results, roles, settings, transaction-clock provenance and all 18 before/after table images; verify fixture continuity and stable identity relationships. Shared reference/final failures remain failed required outcomes, not successful parity. Neither cold reconnects nor timestamp/identity normalization may conceal warm-session defects or unrelated data changes; D4 and D5 need their own exact approvals.
- The release lock's `cosmiconfig` YAML loader must resolve `js-yaml` outside the vulnerable `4.0.0` through `4.3.1` range in `GHSA-2883-xcg3-v3hh`; `4.3.2` is the minimal patched release. Keep its existing compatible dependency range and audit every committed npm graph at `info` severity after transitive security updates, without exceptions.
- Prerelease Helm assets must be produced during the semantic-release prepare phase so the packaged chart version matches the dev release version exactly. OCI publication must consume those already-packaged assets after the GitHub release assets exist.
- Stable tag releases must package the Helm chart once, attach the `.tgz`, `.prov`, and public key to the GitHub release, and publish that exact packaged chart to the OCI registry. Avoid repackaging between release-asset upload and OCI publication.
- Release and Artifact Hub branding documentation must reference the committed `revaer-logo.svg` asset, preserve its approved purple stylized-R identity, and must not advertise removed raster logo variants.
- JavaScript release metadata helpers under `release/**` should stay side-effect scoped. Prefer wiring shell packaging steps in the semantic-release `prepareCmd` over spawning child processes from Node glue unless a documented exception is required.
- Semantic-release command templates under `release/**` must remain lodash-template-safe. Do not embed shell parameter-expansion forms such as `${VAR:-default}` inside configured command strings because lodash templating parses the same `${...}` syntax first. Semantic-release placeholders such as `${nextRelease.version}` remain allowed; for shell conditionals prefer plain shell variable references such as `"$VAR"` or move the conditional logic into a script.
- Helm packaging scripts must exclude repository-level Artifact Hub metadata from the chart tarball itself. Publish `artifacthub-repo.yml` as a separate OCI artifact instead of shipping it inside the chart package.
- Helm publishing must verify signed chart artifacts before OCI push, and temporary exported secret keyring files must be created with owner-only permissions.

# Shell Safety

- Never interpolate untrusted `${{ inputs.* }}` or comparable expression values directly into `run:` blocks.
- Map user-controlled inputs into environment variables first, validate or whitelist them, then consume them in shell.
- When writing validated values to `$GITHUB_OUTPUT`, use the multiline heredoc form so output parsing stays safe even if the value surface changes later.
- Prefer arrays and quoted expansions over word-splitting command strings.
- Temporary compliance findings must use private unpredictable storage with guaranteed cleanup. Never restore a predictable shared `/tmp` pathname; retain a symlink pre-placement regression.
- Setup-action package-list inputs may accept general shell whitespace, including CRLF-pasted multiline input, when that improves YAML readability, but the resulting tokens must still be normalized into a validated array before invocation.

# Credentials And Test Infrastructure

- CI-only credentials may be ephemeral only when they are clearly scoped to isolated test infrastructure, such as throwaway Postgres service containers. Derive those credentials from the isolated workflow run context; do not commit credential-shaped literal passwords even for disposable services.
- Every workflow job that declares a Postgres service must construct `REVAER_TEST_DATABASE_URL`, `DATABASE_URL`, and any E2E admin URL from that service's exact run-derived user, password, and database. Repository variables must not override or drift from an in-job service credential.
- Ephemeral test credentials must never be reused as application secrets, committed runtime credentials, or user-facing examples.
- `just cov` must pass its resolved test database URL to both database startup and the coverage process in the same recipe shell so database-backed tests cannot silently lose their configured endpoint. It must instrument linked first-party C/C++ through a Clang version compatible with rustc's LLVM tools, retain Rust-only package thresholds, emit both Rust LCOV and native LLVM reports, and run `just script-coverage`. Script coverage uses the exact source-built and archive-hash-verified `kcov` revision pinned in the canonical setup action for authored Bash plus Ruby's built-in line and branch coverage, and converts those real execution records to Sonar's generic format; omitted or unexecuted executable shell lines must remain uncovered. Kcov Bash tracing must select and behaviorally validate Bash 4.2 or newer, prepend that interpreter for traced child shells, and cap only the coverage process's open-file limit so kcov cannot derive an invalid trace descriptor from an oversized host limit. Before child tests run, coverage must fail closed unless kcov's generated Bash trace helper contains its exact recognized prompt, then make only the source expression nounset-safe with the stable `kcov-inline` fallback. The complete kcov log must be retained, and a nonzero process or logging status, unexpected helper shape, version drift, or any kcov error or warning is fatal; the pinned kcov binary, authored source paths, coverage scope, and non-coverage execution must remain unchanged. Disposable test databases must be dropped with forced session cleanup, and local-only host fallback must remain bounded to `localhost` and `127.0.0.1` inputs.
- Sonar coverage jobs must fetch complete Git history and exercise authored Rust and native C/C++ code in one `just cov` run so the scanner can resolve the main-branch baseline and import both retained reports.
- PostgreSQL service containers used by pull-request, Sonar, and managed local validation must reserve 1 GiB of shared memory so concurrent isolated-schema migrations cannot exhaust Docker's 64 MiB default. `just db-start` must recreate its named managed container when the configured allocation is smaller.
- Do not log secrets or secret-like values. Mask or omit them.
- Keep Helm registry credentials (`HELM_API_KEY_ID`, `HELM_API_KEY_SECRET`) separate from chart-signing material (`HELM_GPG_PRIVATE`, `HELM_GPG_PUBLIC`). Publishing jobs may use registry credentials only when consuming an already-packaged chart artifact.
- GHCR chart publication on GitHub-hosted runners should prefer the job-scoped `GITHUB_TOKEN` plus explicit `packages: write` over long-lived custom registry secrets. Keep `HELM_API_KEY_*` only for non-GitHub or local override paths.

# Drift Control

- Any change to a workflow, release script, setup action, `justfile`, or `sonar-project.properties` must review the matching instruction file in the same change.
- Revaer enforces that rule mechanically with `just instruction-drift`, backed by `scripts/instruction-drift-check.sh`. Keep the mapping in that script aligned with this file and `AGENTS.md`.
- Keep `scripts/workflow-guardrails.sh` aligned with the live workflow policy when GitHub Actions pinning or shell-safety rules change.
- `pr.yml` must pass explicit base/head SHAs into `just instruction-drift` so pull requests are checked against the real reviewed diff, not an incidental worktree state.
- Drift coverage for actions and release assets is recursive. Changes under `.github/actions/**`, `.github/workflows/**`, and `release/**` must keep matching the devops instruction update rule.
- Reusable workflows that publish images must preserve `packages: write` on the caller job because the callee cannot elevate a more restrictive token.
- Reusable-workflow caller jobs must define one merged `permissions` map. Do not duplicate the `permissions` key in a job to append scopes later; GitHub Actions rejects the workflow before execution.
- Keep the Sonar PR and main gates blocking. Scanner-side waiting, retained report/task evidence, and API verification must all evaluate the same single analysis; branch decoration alone is not sufficient proof.
