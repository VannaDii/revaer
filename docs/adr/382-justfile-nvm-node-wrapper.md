# Justfile NVM Node wrapper

- Status: Accepted
- Date: 2026-08-11
- Context:
  - Local validation shells can expose multiple Node installations even when the operator selected a version through NVM.
  - Node-backed checks must use one intended runtime for release validation, generated API clients, Playwright, and JavaScript coverage.
  - The rebuilt Justfile already uses non-login Bash, preserving the caller's toolchain PATH as required by the later review repair.
- Decision:
  - Add `scripts/with-node.sh` as the canonical Node command wrapper for Justfile recipes.
  - Source NVM when available and select an explicit `REVAER_NODE_VERSION`; otherwise prefer the installed `lts/*` alias.
  - Fail local execution when NVM exists without the requested or LTS runtime instead of silently selecting another Node installation.
  - Permit CI to retain the setup-provided PATH when its NVM installation lacks an LTS alias.
  - Route `npm`, `npx`, `node`, and JavaScript bin shims in the Justfile through the wrapper.
- Consequences:
  - Local recipes consistently use NVM-managed Node, while GitHub Actions can continue using its pinned setup action.
  - Operators with NVM but no installed LTS alias must install one or set `REVAER_NODE_VERSION`.
- Follow-up:
  - Keep future Node-backed Justfile commands behind the wrapper and retain the non-login shell contract.

## Task Record

- Motivation:
  - Reconstruct ADR 382 from `e9c192d6` on the rebuilt head and preserve the later review repair that prevents login-shell startup files from replacing selected toolchains.
- Design notes:
  - The wrapper only selects Node and then replaces itself with the requested command.
  - Explicit version selection remains fail-closed in every environment; only implicit LTS selection can fall back in CI.
  - Direct workflow Node commands remain governed by the repository's exact setup-action pins and are outside the Justfile wrapper boundary.
- Test coverage summary:
  - `bash scripts/tests/with-node-test.sh`
  - The local fail-closed case explicitly sets `CI=false`, so inherited GitHub Actions state cannot turn that assertion into the permitted CI fallback path.
  - `REVAER_NODE_VERSION=24.14.1 bash scripts/with-node.sh node --version`
  - `REVAER_NODE_VERSION=24.14.1 bash scripts/with-node.sh npm --version`
  - `just policy`
  - `just instruction-drift`
- Observability updates:
  - Setup failures identify a missing local NVM LTS version and the required remediation before the requested command runs.
  - No runtime logs, metrics, traces, health checks, or events changed.
- Status-doc validation:
  - Updated `docs/adr/index.md` and `docs/SUMMARY.md`; no product or operator behavior document required another change.
- Risk & rollback plan:
  - The primary risk is an environment-specific NVM initialization regression. Revert the wrapper routing and task record together while retaining the non-login Justfile shell.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, and `.github/instructions/revaer-ui.instructions.md`.
  - Added the missing wrapper requirement to the DevOps instruction; no contradiction remains.
