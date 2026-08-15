# Tooling module boundaries

- Status: Accepted
- Date: 2026-08-15
- Operator approval: Recommendation approved wholesale by the operator on
  2026-08-15: "I approve the ADRs as they are now and I'm resuming your goal."
- Context:
  - The operator review on PR 73 requires the Justfile and workflow-policy
    implementation to be decomposed into tested helpers so their behavior is
    reviewable.
  - The integrated leaf currently has a 925-line `justfile` and a 594-line
    `scripts/workflow-structure-guardrails.rb`. Extracting only quality recipes
    leaves both command ownership and policy parsing concentrated in large files.
  - Recipe names and `just` remain the public command contract. Workflow and
    Sonar policy must continue to fail closed, parse structured inputs, and run
    through that contract.
- Decision:
  - Recommended option: retain one root `justfile` as an import-only command
    index plus shared settings, and split recipes by owned workflow into
    `just/quality.just`, `just/database.just`, `just/ui.just`,
    `just/media.just`, `just/release.just`, `just/images.just`, and
    `just/docs.just`.
  - Recommended option: retain `scripts/workflow-guardrails.sh` as the shell
    entrypoint and split Ruby policy code under `scripts/workflow_guardrails/`
    into input loading, GitHub Actions structure, Sonar properties, required
    checks, and diagnostics. The entrypoint composes those modules and owns the
    process exit status.
  - Keep each recipe and policy rule under exactly one domain owner. Cross-domain
    recipes invoke public recipe names instead of duplicating command bodies.
  - Preserve all current recipe names, environment contracts, action pins,
    scanner criteria, and failure behavior during the move. Add no compatibility
    aliases unless a checked-in caller still requires one.
  - Alternative considered: split only `quality.just` and keep one policy parser.
    Rejected because it leaves the review's two oversized ownership surfaces.
  - Alternative considered: replace `just` and Ruby with another task runner or
    policy framework. Rejected because it changes the repository command contract
    and adds dependencies without solving a product requirement.
- Consequences:
  - Reviewers can evaluate one operational domain or policy parser at a time.
  - Module moves create short-term diff churn and can expose hidden recipe-order
    or environment coupling.
  - The split is structural only; any behavior change requires its own task
    record and tests.
- Follow-up:
  - Move one domain at a time and keep every intermediate stack PR runnable.
  - Add parser-level fixtures for duplicate YAML keys, deceptive comments and
    values, Java-properties continuations, duplicate logical keys, unknown Sonar
    properties, and all required-check contexts.
  - Run focused guardrails, `just ci`, and `just ui-e2e` after the complete split.

## Task Record

- Motivation:
  - Resolve the remaining PR 73 review requirement without inventing module
    boundaries outside operator review.
- Design notes:
  - The recommendation follows the repository's existing command names and
    standard-library Ruby parser approach; it does not move product logic.
- Test coverage summary:
  - Proposal only. Implementation requires unchanged-recipe contract tests,
    policy parser fixtures, full CI, and UI E2E.
- Observability updates:
  - Preserve existing step and diagnostic names so CI history remains legible.
- Status-doc validation:
  - Updated the ADR index and documentation summary. Product status is unchanged.
- Risk & rollback plan:
  - Revert an incomplete module-move PR before merging it. Do not roll back by
    weakening a policy rule or bypassing a canonical recipe.
- Dependency rationale:
  - No dependency is proposed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, the
    Justfile contract, ADR 322, and the current PR 73 review summary.
  - Drift found: ADR 322's partial extraction does not fully satisfy the active
    review or the current file-size policy.
