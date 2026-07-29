# Sonar pull-request result scope

- Status: Accepted
- Date: 2026-08-12
- Context:
  - Pull-request analysis submitted complete scanner evidence and positive coverage, but the post-scan verifier queried the project without a pull-request scope.
  - Sonar project APIs default to main-branch measures when no branch or pull-request selector is supplied.
- Decision:
  - Pass the exact GitHub pull-request number to the Sonar result verifier.
  - Enforce that workflow wiring in the repository workflow guardrails.
  - Remove the empty shallow marker left by the checkout action after proving the repository has complete history, and fail if any shallow marker remains before scanning.
  - Use an analyzer-compatible keyword-initialized value carrier in the Ruby workflow parser so Sonar can publish valid symbol ranges without warnings.
  - Keep the verifier's existing fail-closed checks for positive coverage, lines to cover, quality-gate conditions, unresolved new issues, and unreviewed new hotspots.
  - Extend the canonical audit gate to run npm audit at every severity for both committed JavaScript lockfiles, and update vulnerable `js-yaml` and `undici` resolutions.
- Consequences:
  - PR verification evaluates the same analysis submitted by the scanner instead of unrelated main-branch state.
  - Removing or renaming the scope variable fails local policy and CI lint before analysis runs.
  - Both PR and main-branch analysis retain complete SCM blame instead of trusting Git's shallow-state result alone.
- Follow-up:
  - Confirm the rerun publishes positive coverage and completes every downstream coverage, image, and release gate.

## Task Record

- Motivation:
  - Restore the mandatory Sonar coverage gate after PR 90 retained a valid scanner report but failed result verification against the wrong analysis scope.
- Design notes:
  - The change does not relax any Sonar criterion or exclusion. It makes every existing criterion apply to the intended PR analysis.
- Test coverage summary:
  - Added fixture-driven workflow-policy coverage for the exact PR-number environment binding.
  - Reused the canonical workflow fixture runner so the policy test itself contributes complete executable-line coverage instead of duplicating unexecuted assertion paths.
  - Extended structural policy coverage to require empty-marker cleanup and a fail-closed remaining-marker check in every Sonar-running workflow.
  - Re-ran workflow guardrails, instruction drift, formatting, and the focused policy suite locally.
  - Verified zero npm vulnerabilities in `tests/package-lock.json` and `release/package-lock.json` under NVM Node 24.14.1.
- Observability updates:
  - Existing scanner evidence retention remains unchanged and provides the task and dashboard identifiers used to diagnose scope failures.
- Status-doc validation:
  - Reviewed the Sonar workflow documentation and quality-gate instructions; no user-facing capability status changed.
- Risk & rollback plan:
  - The only operational risk is a malformed PR selector causing a fail-closed API query. Roll back the commit if GitHub changes the pull-request event contract.
- Dependency rationale:
  - No dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - The missing PR-scope invariant was added; no other contradictions or stale references were found.
