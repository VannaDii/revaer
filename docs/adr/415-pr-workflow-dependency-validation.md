# PR Workflow Dependency Validation

- Status: Accepted
- Date: 2026-08-11
- Context:
  - GitHub rejected the pull-request workflow before scheduling jobs because `api-e2e` depended on a deleted `supply-chain` job.
  - The local structural workflow guardrail validated job and step shapes but did not validate that `needs` targets existed.
- Decision:
  - Extend the strict YAML guardrail to reject empty, malformed, or unknown job dependencies before later workflow changes can be accepted.
  - Add an adversarial fixture proving a nonexistent dependency fails local policy validation.
  - Alternatives considered:
    - Remove the dependency: rejected because API E2E should not consume runner time after a supply-chain failure.
    - Rely only on GitHub validation: rejected because invalid workflows produce no normal PR checks and must be caught before push.
- Consequences:
  - The immediately following phase-history layer must replace its stale `supply-chain` dependency with the existing `audit`, `deny`, and `udeps` jobs before it can pass policy or schedule on GitHub.
  - Future job renames or deletions fail `just policy` when downstream dependencies are not updated.
- Follow-up:
  - Correct the stale dependency in the phase-history layer and confirm a pull-request event schedules every required job.

## Task Record

- Motivation:
  - Close the local validation gap before repairing the downstream unschedulable workflow, ensuring the failure cannot recur silently.
- Design notes:
  - `needs` accepts either one scalar job ID or a sequence. Every value must be a nonempty scalar and exist in the workflow's job map.
  - Dependency cycles remain GitHub-schema concerns; this repair addresses the observed missing-reference failure without adding a second workflow engine.
- Test coverage summary:
  - Added an unknown-job fixture to the workflow guardrail regression suite.
  - `actionlint` and the repository policy suite validate the corrected workflow.
- Observability updates:
  - No runtime telemetry changes. Invalid workflow dependencies now produce a local policy diagnostic before GitHub would emit a jobless failure.
- Status-doc validation:
  - Product status is unaffected. ADR indexes and DevOps instructions were updated.
- Risk & rollback plan:
  - The validation may reveal other stale dependencies, which are required fixes. Roll back the parser and fixture together only if GitHub changes the `needs` contract.
- Dependency rationale:
  - No dependency was added; validation uses the existing strict Ruby YAML representation.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift was found: the instructions required structural workflow validation but did not explicitly require dependency-reference validation. That requirement is now explicit.
