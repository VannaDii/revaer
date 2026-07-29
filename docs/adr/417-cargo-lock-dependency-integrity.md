# Cargo Lock Dependency Integrity

- Status: Accepted
- Date: 2026-08-11
- Context:
  - `Cargo.lock` resolved `thiserror` at `2.0.18`, but the media core and runtime package records still named `thiserror 2.0.19`.
  - Cargo could repair the stale package references during an unlocked operation, while `cargo-audit 0.22.0` failed closed because the recorded dependency tree was impossible to resolve.
- Decision:
  - Regenerate the two stale workspace package references so every `thiserror` dependency points to the package version present in the lockfile.
  - Keep the existing exact audit tool version, empty advisory ignore policy, and warning-denial posture unchanged.
  - Treat lockfile graph integrity failures as required dependency fixes rather than audit exceptions.
- Consequences:
  - Locked Cargo metadata and RustSec audit analysis can resolve the complete dependency graph deterministically.
  - The change does not alter the selected dependency versions or runtime behavior.
- Follow-up:
  - Require the PR audit job to pass before downstream coverage, native, E2E, image, and release jobs are accepted.
  - Continue updating dependency records through Cargo rather than hand-maintaining package references.

## Task Record

- Motivation:
  - Restore the audit gate so every dependent PR can run its complete check fan-out.
- Design notes:
  - Only the inconsistent dependency edges are corrected; package versions and checksums remain unchanged.
  - No advisory, source, severity, or quality-gate criterion is suppressed or relaxed.
- Test coverage summary:
  - Validate `cargo metadata --locked --offline --format-version 1 --no-deps`.
  - Validate `cargo tree --locked --offline -i thiserror@2.0.18` resolves both media crates.
  - Run the canonical audit recipe and the complete PR check fan-out after the stack is restacked.
- Observability updates:
  - No runtime observability surface changes. Audit failures remain visible and fail closed in CI.
- Status-doc validation:
  - Rechecked the repository status and media contract documents; no product-status wording changes are required for a lockfile integrity repair.
- Risk & rollback plan:
  - Risk is limited to an incorrect lockfile edge. Locked metadata and audit verification detect that condition; rollback is a single-commit revert.
- Dependency rationale:
  - No dependency is added, removed, upgraded, or downgraded.
- Stale-policy check:
  - Reviewed `AGENTS.md` and `.github/instructions/devops.instructions.md`.
  - No policy drift was found; both already require exact, fail-closed dependency auditing without advisory exceptions.
