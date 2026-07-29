# CI Postgres Credential Coherence

- Status: Accepted
- Date: 2026-08-11
- Context:
  - PR coverage, feature-matrix, UI E2E, and standalone Sonar coverage jobs create isolated Postgres services with run-derived passwords.
  - Their client URLs still came from `REVAER_DATABASE_URL`, so a repository variable could carry a different password than the service container and fail migrations before any coverage was produced.
- Decision:
  - Construct each database client URL from the same workflow-run expression used by its Postgres service.
  - Apply the contract to `REVAER_TEST_DATABASE_URL`, `DATABASE_URL`, and E2E admin URLs in every job with a Postgres service.
  - Add a structural workflow guardrail that derives the expected URL from the service user, password, and database and rejects any mismatch.
  - Keep coverage inputs, analyzers, source scope, thresholds, and Sonar quality-gate criteria unchanged.
- Consequences:
  - Database migrations and tests authenticate against the exact disposable service declared by their job.
  - Repository variables can no longer silently redirect in-job database tests to a different credential or endpoint.
- Follow-up:
  - Require coverage, feature-matrix, and all UI shards to complete before accepting the PR workflow.
  - Require the standalone Sonar workflow to publish nonempty Rust, JavaScript, script, and native analysis inputs.

## Task Record

- Motivation:
  - Make the newly scheduled coverage path produce real reports instead of failing before instrumentation.
- Design notes:
  - The credential remains ephemeral and derived from `run_id`, `run_attempt`, and `job`; no literal password is committed.
  - The guardrail validates semantic equality after strict YAML parsing rather than matching workflow text.
- Test coverage summary:
  - Run the workflow guardrail regression suite with matching and drifting Postgres fixtures.
  - Run `just policy`, `just instruction-drift`, and Actionlint.
  - Verify the complete GitHub PR and standalone Sonar coverage workflows after restacking.
- Observability updates:
  - No runtime observability surface changes. Authentication and missing-artifact failures remain fail-closed CI evidence.
- Status-doc validation:
  - Rechecked the media status and Sonar configuration documents; no product-status claim changes are required.
- Risk & rollback plan:
  - The risk is malformed workflow interpolation. Strict YAML parsing, Actionlint, and live service migrations validate it; rollback is a single-commit revert.
- Dependency rationale:
  - No dependency was added or changed.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/sonarqube_mcp.instructions.md`.
  - Drift was found in the missing service-to-client credential-coherence rule; the DevOps instruction and structural guardrail now state and enforce it.
