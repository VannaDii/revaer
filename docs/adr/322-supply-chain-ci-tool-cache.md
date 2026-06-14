# Supply Chain CI Tool Cache

- Status: Accepted
- Date: 2026-06-13
- Context:
  - The PR workflow ran `audit`, `deny`, and `udeps` as separate jobs. Each job paid repeated setup cost, and `cargo-deny`, `cargo-audit`, and `cargo-udeps` could be compiled from source when absent on a fresh runner.
  - `cargo-udeps` requires nightly-only compiler flags, but the recipe first attempted a stable run before retrying on nightly.
  - Downstream jobs must remain gated on supply-chain checks; the change must not remove the validation dependency.
- Decision:
  - Replace the separate PR `audit`, `deny`, and `udeps` jobs with one `supply-chain` job that restores installed-tool and advisory caches, installs the three cargo tools through the existing pinned prebuilt installer action, and runs `just audit`, `just deny`, and `just udeps`.
  - Keep downstream jobs gated on the combined `supply-chain` job.
  - Change `just udeps` to run directly on `REVAER_UDEPS_TOOLCHAIN`, defaulting to `nightly`, instead of probing stable first.
  - Alternative considered: remove supply-chain checks from downstream `needs` and rely only on branch protection. That would reduce wall-clock time further, but was explicitly out of scope for this task.
- Consequences:
  - Fresh runners avoid compiling supply-chain tools from source when the prebuilt installer can provide them.
  - Warm runners can reuse installed tool binaries and the advisory database.
  - The combined job trades some parallelism for less repeated setup; downstream validation still waits for the full supply-chain result.
  - Branch protection may need to track the new `Supply Chain Checks` job name instead of the previous three separate checks.
- Follow-up:
  - Compare CI timings after this lands and keep the combined job only if setup savings outweigh the parallelism loss.
  - If cargo-udeps becomes stable-compatible later, revisit the nightly-only recipe.

## Task Record

- Motivation:
  - The `udeps` and `deny` CI steps were taking too long because setup and tool installation dominated the checks.
- Design notes:
  - The implementation preserves `just` as the command surface and keeps the workflow using pinned external actions.
  - Installed tool cache keys include exact tool versions so binary reuse cannot silently cross a tool upgrade.
  - The advisory database cache is separate from the installed-tool cache because its refresh cadence is independent of tool versions.
- Test coverage summary:
  - Added workflow guardrails that fail when PR supply-chain checks are split back into standalone setup-heavy jobs or lose tool/advisory caches.
  - Target verification for this change is `bash scripts/workflow-guardrails.sh`, `just fmt`, `just lint`, `just audit`, `just deny`, and `just udeps`.
- Observability updates:
  - No runtime observability changes. CI logs now group audit, deny, and udeps under the `Supply Chain Checks` job.
- Status-doc validation:
  - Updated `.github/instructions/devops.instructions.md`, `docs/adr/index.md`, and `docs/SUMMARY.md`.
  - No user-facing runtime README changes are required.
- Risk & rollback plan:
  - If the prebuilt installer does not support one of the requested cargo tools, revert the `supply-chain` job to source installation through the existing `just` recipes or split only the unsupported tool back out.
  - If branch protection expects the old job names, update branch protection to require `Supply Chain Checks` or temporarily restore the old jobs.
- Dependency rationale:
  - No new project dependency was added.
  - The workflow reuses the already-pinned `taiki-e/install-action` and `actions/cache` actions.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, `.github/workflows/pr.yml`, `.github/actions/setup-revaer/action.yml`, and `justfile`.
  - Drift found: the instructions did not yet describe the combined supply-chain job or direct-nightly `udeps`; this ADR and the devops instruction update record the new policy.
