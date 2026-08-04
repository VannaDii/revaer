# Npm audit refresh

- Status: Accepted
- Date: 2026-08-04
- Context:
  - PR #133's `Supply Chain Checks` job failed in `just audit` after the advisory database reported high-severity npm findings in the test package.
  - `brace-expansion` `4.0.0 - 5.0.8` is vulnerable to `GHSA-rgw5-rvv9-x895`, and `fast-uri` `3.0.0 - 3.1.4` is vulnerable to `GHSA-7p8r-x3mc-p8w7`.
  - The release package also carries audited transitive release tooling and must remain fail-closed instead of relying on later stack layers for lockfile hygiene.
  - Repository policy forbids advisory ignores by default; configured audit severities are mandatory fixes.
- Decision:
  - Update the tests package overrides to `brace-expansion` `5.0.9` and `fast-uri` `3.1.5`.
  - Update the release package override to `undici` `7.29.0`.
  - Regenerate only `tests/package-lock.json` and `release/package-lock.json` with NVM-selected Node `v24.14.1` and npm `11.12.1`.
  - Keep the audit gate strict and add no npm audit exception.
  - Alternatives considered:
    - Ignoring the advisories was rejected because supply-chain findings at configured severities must be fixed.
    - Leaving the fix in a later stack PR was rejected because PR #133 is the first failing layer, causing downstream checks to skip.
- Consequences:
  - Positive outcomes:
    - `just audit` passes for RustSec plus both npm package scopes.
    - Lower stack PRs no longer inherit a known supply-chain failure before their own tests run.
  - Risks or trade-offs:
    - The overrides depend on upstream patch releases preserving the transitive public API expected by their consumers.
- Follow-up:
  - Continue treating future npm audit findings as required fixes unless the operator explicitly approves a time-bounded exception.

## Task Record

- Motivation:
  - Restore PR #133's supply-chain gate without weakening audit criteria.
- Design notes:
  - The change is limited to package manifests and lockfiles for the existing test and release toolchains.
  - No source dependency surface was added, and no dependency scanner criteria were relaxed.
- Test coverage summary:
  - `source ~/.nvm/nvm.sh && nvm use v24.14.1 && npm --prefix tests install --package-lock-only`
  - `source ~/.nvm/nvm.sh && nvm use v24.14.1 && npm --prefix release install --package-lock-only`
  - `source ~/.nvm/nvm.sh && nvm use v24.14.1 && just audit`
- Observability updates:
  - None.
- Risk and rollback plan:
  - Roll back by reverting this ADR plus the package manifest and lockfile updates.
  - The rollback risk is restoring a known high-severity npm audit failure.
- Dependency rationale:
  - No new dependency. Existing transitive dependencies are refreshed to patched versions already allowed by the consuming graph.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - No policy drift, scanner relaxation, or criteria exception was introduced.
