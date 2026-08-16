# Exact CI Node PATH fallback

- Status: Recorded
- Date: 2026-08-16
- Operator approval: Not applicable: nonarchitectural task record
- Context:
  - GitHub-hosted runners expose an ambient NVM installation even after the
    pinned `actions/setup-node` step installs Node 24.19.0 on `PATH`.
  - The exact-version wrapper rejected that valid CI installation whenever the
    ambient NVM catalogue did not also contain the version.
- Decision:
  - Keep NVM authoritative for local runs.
  - In CI only, allow the wrapper to continue after `nvm use` misses and require
    the existing exact `node --version` check to validate the manifest-pinned
    PATH installation before any Node command runs.
- Consequences:
  - CI consumes the exact Node installed by the pinned setup action instead of
    failing because of unrelated runner NVM state.
  - Local users with NVM still receive a fail-closed missing-version error.

## Task Record

- Motivation:
  - Restore every Node-backed PR check without weakening the exact-version gate.
- Design notes:
  - The fallback is gated by `CI=true`; it does not install software or accept a
    floating, missing, older, or newer Node version.
- Test coverage summary:
  - Added exact-PATH CI success and mismatched-PATH CI rejection cases while
    retaining the local NVM-miss rejection.
- Observability updates:
  - Existing bounded wrapper diagnostics remain unchanged for local failures and
    exact-version mismatches.
- Status-doc validation:
  - Updated the ADR index, documentation summary, and scoped DevOps instruction.
- Risk & rollback plan:
  - Risk is limited to CI selecting an unintended PATH binary; the exact semantic
    version check prevents that. Revert this task as one unit if setup behavior
    changes, then repair the setup action before rerunning Node-backed checks.
- Dependency rationale:
  - No dependency was added.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - Drift found: the instruction described only local NVM selection and did not
    account for the manifest-pinned CI setup action. The wording now covers both.
