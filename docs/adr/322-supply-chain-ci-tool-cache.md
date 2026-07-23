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
- Reject every npm advisory severity in both lockfiles and every Cargo advisory warning.
- Keep `.secignore` and `deny.toml` advisory ignores empty. Remove vulnerable crates from the resolved lock graph by upgrading or replacing dependencies.
- Keep duplicate-crate allowances exact-version scoped and remove them when the lock graph no longer needs them.

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
- Test coverage summary:
  - `just audit`
  - `just deny`
  - `just udeps`
  - `just policy`
  - `just ci`
  - Remote Supply Chain Checks and Trivy jobs.
- Observability updates:
  - CI exposes tool installation, cache restoration, advisory refresh, and each canonical gate as separate log steps.
- Risk and rollback plan:
  - Delete a broken cache entry or bump its version key; do not bypass a check.
  - Revert a dependency upgrade only with an alternative lock graph that remains advisory-clean.
- Dependency rationale:
  - No runtime dependency was added. Tool versions are pinned CI dependencies; lockfile upgrades remove vulnerable transitive packages.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift found in duplicate cache setup and advisory exception language; the workflow and instructions now require one canonical, zero-ignore path.
