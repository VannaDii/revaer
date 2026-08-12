# CI Postgres Credential Coherence

- Status: Accepted
- Date: 2026-08-12
- Context:
  - PR coverage, feature-matrix, and UI E2E jobs create isolated Postgres services with run-derived passwords.
  - Their client URLs came from `REVAER_DATABASE_URL`, so a repository variable could carry a different password than the service container and fail migrations before tests or coverage ran.
- Decision:
  - Construct each database client URL from the same workflow-run expression used by its Postgres service.
  - Apply the contract to `REVAER_TEST_DATABASE_URL`, `DATABASE_URL`, and E2E admin URLs in every job with a Postgres service.
  - Enforce semantic equality with the YAML-aware workflow guardrail.
- Consequences:
  - Database migrations and tests authenticate against the exact disposable service declared by their job.
  - Repository variables cannot silently redirect in-job database tests to a different credential or endpoint.
- Follow-up:
  - Require coverage, feature-matrix, and all UI shards to complete before accepting the PR workflow.

## Task Record

- Motivation:
  - Make every stacked PR run its database-backed gates instead of failing before test execution.
- Design notes:
  - Credentials remain ephemeral and derive from `run_id`, `run_attempt`, and `job`; no literal password is committed.
  - The guardrail validates parsed YAML values rather than matching workflow text.
- Test coverage summary:
  - Added matching and drifting Postgres workflow fixtures plus adversarial coverage for missing environment mappings, incomplete service credentials, and a drifting admin URL.
  - Ran `just policy`, `just instruction-drift`, and formatting validation.
- Observability updates:
  - No runtime surface changes. Authentication and missing-artifact failures remain fail-closed CI evidence.
- Status-doc validation:
  - Rechecked the media status and Sonar configuration documents; no product-status claim changes are required.
- Risk & rollback plan:
  - The risk is malformed workflow interpolation. Strict YAML parsing, Actionlint, and live service migrations validate it; rollback is a single-commit revert.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift was found in the missing service-to-client credential-coherence rule; the DevOps instruction and structural guardrail now state and enforce it.
