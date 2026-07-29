# Supply Chain CI And Advisory Remediation

- Status: Accepted
- Date: 2026-07-18

## Context

- PR validation repeatedly installed supply-chain tools and downloaded advisory data, delaying required checks and increasing runner pressure.
- The dependency graph also contained RustSec findings that had to be removed from the lock graph rather than ignored.
- This record consolidates the CI tool-cache decision and the RustSec lock-graph remediation previously recorded separately in ADR 357.

## Decisions

- Run Cargo and npm supply-chain validation through the canonical `just audit`, `just deny`, and `just udeps` recipes.
- Use one PR supply-chain job with version-keyed installed-tool caches and a separately keyed advisory-database cache.
- Install pinned prebuilt tool releases through the pinned installer action and fail if the required versions are unavailable.
- Do not restore the multi-gigabyte shared Cargo/sccache artifact for first-wave policy, lint, Helm, supply-chain, or other jobs that do not need compiled artifacts.
- Keep the media conversion fixture job present on every PR, but publish an explicit not-applicable report in early stack layers before the fixture integration test target exists.
- Install Playwright's `chromium-headless-shell` alongside `chromium` whenever Chromium UI tests are requested, because headless UI tests can launch the shell executable even when CI also asks for a system browser channel. A partial browser cache must fail during installation rather than at test start.
- Keep PR UI E2E readiness bounded but long enough for cold Rust and Trunk setup to complete before declaring the UI server unavailable.
- Validate and fetch the actual pull request base branch before Sonar analysis, and fail if the fetched ref does not resolve to the pull request's reviewed base SHA, so manually stacked PRs retain strict new-code SCM context instead of skipping the scan or assuming `main`.
- Retry factory reset on PostgreSQL deadlock SQLSTATE `40P01` so reset can safely converge when active background jobs briefly hold conflicting locks during PR E2E setup.
- Reject every npm advisory severity in both lockfiles and every Cargo advisory warning.
- Keep `.secignore` and `deny.toml` advisory ignores empty. Remove vulnerable crates from the resolved lock graph by upgrading or replacing dependencies.
- Keep duplicate-crate allowances exact-version scoped and remove them when the lock graph no longer needs them.
- Replace semantic-release's unused npm publish plugin with a local no-op package so release tooling does not pull a vulnerable bundled npm CLI into the audited lockfile.
- Parse workflow and composite-action YAML semantically, including duplicate-key detection, before enforcing jobs, steps, conditions, permissions, immutable action pins, direct-input shell boundaries, and known `just` recipes.
- Parse `sonar-project.properties` as Java properties and reject noncanonical leading whitespace, escaped keys, continuations, duplicate logical keys, unknown keys, nonempty scope filters, and analyzer or SCM deactivation.
- Build publication images before resolving their immutable OCI digest. Inventory and scan the digest-qualified reference, verify that HIGH and CRITICAL SARIF is empty, generate evidence bound to that exact digest, validate both caller authorization and generated evidence against the digest, and cryptographically sign and verify the resulting attestation before manifest publication.
- Preserve Trivy SARIF with an `if: always()` upload while making the separate HIGH/CRITICAL verifier blocking for pull-request verification and publication.
- Install the canonical setup action before every reusable image architecture job invokes `just`; runner images are not a tool contract.
- Generate real authored Bash line coverage with `kcov`, Ruby line and branch coverage with the standard `Coverage` API, and convert both to Sonar's generic coverage schema while retaining uncovered executable lines.
- Keep PR Sonar coverage, native inputs, post-scan verification, and scanner evidence identical to the main-branch analysis path.
- Use private unpredictable temporary storage for nonfree-build-token matches and remove it on every exit path.
- Keep exact Rust, Node.js, npm, just, OCI base-image digests, Alpine release, and Alpine package versions in `.github/build-inputs.env`; make local, CI, image, cache, label, and compliance-evidence paths consume or verify that one manifest.
- Use digest-only OCI base references while separately verifying the expected Rust and Alpine versions inside the selected images, so immutable identity and runtime-version intent are both enforced without ambiguous tag-plus-digest references.
- Derive the Rust musl target from BuildKit's per-platform architecture and reject any explicit target that does not match it, so local and multi-architecture image builds cannot compile for an unset or mismatched target.
- Compile and execute the complete Playwright TypeScript harness under `c8` in every required E2E path, merge its LCOV with release-tooling LCOV, and remove untracked npm dependency trees and generated source mirrors only after coverage generation and before Sonar indexing.

## Consequences

- Required supply-chain checks start promptly and remain independent of large build caches.
- Advisory data can refresh independently from tool binaries.
- Lockfile changes are intentional evidence of remediation, not warning suppression.
- A new advisory blocks the PR until the dependency graph is fixed or the operator explicitly approves a time-bounded exception with an ADR and guardrail update.

## Task Record

- Motivation:
  - Make mandatory supply-chain checks fast enough to run consistently while preserving zero-ignore advisory policy.
- Design notes:
  - Cache keys include tool versions and platform identity.
  - The Justfile remains the only build/test/lint/audit command surface.
  - The media conversion job detects the fixture test target before fixture preparation so lower stack layers have visible check evidence without claiming absent tests ran.
  - `just ui-e2e` expands Chromium installs to include `chromium-headless-shell` and uses Playwright's installer for browser targets plus operating-system dependencies in CI.
  - PR Sonar setup validates the pull request base ref with Git's branch-name parser, fetches that base from origin, and fails before scanning if the ref is missing, invalid, or does not resolve to `github.event.pull_request.base.sha`.
  - PR UI E2E sets a 900-second readiness budget so cold CI runner setup does not fail before browser tests can exercise the app.
  - Factory reset retries only database deadlocks with a bounded attempt count; non-deadlock reset failures still propagate immediately.
  - YAML checks inspect parsed nodes, so policy-looking text in comments, descriptions, environment values, or unrelated keys cannot satisfy a rule. Java-properties checks decode logical keys before applying the strict allowlist.
  - Quality-policy entry recipes live in `just/quality.just`, imported by the root Justfile. Workflow policy is split between an 11-line launcher and the tested semantic parser so both command surfaces remain reviewable.
  - Architecture image compliance bundles cryptographically include the digest-qualified image reference, exact Trivy SPDX inventory hash, declared source inventory hash, source-compliance hash, notices hash, revision, and release-gate state.
  - The fixed `/tmp/revaer-media-nonfree.matches` pathname was removed; `mktemp` creates a mode-0600 match file under the caller's temporary directory and a trap removes it.
  - The coverage profile installs `kcov` 43 from source commit `a39874f938ce13f7a65f253120d1ec946b349ffe`, verifies archive SHA-256 `dac01569171979477b500924be264d2a1bc649dae6010536228cbb319344d516`, and caches only that exact installed prefix because Ubuntu 24.04 does not package `kcov`. `just cov` executes the same complete policy suite under `kcov` and a Ruby coverage bootstrap, then emits `coverage/script-coverage.xml` with covered and uncovered line records plus Ruby branch records.
  - The exact-input manifest pins Rust 1.96.0, Node.js 24.14.1, npm 11.12.1, just 1.49.0, immutable multi-platform Rust/Alpine image digests, Alpine 3.23.5, and every builder/runtime APK version. Just 1.49.0 is the newest checksum-backed release in the pinned `taiki-e/install-action` manifest; newer releases fail closed as unsupported with `checksum: true` and `fallback: none`. Docker verifies requested build arguments against the manifest and embeds it in the runtime image; compliance bundles hash and validate the same file. The declared runtime inventory records verified arm64 and amd64 APK SHA-256 values for updated runtime packages.
  - PR Sonar now generates JavaScript LCOV, validates every configured report and native input, verifies the published result, and retains the scanner report. Reusable image architecture jobs install `just` before the blocking Trivy SARIF verifier.
  - The npm self-install remains a literal exact version so static workflow analysis can prove it is locked; the media-compliance guardrail requires that literal to match `NPM_VERSION` in `.github/build-inputs.env`.
  - Container builds resolve `TARGETARCH` to the corresponding supported musl triple and fail closed when a caller supplies a different `RUST_TARGET`; this keeps the same Dockerfile valid for local and matrix-driven architecture builds.
  - Playwright runs the compiled specs, fixtures, global setup, and global teardown under one `c8` process and emits shard-aware, source-mapped LCOV from the real harness. Sonar input verification remains a canonical `just` recipe, and the scanner runs only after npm dependency installs and the generated API schema mirror are removed from the checkout without changing source scope or adding exclusions.
- Test coverage summary:
  - `just audit`
  - `just deny`
  - `just udeps`
  - `just policy`
  - `just ui-e2e` browser setup on CI runners through PR UI E2E shards.
  - `just ci`
  - `just workflow-guardrails-test`
  - `just trivy-sarif-policy-test`
  - `just image-compliance-test`
  - `just media-compliance-guardrails-test`
  - Remote Supply Chain Checks and Trivy jobs.
  - Remote SonarQube PR checks on normally chained branches.
  - Remote UI E2E shard startup on cold GitHub-hosted runners.
  - Remote UI E2E exposed a factory-reset deadlock against RSS backfill; the reset path now retries SQLSTATE `40P01`.
  - Sonar JavaScript/TypeScript LCOV is generated from executed Playwright and
    release-tooling code, merged into `coverage/js-lcov.info`, and verified for
    source and line records before every scan.
  - The PR and main Sonar paths execute the same compiled Playwright coverage producer before merging JavaScript LCOV; sharded PR E2E jobs retain line coverage separately from route and operation coverage so artifact path aggregation remains stable.
  - Coverage-only TypeScript compilation embeds source text and source maps so Playwright worker V8 ranges resolve to actual authored lines. Input verification rejects collapsed reports by requiring at least 1,000 JavaScript/TypeScript line records with both covered and uncovered entries; the complete local suite reported 3,835 of 4,379 Playwright harness lines covered.
  - `just script-coverage` generates and validates nonempty Bash/Ruby generic coverage with both covered and uncovered records; `scripts/tests/script-coverage-test.sh` verifies native `kcov` generic-report import, Ruby line/branch merging, bootstrap output, and rejection of ambiguous collector reports. `scripts/tests/sonar-result-guardrails-test.sh` exercises positive metrics, pull-request scoping, and every post-scan rejection condition against a deterministic local API fixture.
  - `sonar verify --file ... --project VannaDii_Revaer` was attempted for every changed shell/Ruby file; SonarCloud returned HTTP 403 because Agentic Analysis is not enabled for the organization, so the repository scanner remains the authoritative remote verification path.
- Observability updates:
  - CI exposes tool installation, cache restoration, advisory refresh, and each canonical gate as separate log steps.
  - Image jobs retain Trivy SARIF even when findings fail the job and upload verified digest-bound compliance evidence per architecture when publication is enabled.
- Risk and rollback plan:
  - Delete a broken cache entry or bump its version key; do not bypass a check.
  - Revert a dependency upgrade only with an alternative lock graph that remains advisory-clean.
  - Do not roll back to text-search workflow checks, a mutable image tag, pre-build final evidence, nonblocking vulnerability findings, or predictable temporary files. If attestation tooling fails, stop publication and repair the evidence chain.
- Dependency rationale:
  - No runtime dependency was added. Tool versions are pinned CI dependencies; lockfile upgrades remove vulnerable transitive packages.
  - Base-image and APK pins are build inputs, not application dependencies. Updates require digest/version resolution, both target-architecture smoke builds, inventory synchronization, and review of this manifest.
  - `vendor/semantic-release-npm-stub` is a release-tooling-only replacement for the unused npm publish plugin that semantic-release depends on by default; the active release config publishes GitHub assets through `@semantic-release/github`, not npm.
  - A local `flume` patch was rejected because the registry version is advisory-clean and duplicate-clean, while committing the dependency source would add uncovered third-party code to Sonar's main-code scope.
  - `c8`, `typescript`, and `@types/node` are dev-only test harness dependencies used to compile the Playwright TypeScript harness with source maps and emit Sonar-compatible JavaScript/TypeScript LCOV from executed CI code.
  - `kcov` is a development-only coverage tool. CI builds the exact pinned commit and verifies the source archive hash; local environments may install the same version through their package manager. It is required because Sonar has no native Bash execution collector; no application or runtime dependency was added.
  - Ruby's standard Psych YAML parser is used by local and hosted policy jobs; no application or runtime dependency was added.
  - Ruby's standard `Coverage`, `JSON`, and `REXML` APIs collect branch-aware execution data and emit Sonar's generic XML without adding a gem dependency.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/revaer-ui.instructions.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift found in line-oriented workflow/Sonar parsing, legacy Sonar exclusions, pre-build compliance evidence, nonblocking Trivy findings, predictable temporary storage, a reusable image job that assumed `just` existed, a PR Sonar path that omitted configured coverage and evidence, and scanner indexing of untracked npm dependencies. The policy, workflow, Sonar scope, coverage collectors, and regression tests now fail closed on those paths.
