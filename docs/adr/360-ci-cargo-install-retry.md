# CI Cargo Install Retry

- Status: Accepted
- Date: 2026-07-30
- Context:
  - PR E2E coverage jobs failed before tests could run when `cargo install` hit transient crates.io HTTP/2 transport failures while installing exact, locked Rust CLI tools.
  - Required gates must still run through the Justfile, keep reviewed tool versions, keep `--locked`, and fail closed when installation genuinely cannot complete.
- Decision:
  - Add `scripts/cargo-install-retry.sh` as the shared wrapper for required-recipe Cargo tool installs.
  - Keep each tool version pinned in the Justfile and keep published lockfile installs. The wrapper retries the same install command and disables Cargo HTTP multiplexing only after the first transport failure.
  - Update Rust and DevOps instructions so future required-recipe Cargo installs use the wrapper or a pinned prebuilt installer path.
- Consequences:
  - Required CI jobs are less likely to skip their real gates because of transient registry transport errors.
  - Install failures still fail closed after bounded retries.
- Follow-up:
  - Prefer prebuilt pinned installer actions for required CI tools when the repository already has an approved action path.
  - Keep exact versions reviewed in the Justfile and task records before upgrading tools.

## Task Record

- Motivation:
  - API E2E coverage and UI E2E failed before route and JavaScript coverage could run because live Cargo tool installation hit transient HTTP/2 registry errors.
- Design notes:
  - The wrapper does not change package names, versions, feature flags, `--locked`, or `--force` behavior supplied by recipes.
  - Retry count defaults to three attempts and can be tightened or widened with `REVAER_CARGO_INSTALL_ATTEMPTS` without editing recipe criteria.
- Test coverage summary:
  - `bash scripts/cargo-install-retry.sh` argument validation was exercised with an empty invocation.
  - Justfile, instruction drift, workflow guardrails, and GitHub PR checks were rerun after the change.
- Observability updates:
  - Retry attempts print the failing status and sleep duration to CI logs.
- Status-doc validation:
  - Root `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/devops.instructions.md` were re-checked for required tool-install policy alignment.
- Risk & rollback plan:
  - Risk: a truly broken registry, yanked package, or incompatible lockfile still fails after retries. Roll back by reverting the wrapper and Justfile call-site changes.
- Dependency rationale:
  - No new dependencies were added.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Drift found: instructions required exact locked installs but did not require the retry wrapper that prevents transport flakes from blocking required gates. The relevant scoped instructions now reference the wrapper.
