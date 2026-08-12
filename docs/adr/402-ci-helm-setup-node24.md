# CI Helm Setup Node 24 Runtime

- Status: Accepted
- Date: 2026-08-04
- Context:
  - GitHub Actions reports that the pinned `Azure/setup-helm` `v4.3.1` action targets deprecated Node.js 20 and is being forced onto Node 24 at runtime.
  - Revaer uses the action in seven Helm validation, packaging, and publication steps across pull-request, release, image, and manual-verification workflows.
  - Upstream `Azure/setup-helm` `v5.0.1` preserves the existing optional `version`, `token`, and `downloadBaseURL` inputs and declares Node 24 as its action runtime.
- Decision:
  - Pin every `Azure/setup-helm` use to the immutable commit for upstream `v5.0.1`.
  - Keep the existing default Helm version behavior and workflow step ordering unchanged.
  - Require future Helm setup pins to remain on a Node 24-capable release line while GitHub Actions requires that runtime.
- Consequences:
  - Helm jobs stop relying on GitHub's compatibility override for a deprecated Node.js 20 action runtime.
  - The same reviewed action version is used consistently by every Helm workflow path.
  - Future upstream major-version changes still require documentation and input-contract review before repinning.
- Follow-up:
  - Confirm pull-request checks execute Helm lint, packaging, image, and release-build paths without the Node.js 20 annotation.
  - Continue reviewing third-party action runtime annotations as part of workflow maintenance.

## Task Record

- Motivation:
  - Remove the remaining deprecated Node.js 20 action runtime from required Helm CI paths before GitHub removes the compatibility override.
- Design notes:
  - The update changes only the immutable action pin and its audit comment; no workflow inputs, permissions, conditions, or release behavior change.
  - The upstream `v5.0.1` `action.yml` was inspected directly and confirms `runs.using: node24` with an input contract compatible with current use.
- Test coverage summary:
  - `just instruction-drift` validates that the workflow change and scoped instruction update remain aligned.
  - `just lint` validates immutable action pinning and workflow policy guardrails.
  - `just ci` and `just ui-e2e` exercise the repository handoff gates locally.
  - Pull-request checks provide GitHub-hosted validation of every changed workflow path.
- Observability updates:
  - GitHub Actions annotations are the affected operational signal; successful validation removes the Node.js 20 deprecation annotation from Helm setup steps.
- Status-doc validation:
  - Reviewed the README, release checklist, and operator-facing documentation; no user-facing Helm command or release procedure changes.
- Risk & rollback plan:
  - Risk is limited to upstream action execution in Helm jobs because the action input surface and step ordering are unchanged.
  - Roll back by reverting this commit and restoring the prior immutable pin if upstream `v5.0.1` behaves incompatibly on GitHub-hosted runners.
- Dependency rationale:
  - No repository dependency is added.
  - Updating the existing official Helm setup action is smaller and more maintainable than introducing a custom installer.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, `.github/workflows/ci.yml`, `.github/workflows/pr.yml`, `.github/workflows/build-images.yml`, `.github/workflows/helm-oci-verify.yml`, `scripts/workflow-guardrails.sh`, ADR 307, and the upstream `Azure/setup-helm` `v5.0.1` action metadata.
  - Drift was found between the workflow runtime pin and GitHub's current Node 24 action runtime requirement.
  - The contradiction is removed by using the current Node 24 action release consistently and recording the requirement in the scoped instruction file.
