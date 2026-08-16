# SonarQube Workflow With Root Coverage LCOV

- Status: Accepted
- Date: 2026-03-22
- Invocation supersession:
  - ADR 488 supersedes only this record's choice to execute the scanner through
    `SonarSource/sonarqube-scan-action`. The accepted Rust LCOV, native C-family
    compilation-database, source-scope, and workflow-trigger decisions remain in
    force. The exact signature-verified scanner is now installed by the setup
    action and invoked exactly once through `just sonar-scan`.
- Context:
  - Motivation:
    - Add automated SonarQube analysis for every pull request and every push to `main`.
    - Publish Rust coverage into SonarQube from a deterministic file in the repository root.
    - Keep the native libtorrent FFI source analyzed instead of excluding it from SonarQube.
  - Constraints:
    - CI workflows must use `just` recipes for operational steps.
    - Coverage artifact must be generated as `coverage/lcov.info`.
    - Generated coverage file must not be tracked by git.
    - The repository contains C++ FFI sources under `crates/revaer-torrent-libt/src/ffi`, and SonarQube requires a C-family compilation database to analyze them correctly.
- Decision:
  - Use `just cov` to build a combined workspace LCOV report at `coverage/lcov.info`.
  - Add `just sonar-compile-db` to build `revaer-torrent-libt` with `REVAER_NATIVE_COMPILE_COMMANDS_PATH` set so `build.rs` emits `coverage/compile_commands.json`.
  - Add `.github/workflows/sonar.yml` to trigger on `pull_request` and `push` to `main`.
  - Historical implementation: the Sonar workflow ran migrations, generated the LCOV file via `just cov`, generated the native compile database via `just sonar-compile-db`, and executed `SonarSource/sonarqube-scan-action@a31c9398be7ace6bbfaf30c0bd5d415f843d45e9` (`v7`). ADR 488 supersedes that executor with one exact scanner invocation through `just sonar-scan`; the accepted analysis inputs remain:
    - `sonar.projectKey=VannaDii_Revaer`
    - `sonar.organization=vannadii`
    - `sonar.rust.lcov.reportPaths=coverage/lcov.info`
    - `sonar.cfamily.compile-commands=coverage/compile_commands.json`
  - Ignore `/coverage` in `.gitignore`.
  - Alternatives considered:
    - Separate per-crate LCOV files merged post-process: rejected as unnecessary complexity for Sonar ingestion.
    - Invoking cargo directly in workflow: rejected because repository policy requires `just` recipes.
    - Excluding C-family files from SonarQube: rejected because the native adapter is first-party code and should remain part of static analysis.
    - Using external interception tooling such as Bear: rejected because the existing `cxx_build` path already knows the exact compiler flags, so emitting the compile database in `build.rs` keeps local and CI behavior aligned with fewer moving parts.
- Consequences:
  - Positive outcomes:
    - SonarQube now runs on PRs and `main` pushes with workspace Rust coverage.
    - Coverage path is stable and tool-agnostic (`coverage/lcov.info`).
    - Native FFI shim sources stay included in SonarQube analysis with the correct compiler context.
  - Risks and trade-offs:
    - Sonar job runtime includes full coverage execution and DB-backed tests.
    - Workflow requires valid `SONAR_TOKEN` repository secret and database variable setup.
    - The compile database currently describes the checked-in `session.cpp` translation unit, so future native sources must be added deliberately if the FFI surface grows.
- Follow-up:
  - Test coverage summary:
    - Validation must cover `just sonar-compile-db` producing `coverage/compile_commands.json` plus the repository's required `just` gates.
  - Observability updates:
    - No runtime telemetry changes; this is CI/workflow-only.
  - Risk and rollback plan:
    - Roll back by removing `just sonar-compile-db`, the `build.rs` compile database emission, and the workflow scan property if SonarQube compilation-database support causes instability.
  - Dependency rationale:
    - No Rust dependencies added.
    - Historical decision: this record originally retained
      `SonarSource/sonarqube-scan-action@a31c9398be7ace6bbfaf30c0bd5d415f843d45e9`
      (`v7`). ADR 488 supersedes that invocation dependency with the repository's
      exact signature-verified scanner installer and canonical `just` recipe.
