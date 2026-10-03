---
applyTo:
  - ".github/workflows/**"
  - ".github/actions/**"
  - "Dockerfile"
  - "release/**"
  - "sonar-project.properties"
  - "setup.sh"
  - "tools/**"
  - "pyproject.toml"
  - "uv.lock"
---

`AGENTS.md` is the root contract. This file specializes workflows, release automation, container build files, and Sonar config.

# Workflow And Release Rules

- An audit exit code of zero is insufficient when packages were skipped. Source
  dependencies must retain uv provenance/hash verification and complete advisory
  coverage; URL requirements must not silently escape the dependency gate.

- Root `setup.sh` coverage follows the operator-approved allowance in ADR 592:
  at most one uncovered executable line, with positive execution and complete
  native records. Fully covered runs pass. No hit counts or scanner scope change.

- Development tasks use the locked `watchfiles` and `psutil` APIs. Preserve Git
  ignore behavior, Trunk live reload, failed-build recovery and retained logs.
  `rv zombies` may stop only recorded checkout-owned processes, validated by
  creation identity and inherited session token. Port occupancy or an executable
  name is not cleanup authority. Keep ownership receipts after cleanup failure.

- The accepted Python migration is recorded in `docs/adr/592-python-tooling.md`.
  Keep existing gates in place until their `rv` replacements have demonstrated
  parity; do not confuse the new tooling checks with completion of project CI.
- Changed-line commands count the complete exact-commit diff against the selected
  reviewed limit. Preserve ASSET-1's exact inventory, original blobs, committed
  implementation/decision checks and fresh complete provider history. It remains
  scoped to PR 130 and permanently expires on close, merge or reopen. The migrated
  implementation paths do not authorize a new exception or extend its lifetime.
  Ordinary binaries and all assembly-scope binaries remain uncountable failures.
- uv owns Python installation, project environments, lockfiles, and Python tool
  entry points. Bootstrap uses its official versioned installer and supported
  options; the setup action uses the official `astral-sh/setup-uv` action and
  its documented version-file/cache inputs. Never extract uv wheels, maintain platform download
  tables, copy launcher executables, or modify uv environments manually.
- Run project Python tools with `uv run --locked`. Install the dependency-free
  worktree launcher through `uv tool install`; do not install the full project's
  dependencies into a second, independently resolved tool environment.
- Native package managers own their installation records. Do not force matching
  Cargo tools to rebuild during every setup invocation.
- Workflow setup declares its Cargo/browser selections explicitly and validates
  them before installation. Local setup retains the complete default. Debian
  packages go through typed apt arguments with native signature verification;
  repository-refresh failure must prevent installation. Root test containers and
  noninteractive sudo on hosted runners are separate injected privilege paths.
- GitHub environment/output writes use the shared multiline command-file writer.
  Reject invalid record names, symlinks and nonregular destinations, preserve
  previous values, and publish release completion only after artifact verification.
- The setup action's README is the input reference. Caller workflows must use its
  declared inputs and invoke `uv run --locked -- rv`; retired Node/Just/toolchain
  inputs must disappear with the corresponding workflow migration.
- Every workflow command requires an earlier unconditional setup action. Its uv
  install, locked synchronization and initial setup must also be unconditional;
  optional native tools may be selected by the declared action inputs.
- Documentation deployment runs `rv docs-guard` and `rv docs-prepare` before the
  pinned Pages action publishes the prepared payload to `gh-pages`. Keep the
  existing secret-pattern and committed-file-type gates, optional CNAME, source
  identity and LLM manifests. Do not embed shell date expressions in messages.
- Manual and reusable chart workflows share `rv workflow-chart-versions`.
  Preserve explicit overrides and PR/run-number defaults, validate versions
  before external operations, and treat an ambiguous or failed PR lookup as an
  error. `gh` owns query encoding/authentication; `rv` owns output framing.
- Image evidence uses the Python SARIF and v2 compliance validators. Retain the
  separate HIGH/CRITICAL gate, complete SPDX/source checks, five artifact hashes
  and exact image binding. Generation stages a complete bundle before replacing
  output and must not replace tracked source or follow linked artifacts.
  Fixture inventories prove validation behavior, never actual image compliance.
- Image workflow stages use typed Buildx/Trivy/Cosign adapters. Build completion
  must compare native metadata with the independently resolved registry digest.
  Resolve architecture tags before manifest creation, require destination tags to
  agree, and sign an immutable digest. Preserve exact repository/workflow identity,
  issuer and predicate verification, including literal dots in certificate patterns.
  Verification-only jobs retain their local-image path and unsigned chart checks.
- Existing chart verification reads the package through Helm and checks the
  requested chart/application versions and Artifact Hub repository ID. Registry
  publication always verifies signed artifacts, including when called directly.
- Post-merge/tag release automation uses the existing `rv release publish`
  operations. Stable tags use `--tag`; preserve detached-tag verification and
  complete source history. Downstream artifact names come from the producer's
  actual checkout identity. Dev chart/image publication requires the release
  task's `released=true` output, emitted only after remote assets verify. A
  successful no-change run must not publish an earlier release's tag or chart.
- Release registry jobs consume the exact already-signed Helm artifact and
  verified release version/tag outputs. Keep signing material in the release
  packaging job, and preserve stable image publication's independence from the
  main-only prerelease job. `rv policy` checks these release workflow contracts.
- Tooling tests must not install or replace global native tools. Inject the
  installer boundary for setup orchestration tests and use actual local tools
  for compatibility checks. Database tests own isolated servers and their data.
- Preserve the media stack's explicit disposable database requirement and native
  test serialization. cargo-udeps uses the compiler/tool pins in
  `tools/versions.toml`; analysis must retain compiler evidence and reject
  conflicting overrides instead of falling back to another toolchain.
- Preserve existing release branch eligibility, opt-in Helm release assets,
  credentials precedence, artifacts, and failure behavior during migration.
  A release preview must use the same configured branch rules as publication.
- Registry publication uses the official Helm/ORAS registry configuration and CA
  options. Keep operation credentials in a private temporary configuration and
  require TLS certificate/hostname verification, including localhost registries.
- Maintain focused README files for setup, architecture, command ownership, and
  verification. Comments should explain ordering, ownership, and compatibility
  constraints that are not apparent from the code.
- Rust coverage retains the existing per-crate 90% Rust line gate and publishes complete
  LCOV, text, and HTML evidence with native sources and build scripts. Do not add
  filename exclusions during the migration. Resolve workspace members through
  Cargo metadata, preserve other coverage producers, and invalidate old reports
  before a new collection. Analysis uses installed tools and retains their versions.
- Native integration is enabled during collection. Keep native measurements in
  per-package JSON and all published reports. The existing Rust threshold uses
  the engine's exact Rust line counts; adding C++ measurement must not silently
  redefine that threshold. Matching `rust-src` and LLVM path equivalence supply
  missing compiler-mapped source files without suppressing diagnostics.
- `rv sonar-compile-db` must obtain compiler commands from the native build
  script on every run, verify the current checkout's bridge is present, and
  invalidate stale output before building. It may not fabricate analyzer inputs
  or narrow Sonar criteria. The scanner migration must retain the stricter
  user-supplied root contract; old scoped exclusion advice is superseded by it.
- The native compilation database must retain its generated CXX headers after
  command completion. Scanner installation uses SonarSource's official ZIP,
  reviewed archive digests, and committed primary signing fingerprint in an
  isolated GnuPG home. Reverify cached archives and recreate extracted files.
- Python analysis coverage combines actual tooling and completed E2E data with
  Coverage.py's supported commands. Report every authored Python file with its
  full checkout-relative path, including never-imported files; retain the
  independent 90% tooling/launcher gate. Corrupt coverage inputs must fail.
- Browser source coverage parses all checkout JavaScript, including unused ES
  modules, through the official Tree-sitter Python bindings and grammar. Chromium
  V8 supplies every positive count, bound to exact source hashes and UTF-16
  offsets. Preserve nested zero-count ranges and full per-file line evidence;
  do not execute imports to inventory sources or invent positive defaults.
  Instrument pages before scenarios receive them and capture before closing.
  Coverage failures retain the attempt's traces, videos and partial raw records.
- Direct audit/deny commands enforce the empty advisory policies. Direct scanner
  execution enforces source inventory and the exact properties contract after
  invalidating old success artifacts. Keep these checks shared with `rv policy`.
- Root bootstrap tests execute the unchanged script through the official uv
  installer in disposable checkouts. Measure it with kcov's documented DEBUG
  method and retain complete raw line records bound to its exact source bytes.
  Delegate child descriptor limits to uv's `UV_RUN_RLIMIT_NOFILE` option; do not
  change the parent process limits or introduce a custom child launcher. Do not patch kcov's
  generated helper or turn unexecuted lines into hits. The Linux source build
  retains the existing commit/SHA-256 pin and uses CMake's supported commands.
  Native installation fixtures must own their install prefixes and caches.
- Scanner preparation must preserve exact event-base/head attribution and refuse
  tracked-source cleanup. Retain complete scanner reports and the last API
  attempt, including failure evidence. Keep the scanner's parameter dictionary
  and JVM property overrides unavailable; criteria belong in the properties file.
- Media tasks read the selected checkout's manifest and source lock. Preserve
  HTTPS-only native curl transfers, exact source hashes/size bounds, all declared
  FFmpeg generation recipes and byte-exact reviewed ffprobe snapshots. Keep
  ADR 578 F1 bound to its existing source/snapshot/tool-profile/diagnostic bytes;
  do not broaden it during migration. Preserve raw process line endings and
  complete private diagnostic evidence. Conversion acceptance requires a fresh
  report from the actual ignored integration suite, positive audio/video actions
  and zero failures. Scope temporary conversion files and cleanup to this checkout.
- `rv validate` holds the selected database's operation lock across its ordered
  gates; `rv ci` performs the release build only after all gates succeed.
  Fixture validation does not stand in for full foundation and media acceptance.
- `rv runbook` keeps the E2E lock through artifact archival and writes its summary
  last. Failed and interrupted runs retain evidence and a failing exit status.
- Local image builds name their Buildx builder explicitly without changing the
  developer's selected builder. Failed builds invalidate their partial outputs.
  Container scans use versioned Trivy policy, include unfixed HIGH/CRITICAL
  findings, preserve failing reports, and do not consume advisory-ignore files.
- The Python E2E runner loads `tests/.env` through `uv run --env-file`, owns a
  unique database, and checks listener ownership before setup/reset API calls.
  Its service cleanup precedes database deletion. Preserve phase ordering,
  recording modes, overrides, route evidence, and current-run failure summaries.
- E2E CI shard aggregation must consume only the current workflow attempt's
  artifacts. Run both `rv ui-e2e-shard-coverage --shards N` and the route gate;
  preserve each shard's completion summary instead of overwriting a shared file.
- Managed database operations require matching checkout ownership in Docker,
  persistent storage, and the responding server. Never adopt by container name,
  reset a caller-owned database, or reset automatically after a migration error.
  Keep SQLx responsible for migration history and explicit reset. Use official
  PostgreSQL image options and libpq inputs, and keep passwords out of arguments.

- Use minimal GitHub token permissions at the workflow or job level. Only grant elevated scopes to the job that needs them.
- External GitHub actions in modified files must pin the exact upstream commit SHA. Do not use floating branch refs such as `main`, `master`, or `trunk`, and do not rely on mutable release tags alone.
- When updating an external action reference, resolve the chosen stable upstream release tag to its full 40-character commit SHA at the time of the change. Keep the originating tag in an inline comment when practical so upgrades stay auditable.
- Verify action usage against the action's current official documentation when changing its major or minor release line. Preserve documented step ordering and supported inputs.
- ORAS setup jobs in workflows must stay on a node24-capable `oras-project/setup-oras` release line and request an ORAS CLI version that the pinned action release explicitly supports.
- ORAS publish commands in release scripts must avoid absolute on-disk layer paths unless path validation is intentionally disabled; prefer running from the asset directory and pushing relative artifact names.
- Helm OCI publication defaults must target the owner-qualified GHCR namespace derived from the active GitHub repository. If a non-GitHub registry layout is needed, override it explicitly with `HELM_REGISTRY_NAMESPACE` rather than relying on an incomplete default path.
- Revaer's default public Helm OCI repository is `oci://ghcr.io/<owner>/charts/revaer`. Keep workflow defaults, install docs, and Artifact Hub registration aligned to that owner-scoped path.
- The shipped `charts/revaer/artifacthub-repo.yml` template is the source of truth for the Artifact Hub repository ID. Release packaging may append ownership data, but it must not duplicate an existing `repositoryID`.
- Trivy SARIF uploads from the reusable image workflow must set an explicit `upload-sarif` category when workflow refactors would otherwise rename the analysis identity. Keep that category aligned with the legacy `ci.yml` build-image matrix key so GitHub code scanning can compare PR scans against `main`.
- Release packaging must preserve Artifact Hub ownership metadata when `ARTIFACTHUB_OWNER_NAME` and `ARTIFACTHUB_OWNER_EMAIL` are provided, even for unsigned packaging paths, because Artifact Hub ownership claim and verified-publisher flows depend on that published owner identity.
- Release packaging should publish an explicit `artifacthub.io/images` chart annotation for the Revaer image so Artifact Hub can index the runtime image and generate package security scans reliably.
- Helm release annotations must be rendered from a file through the portable renderer and covered by `rv helm-annotation-test`. Do not pass multiline YAML through `awk -v` or interpolate annotation bodies into command text.
- Workflows that install Rust toolchains must use the repository's configured toolchain source of truth rather than hard-coded ad hoc channels unless a documented exception is required.
- Workflow build, lint, test, coverage, and release gates must call `just` recipes. Do not reintroduce raw `cargo` pipelines into CI jobs.
- Media fixture acquisition must use `test-fixtures/lock.json` as the immutable
  source, revision, SHA-256, and byte-bound record. Cache keys must include that
  lock. Normal verification must create canonical probes in a private temporary
  tree and diff them against reviewed snapshots without modifying the worktree;
  snapshot replacement is allowed only through the explicit
  `rv update-test-fixture-probes` operator recipe.
- External tools receive literal argument vectors and the injected environment through typed adapters; do not invoke a login shell to execute tasks.
- Python API scenarios validate responses against the committed OpenAPI document. Keep transient generated client output ignored; `uv.lock` is the tooling dependency-resolution source of truth.
- Asset verification must run through `rv check-assets`, which regenerates and compares `crates/revaer-ui/static/nexus`; release validation must also confirm that referenced `/static/...` icon, logo, and DataTables URLs exist in the Trunk release output.
- `pr.yml` is the sole pull-request validation workflow. Keep formatting, lint, test, audit, deny, coverage, E2E, and other verification gates there so pull requests are validated exactly once before merge.
- `pr.yml` must run its release-build validation job on pull requests. Keep post-merge and tag publication in `ci.yml`, but do not hide PR release-build validation behind main/tag-only guards.
- Required CI recipes must install Rust CLI tools at exact reviewed versions with `--locked`; do not let floating registry resolution decide the tool version at check time. Route cargo-audit, cargo-deny, cargo-llvm-cov, cargo-udeps, SQLx CLI, and Trunk through the typed Cargo installer, which replaces missing or mismatched versions while retaining an exact match. Setup must install SQLx CLI `0.8.6` because newer CLI releases can outrun the configured Rust toolchain, and setup must install Trunk `0.21.14` because newer transitive CSS tooling can outrun the configured Rust toolchain. `rv udeps` must pair cargo-udeps `0.1.57` with `nightly-2026-06-13`, run `--workspace --all-targets`, and emit compiler, tool, and command evidence; keep both pins in the PR cache key and update them only through an explicit reviewed change with a successful smoke run.
- The canonical UI E2E gate must install the exact `tests/package-lock.json` graph with lifecycle scripts disabled and run `npm audit --audit-level=info` before generating clients or starting browsers. Every reported npm severity is blocking; refresh the lock or dependency graph instead of adding an audit exception.
- Documentation builds must pin mdBook `0.5.0` to the protocol version used by pinned `mdbook-mermaid 0.17.0`. Browser validation must not pass conflicting `NO_COLOR` and `FORCE_COLOR` settings into Playwright; remove the inherited `NO_COLOR` setting at that process boundary instead of discarding warning output.
- Every pull request must emit `Supply Chain Checks` as a fail-closed aggregate of the independent audit, deny, and unused-dependency jobs. The aggregate must run under `if: always()` and reject every upstream result except `success` through the canonical `just` verifier.
- `pr.yml` must run the media fixture gate through the canonical fixture recipes, publish a nonempty report, and run `rv clean-test-fixtures` under `if: always()` after report upload. Image and release-build jobs must depend on that fixture gate.
- `.github/build-inputs.env` is the single reviewed manifest for exact Rust, uv,
  Python, Dockerfile frontend digest, OCI base-image digest, Alpine release, and
  Alpine package inputs. The Dockerfile, project pins, cache keys, image labels,
  embedded compliance data, and digest-bound evidence must consume or verify it.
  Parse its fixed literal assignments as data; never source or evaluate them.
  Floating tool channels, mutable Dockerfile frontends, unqualified base-image
  tags, and unversioned APK resolution are forbidden.
- Container preparation uses the same installed core `rv` package as the local
  CLI. Copy uv from its official digest-pinned image; let `uv sync --locked
  --no-default-groups --managed-python` own the interpreter and environment.
  BuildKit mounts that environment only while preparing the final stage; do not
  add Python or development tools to the runtime inventory. Preserve native APK
  signature checks, exact package pins, nonroot runtime ownership and exec-form
  health checks. Failed index downloads must fail the build.
- Only container preparation may resolve an explicit source archive without Git
  metadata, and it must validate Revaer's project markers and lockfile. Normal
  `rv` dispatch still requires a real checkout and respects foreign Git boundaries.
- `rv policy` includes `rv media-compliance-guardrails`. Keep all required native
  packages, exact declaration versions, source evidence and image labels. Package
  upgrades require reviewed upstream package evidence and matching SPDX records;
  a successful fixture image is not a complete application compliance result.
- `ci.yml` is the post-merge and tag-release workflow. Limit it to release-artifact, publish, and image-build activity for `main` pushes and release tags; do not duplicate PR validation jobs there.
- Manual release verification belongs in dedicated `workflow_dispatch` workflows, not in `pr.yml`, and should reuse the same `just` entrypoints and pinned third-party actions as the release path they exercise.
- Manual workflows that publish PR-scoped dev Helm artifacts should encode the PR number into the default prerelease version so registry output is traceable back to the reviewed change.
- `workflow_dispatch` string inputs that flow into shell or release commands must be validated and normalized before use. Reject unsafe or malformed values instead of passing them through to `just`, Helm, or release scripts.
- Reusable image workflows may publish PR-scoped dev Helm charts only as an optional post-manifest job. Keep that publish step downstream of the multi-arch manifest job, drive it through `rv helm-package` and `rv helm-publish`, and derive the default prerelease chart version from the caller-provided PR number.
- Release-tag image publication in `ci.yml` must not depend on `release-dev` or any other `main`-only job. Split dev and tag image publishing into separate jobs when their prerequisites differ.
- Stable tag activity in `ci.yml` must exclude prerelease tags consistently at the job boundary, not only in downstream publish jobs. Do not let prerelease tags build stable release artifacts that the later jobs refuse to publish.
- Reusable-workflow caller jobs must not use `secrets: inherit` unless the callee truly requires repository secrets. Prefer the default GitHub token plus explicit job permissions, and pass named secrets only when the callee consumes them.
- Helm chart validation and publication must flow through `rv helm-lint`, `rv helm-package`, and `rv helm-publish`. Do not add ad hoc packaging or registry-push shell blocks to workflows.
- Helm packaging must render multiline annotations without passing embedded newlines through `awk -v`; keep the renderer portable across the BSD and GNU userlands used by local and hosted gates.
- Every workflow job that invokes `just` must install it first through `./.github/actions/setup-revaer`; do not assume any hosted or self-hosted runner image already provides it. This includes each architecture job in the reusable image workflow before the Trivy verifier runs.
- `rv lint` runs the structured Python workflow policy, which rejects unpinned external action refs, direct `${{ inputs.* }}` interpolation inside `run:` blocks, and nonempty Sonar coverage exclusions.
- Workflow guardrails must parse YAML structure rather than search whole files for policy strings. Validate real jobs, steps, conditions, permissions, action references, and `just` invocations, and retain adversarial fixtures proving comments, environment values, descriptions, and unrelated keys cannot satisfy or trigger a rule.
- Sonar property guardrails must parse Java-properties logical keys before applying the exact allowlist. Leading-whitespace forms, escaped keys, continuations, duplicate logical keys, and unknown properties are fail-closed errors; do not return to line-oriented `awk` or `grep` parsing.
- Treat `sonar-project.properties` as the versioned source of truth for Sonar analysis scope. Coverage exclusions must remain explicitly empty so Sonar imports the Rust LCOV, native LLVM coverage, JavaScript LCOV, and Python and measured root bootstrap coverage generated by the canonical coverage tasks instead of publishing zero coverage.
- PR and main-branch Sonar jobs must generate and validate every configured coverage input, including LCOV from the compiled Playwright TypeScript harness that executes the real specs, fixtures, setup, and teardown, retain the native compile database and CXX bridge headers, run the post-scan result verifier, and upload the complete nonempty scanner report. Remove untracked dependency-install directories, generated API clients, UI build output, and Playwright result mirrors before scanning so third-party or generated contents cannot dilute authored analysis; do not add scanner exclusions for them. Do not let the PR workflow run a smaller analysis than the main-branch workflow.
- Published architecture images must be built before compliance evidence is evaluated, independently resolved to an immutable registry digest, inventoried and scanned through that digest-qualified reference, and bound to complete generated evidence. Validate the exact digest before signing, sign the digest and compliance predicate, verify the signed attestation, and block manifest or Helm publication when any stage fails.
- Trivy must emit SARIF with `exit-code: 0` only so the report survives for `if: always()` upload; a separate mandatory verifier must fail on every HIGH or CRITICAL result. Keep deterministic vulnerable-image SARIF regression coverage for this control.
- Sonar-running jobs must prove the checkout is non-shallow and explicitly fetch the reviewed base ref before analysis so pull-request new-code attribution and SCM blame cannot silently degrade.
- `rv sonar-compile-db` must clean the isolated native package build before compilation and fail unless it emits a nonempty `coverage/compile_commands.json`; repeated local or CI invocations must never reuse a cached build-script result after deleting the prior database.
- Release-tooling dependency changes under `release/**`, including JavaScript lockfiles such as `release/package-lock.json`, must stay manifest-scoped, avoid unrelated workflow churn, and update this instruction file in the same change so instruction-drift remains explicit.
- Prerelease Helm assets must be produced during the semantic-release prepare phase so the packaged chart version matches the dev release version exactly. OCI publication must consume those already-packaged assets after the GitHub release assets exist.
- Stable tag releases must package the Helm chart once, attach the `.tgz`, `.prov`, and public key to the GitHub release, and publish that exact packaged chart to the OCI registry. Avoid repackaging between release-asset upload and OCI publication.
- Release and Artifact Hub branding documentation must reference the committed `revaer-logo.svg` asset, preserve its approved purple stylized-R identity, and must not advertise removed raster logo variants.
- Release metadata and packaging use the Python release tasks and typed external tools. Keep side effects inside explicit publication stages and preserve dry-run verification.
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
- Ephemeral test credentials must never be reused as application secrets, committed runtime credentials, or user-facing examples.
- `rv cov` must pass its resolved test database URL to both database startup and the coverage process in the same task context so database-backed tests cannot silently lose their configured endpoint. It must instrument linked first-party C/C++ through a Clang version compatible with rustc's LLVM tools, retain Rust-only package thresholds, emit both Rust LCOV and native LLVM reports, with the scanner workflow separately running `rv script-coverage`. Script coverage uses the exact source-built and archive-hash-verified `kcov` revision pinned in the canonical setup action for the retained root Bash bootstrap, and converts those real execution records to Sonar's generic format; omitted or unexecuted executable shell lines must remain uncovered. Disposable test databases must be dropped with forced session cleanup, and local-only host fallback must remain bounded to `localhost` and `127.0.0.1` inputs.
- Sonar coverage jobs must fetch complete Git history and exercise authored Rust and native C/C++ code in one `rv cov` run so the scanner can resolve the main-branch baseline and import both retained reports.
- PostgreSQL service containers used by pull-request, Sonar, and managed local validation must reserve 1 GiB of shared memory so concurrent isolated-schema migrations cannot exhaust Docker's 64 MiB default. `rv db-start` must recreate its named managed container when the configured allocation is smaller.
- Do not log secrets or secret-like values. Mask or omit them.
- Keep Helm registry credentials (`HELM_API_KEY_ID`, `HELM_API_KEY_SECRET`) separate from chart-signing material (`HELM_GPG_PRIVATE`, `HELM_GPG_PUBLIC`). Publishing jobs may use registry credentials only when consuming an already-packaged chart artifact.
- GHCR chart publication on GitHub-hosted runners should prefer the job-scoped `GITHUB_TOKEN` plus explicit `packages: write` over long-lived custom registry secrets. Keep `HELM_API_KEY_*` only for non-GitHub or local override paths.

# Drift Control

- Any change to a workflow, release script, setup action, the `rv` registry, or `sonar-project.properties` must review the matching instruction file in the same change.
- Revaer enforces that rule mechanically with `rv instruction-drift`, backed by the Python policy tasks. Keep the mapping in those tasks aligned with this file and `AGENTS.md`.
- Keep `scripts/workflow-guardrails.sh` aligned with the live workflow policy when GitHub Actions pinning or shell-safety rules change.
- `pr.yml` must pass explicit base/head SHAs into `rv instruction-drift` so pull requests are checked against the real reviewed diff, not an incidental worktree state.
- Drift coverage for actions and release assets is recursive. Changes under `.github/actions/**`, `.github/workflows/**`, and `release/**` must keep matching the devops instruction update rule.
- Reusable workflows that publish images must preserve `packages: write` on the caller job because the callee cannot elevate a more restrictive token.
- Reusable-workflow caller jobs must define one merged `permissions` map. Do not duplicate the `permissions` key in a job to append scopes later; GitHub Actions rejects the workflow before execution.
- Keep the Sonar PR and main gates blocking. Scanner-side waiting, retained report/task evidence, and API verification must all evaluate the same single analysis; branch decoration alone is not sufficient proof.

- The Python workflow contract must preserve canonical media conversion,
  unconditional required-report upload, cleanup after upload, and downstream
  image/release dependencies. `rv tooling-check` contains the former standalone
  stack-contract and advisory-exception regression suites.

- When the chart exposes media compliance bindings, `rv helm-lint` must run
  `rv compliance-chart-test` against that checkout's actual chart before
  packaging. The reference fixture in tooling tests is not integration evidence.
  Preserve independent schema/template validation and empty invalid manifests.

- The PR media job invokes Python tasks for fixture cache identities, acquisition,
  generation, conversion, report publishing and cleanup. Preserve its prerequisite
  jobs, step order, unconditional report/upload/cleanup and missing-report failure.
  Cache keys consume `tool_version` and `fixtures` from `rv fixture-cache-key`;
  do not retain shell-script hashes after migrating the fixture implementation.

- Helm packaging uses strict lint: warnings must prevent package publication.
  Negative fixtures must pass ordinary lint to distinguish warning enforcement
  from unrelated syntax or schema errors.

- Media chart lint inputs are ephemeral: keep synthetic digest, architecture and
  compliance identities outside the packaged chart. Verify source/default bytes
  remain unchanged when packaging, including the current media chart.

- Chart annotation rendering requires exactly one standalone release-marker
  comment. Reject substring and duplicate matches, preserve neighboring authored
  YAML, and retain existing package output on template validation failure.

- Preserve Helm lint prerequisite order: annotation tests, applicable media
  compliance tests, package tests, then unsigned packaging. Package regression
  tests exercise the package operation directly to prevent recursive test runs.

- Documentation composition retains install/build/index ordering. Use pinned
  Cargo tools and Mermaid's own integration installer; link checking installs
  the pinned Lychee tool before running the existing verbose/no-progress check.

- Preserve standalone license reporting's pinned cargo-deny installation
  prerequisite. A generated license inventory does not replace audit or deny.

- The shared build manifest's native PostgreSQL debugger block is optional but
  all-or-nothing. Validate its exact eleven fields, immutable image ID, artifact
  hashes, package pins and credential-free HTTPS source URL. Container tasks
  must reject unknown keys and invalid debugger pins even though they do not
  install the debugger. Do not replace this contract with prefix-based ignores.

## Tooling foundation workflow cutover

The ordinary PR gates (instruction drift, formatting, lint, check, audit, deny,
unused dependencies, supply-chain aggregation and release build) invoke the
locked `rv` CLI through the shared uv setup action. Select native tools through
its supported inputs; retain each gate, dependency, condition and failure result.
Setup has a 20-minute bound, and each job retains an explicit bounded timeout.

All PR jobs now use the shared setup inputs and locked Python executor. The
scanner job owns coverage collection and analysis in one checkout, including
measured root bootstrap coverage. Database URLs must match the isolated service.
UI shards retain raw browser records, selection manifests and completion
summaries; aggregation verifies those summaries before combined route coverage.

Chart regression gates need both Helm and ORAS. Coverage jobs run the complete
Python integration suite and therefore select all pinned Cargo tools plus Helm,
ORAS, Trivy and the three browsers; a Python-only environment is insufficient.
