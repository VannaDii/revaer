# Exact Documentation Tools

- Status: Accepted
- Date: 2026-08-11
- Context:
  - The documentation recipes selected the newest available mdBook and Lychee releases and reused arbitrary installed versions.
  - Locked dependency resolution does not select an exact crate release without an explicit version.
  - The link checker also suppressed failures, so broken documentation links could not fail the recipe.
- Decision:
  - Pin the warning-free compatible pair mdBook `0.5.0` and mdbook-mermaid `0.17.0`, plus Lychee `0.24.2`.
  - Provision all three through `scripts/ensure-exact-cargo-tool.sh`, which verifies the installed version and uses locked, bounded-retry installation on mismatch.
  - Normalize the conventional leading `v` in version output and put Cargo's install directory first for documentation recipes so an unrelated system binary cannot shadow the verified executable.
  - Make Lychee findings fail `docs-link-check` and assert the exact documentation-tool contract in the Cargo-tool guardrail test.
  - A separate documentation container was considered, but it would duplicate tool ownership and add image maintenance without improving the existing exact installer contract.
- Consequences:
  - Identical commits select the same documentation tool releases locally and in automation.
  - Tool upgrades become explicit reviewed changes.
  - Existing broken links now fail instead of being silently ignored.
- Follow-up:
  - Upgrade pins only with a successful documentation build and link-check run.
  - Keep the recipe assertions aligned when documentation tooling changes.

## Task Record

- Motivation:
  - Resolve PR 116 feedback that identified floating and unverified documentation tools.
- Design notes:
  - Reuse the shared exact Cargo-tool installer rather than duplicating version parsing and reinstall logic.
  - Keep versions adjacent to the recipes that own them and test all required documentation-tool invocations.
- Test coverage summary:
  - Run `bash scripts/test-exact-cargo-tool.sh`, `just policy`, `just instruction-drift`, `just docs-build`, and `just docs-link-check`.
- Observability updates:
  - Exact installer output reports whether each required version is retained, installed, or replaced.
  - Lychee failures now remain visible through the recipe exit status.
- Status-doc validation:
  - Reviewed the documentation recipes and ADR indexes; no user-facing status claim changed.
- Risk & rollback plan:
  - Risk is incompatibility in one selected release or newly exposed broken links.
  - Roll back by reverting this ADR and recipe change; do not restore floating versions or failure suppression as a forward fix.
- Dependency rationale:
  - No application dependency was added. The existing documentation tools are pinned to reviewed releases.
  - The workspace lockfile records the latest resolver-selected patch releases for two existing transitive platform crates so locked workspace tests remain reproducible under Rust 1.96.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/devops.instructions.md`.
  - Drift was present because the required-tool rule did not name the documentation toolchain and the recipes did not satisfy the exact-version contract.
  - Updated both scoped instruction files and removed silent link-check suppression.
