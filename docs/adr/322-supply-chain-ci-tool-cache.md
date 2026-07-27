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
- Validate and fetch the actual pull request base branch before Sonar analysis so normally chained PRs retain strict new-code SCM context instead of skipping the scan or assuming `main`.
- Reject every npm advisory severity in both lockfiles and every Cargo advisory warning.
- Keep `.secignore` and `deny.toml` advisory ignores empty. Remove vulnerable crates from the resolved lock graph by upgrading or replacing dependencies.
- Keep duplicate-crate allowances exact-version scoped and remove them when the lock graph no longer needs them.
- Replace semantic-release's unused npm publish plugin with a local no-op package so release tooling does not pull a vulnerable bundled npm CLI into the audited lockfile.

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
  - PR Sonar setup validates the pull request base ref with Git's branch-name parser, fetches that exact base from origin, and fails before scanning if the ref is missing or invalid.
  - PR UI E2E sets a 900-second readiness budget so cold CI runner setup does not fail before browser tests can exercise the app.
- Test coverage summary:
  - `just audit`
  - `just deny`
  - `just udeps`
  - `just policy`
  - `just ui-e2e` browser setup on CI runners through PR UI E2E shards.
  - `just ci`
  - Remote Supply Chain Checks and Trivy jobs.
  - Remote SonarQube PR checks on normally chained branches.
  - Remote UI E2E shard startup on cold GitHub-hosted runners.
  - Sonar JavaScript/TypeScript LCOV is generated from executed Playwright and
    release-tooling code, merged into `coverage/js-lcov.info`, and verified for
    source and line records before every scan.
- Observability updates:
  - CI exposes tool installation, cache restoration, advisory refresh, and each canonical gate as separate log steps.
- Risk and rollback plan:
  - Delete a broken cache entry or bump its version key; do not bypass a check.
  - Revert a dependency upgrade only with an alternative lock graph that remains advisory-clean.
- Dependency rationale:
  - No runtime dependency was added. Tool versions are pinned CI dependencies; lockfile upgrades remove vulnerable transitive packages.
  - `vendor/semantic-release-npm-stub` is a release-tooling-only replacement for the unused npm publish plugin that semantic-release depends on by default; the active release config publishes GitHub assets through `@semantic-release/github`, not npm.
  - `c8`, `typescript`, and `@types/node` are dev-only test harness dependencies used to compile the Playwright TypeScript harness with source maps and emit Sonar-compatible JavaScript/TypeScript LCOV from executed CI code.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift found in duplicate cache setup and advisory exception language; the workflow and instructions now require one canonical, zero-ignore path.
