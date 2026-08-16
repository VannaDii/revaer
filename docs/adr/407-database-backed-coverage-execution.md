# Database-Backed Coverage Execution

- Status: Accepted
- Date: 2026-08-04
- Context:
  - `just cov` resolved and exported the test database URL in the shell that invoked `just db-start`, then launched `cargo llvm-cov` from a separate recipe shell.
  - Database-backed tests therefore observed no configured endpoint during local coverage runs and returned through their unavailable-database paths, leaving seven crates below the mandatory 90% line floor.
  - The later quality-baseline implementation at `104c54f0` demonstrated that preserving the endpoint through coverage execution activates the existing test corpus without exclusions or threshold changes.
- Decision:
  - Resolve the maintenance database URL once and pass it explicitly to both `just db-start` and `cargo llvm-cov` in one fail-closed command chain.
  - Compose the local URL through `scripts/local-postgres-url.sh` so credentials, host, port, and database remain explicit inputs rather than duplicated URI literals.
  - Try a `host.docker.internal` fallback only after a local `localhost` or `127.0.0.1` endpoint fails, and force disposable database removal so stale test-pool sessions cannot leak databases between tests.
  - Preserve the exact 90% per-package coverage floor and complete workspace/all-features instrumentation.
  - Fetch complete Git history for Sonar analysis and retain separate Rust LCOV and native LLVM coverage reports from the same instrumented run.
  - Rejected alternatives were coverage exclusions, threshold relaxation, redundant tests before activating the existing corpus, and moving the broader quality-baseline commit ahead of its dependencies.
- Consequences:
  - Existing integration tests contribute coverage consistently in local and CI execution.
  - Sonar can resolve the main-branch baseline and publish coverage for authored Rust and native C/C++ code instead of accepting a zero-coverage analysis.
  - Coverage now fails before instrumentation when the required database cannot be started or reached.
  - PostgreSQL 13 or newer is required for `DROP DATABASE ... WITH (FORCE)`; CI and local development use PostgreSQL 16.
- Follow-up:
  - Keep database URL propagation aligned when coverage or database lifecycle recipes change.
  - Retain per-crate coverage evidence for the seven previously failing crates.

## Task Record

- Motivation:
  - Restore truthful coverage execution before PR #71 without adding suppression or consuming that PR's remaining review budget.
- Design notes:
  - The change extracts only the coverage and disposable-Postgres lifecycle behavior validated by `104c54f0`.
  - The prerequisite also aligns PR and main Sonar jobs with the coverage tool profile, complete SCM history, and both authored-language report formats.
- Test coverage summary:
  - Focused URL composition and test-support unit tests cover local fallback, forced cleanup, and an unreachable IPv6 loopback endpoint without invoking Docker-host fallback.
  - `just ci` passed with database-backed coverage active and every per-package coverage assertion at or above 90%.
  - `just ui-e2e` passed all 101 API and browser tests.
- Observability updates:
  - No production telemetry changes. Coverage startup continues to report the selected database endpoint through the existing database recipe.
- Status-doc validation:
  - Repository status and operator documentation were reviewed; no product-status change is introduced.
- Risk & rollback plan:
  - Risk is limited to local database endpoint selection and disposable database cleanup. Revert this commit to restore the previous lifecycle, then investigate without lowering coverage.
- Dependency rationale:
  - No dependencies are added. The implementation uses Bash, `std`, `url`, and PostgreSQL capabilities already required by the repository.
- Stale-policy check:
  - Reviewed `AGENTS.md`, `.github/instructions/devops.instructions.md`, `.github/instructions/rust.instructions.md`, and `.github/instructions/revaer-data.instructions.md`.
  - Drift was found in the missing requirement that coverage preserve its database endpoint across recipe execution; the scoped DevOps instruction now records that invariant.
  - No contradictory or stale references were removed.
