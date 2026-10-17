---
applyTo:
  - "**/*.py"
  - "tools/**"
  - "pyproject.toml"
  - "uv.lock"
  - ".python-version"
  - ".uv-version"
---

`AGENTS.md` is the root contract. This file specializes Python implementation; architectural rationale is recorded in `docs/adr/592-python-tooling.md`. Workflow, release and installation policy remains in `devops.instructions.md`.

# Package management

- Use uv's official supported installer, project, dependency-group and tool interfaces. uv owns its environments and executables; do not alter them by hand or introduce parallel installation/resolution mechanisms.
- Ordinary invocations use the checked-in pins and `uv run --locked`. Dependency changes use uv's resolver and include the lockfile and written rationale. Select the appropriate group; keep core-only container commands independent of browser, development and release dependencies.
- Keep the SQL scanner on a published, auditable `pglast` release matching the reviewed PostgreSQL runtime major and preserve the existing token/UTF-8 boundary contract. The manifest and uv lockfile own the exact pin. Retired JavaScript collectors do not justify retaining their unused parser dependencies.
- Install the small worktree-aware launcher through `uv tool install`. It selects the checkout implementation; it must not carry another full dependency graph.

# Architecture and errors

- Native text coverage sent to Sonar uses numbered source-line counts without macro expansion or sub-line region displays. Preserve measured line/branch counts, complete source scope and expanded HTML diagnostics.

- Bootstrap behavior runs use a private disposable pytest base directory and remove it on success or failure, leaving other pytest sessions' directories intact. Retain raw upstream reports without a custom merger, hit-count validation or Sonar coverage import.

- Follow the root `.test-tmp/` contract. Test CLI commands supply private run
  directories through `TMPDIR` and restore environment/cache state afterward.
  Managed media fixtures create owned children beneath that same repository root;
  Rust and coverage tasks pass them as `REVAER_NATIVE_RECOVERY_ROOT`. Keep normal
  bootstrap attestation and production free-space reserves intact. GnuPG homes
  use short private children of the common root to respect Unix socket limits.

- The optional fixture-probe container is caller-selected at bootstrap. Setup provisions the existing approved Alpine build; cleanup verifies project, purpose and read-only checkout ownership before removing that container.

- Rootless Docker database fixtures grant the mapped server group access only to their owned scratch directory, retain PostgreSQL data permissions and restore caller ownership before cleanup. Never widen repository permissions.
- Alpine interpreter fixtures run as the caller on rootful Docker and as mapped container root on rootless Docker. Keep interpreter/cache writes inside their owned checkout and verify bind-mounted outputs retain caller ownership.
- `rv test-media-root-catalog` exercises both runtime catalog primitives and native application attestation against its managed repo-local directories.

- Keep the explicit CLI dictionary mapped to derived tasks' static `run(context)` methods. Internal steps stay ordinary calls unless useful as independent commands. Follow `tools/src/revaer_tooling/tasks/README.md`.
- Load environment settings and construct concrete external collaborators at the CLI boundary. Pass immutable typed settings/options and injected tools, filesystem operations and output functions to tasks.
- Native tools have classes that locate/verify the executable and accept typed operation arguments. Use the shared process runner; never evaluate a shell command string. Preserve cancellation, diagnostics, redaction and exact data bytes when the native protocol requires them.
- Use strict type checking. Narrow JSON and native output at their boundaries. Proof/provider evidence rejects duplicate keys and nonfinite numbers; missing or malformed input cannot become a default success.
- Propagate or translate failures explicitly. Runtime checks must not depend on Python assertions, which can be disabled. Do not swallow errors, suppress lints/types/warnings, retain dead paths or add future implementation stubs. Protocol declarations describe collaborators actually used by the code.

# Documentation and verification

- The essential `rv` percentage floor uses the implementation-only include set in `[tool.coverage.report]` of `pyproject.toml`. Raw execution data may retain tests, and the aggregate Sonar report preserves authored inventory, but neither makes test scaffolding part of that percentage denominator.

- Follow Section 3 of `AGENTS.md` for coverage targets and validation order. The independent tooling/launcher gate measures implementation code in the designated essential paths; test modules and embedded fixture scripts do not acquire coverage targets from inclusion in a Python report. Use supported coverage reports; do not implement or test collector behavior or create fixture hit-count requirements.

- Instruction drift receives PR author/repository identity from workflow event fields at the CLI boundary. Limit the Dependabot exception to same-repository dependency-only changes; spoofed authors, forks and mixed source changes retain normal drift findings. Native auto-merge must validate provider metadata, paginate all PRs and propagate API/merge failures.

- Bound the complete tooling suite to two file-grouped pytest workers. Preserve module fixture isolation and complete coverage merging. Measure tool setup and test execution separately.

- Native LLVM text reports use numbered source lines without macro expansion or region subviews for scanner compatibility. Preserve coverage measurements and expanded HTML diagnostics. Scanner debug logging must use the supported `--debug` option, without command-line criteria overrides.
- Feed the Rust sensor the Cobertura report generated by `cargo-llvm-cov`; retain complete native, dependency and generated measurements in the original LCOV, JSON, text and HTML reports. Apply the supported native-source suffix filter consistently to the Rust threshold and importer reports, including implementation headers, while retaining the separate first-party native execution gate. Do not parse, merge or normalize Rust coverage counters.

- Browser startup regressions in `tools/tests/fixtures/browser_startup.py` exercise the shipped page helper in Chromium and WebKit. Keep delayed rendering and missing-control cases together; use the readiness contract in `revaer-ui.instructions.md` without changing application timeout defaults.

- Keep a structured README for each substantial tooling package: purpose, commands/inputs, module responsibilities, failure/cleanup behavior and evidence. Comment non-obvious invariants and native protocol details at their implementation.
- Test task behavior through the supported native operation. Test meaningful failures and ownership boundaries. Fixtures do not establish application, hosted workflow or publication acceptance.
- Chart orchestration fixtures stage explicit media or empty chart values so the checkout's compliance settings cannot change the scenario being tested.
- Distributed coverage consumes separately downloaded producer artifacts. Use the existing shard selection/completion validator before combining measurements; reject missing phases, inconsistent partitions, corrupt databases and missing required execution data. Keep the unsharded local/main scanner path functional.
- Scanner extraction materializes upstream internal JRE license links from regular archive members. Reject escaping, missing or chained targets and retain expanded-byte limits, pinned archive hashes and signature verification.

- Keep distributed producer validation in focused functions, share repeated coverage paths and workflow step names as constants, and keep regression assertions and expected-failure calls individually scoped.
- Fixtures must supply their own browser phase selection, instruction-diff refs and native recovery roots; operator qualification settings must not change the controlled fixture data.
- Composite qualification must enter through the same uv dotenv-loading boundary as standalone E2E before constructing its task context.
- Generate the Rust importer report through the supported `cargo-llvm-cov --cobertura` interface. Keep ingestion limited to requiring the report; upstream tools own coverage calculations and report validation.
- Sonar source preparation removes untracked Python bytecode caches beside authored Python files, including caches left by earlier runs; refuse tracked content and links.
- Coverage fixtures construct Context with their existing injected collaborators and explicit Options, and own their managed-root/native-required settings so qualification flags cannot enable unrelated integration scenarios. URL fixtures generate credentials at runtime while retaining percent-encoding cases.
- Exact SCM preparation must reject a stale local target branch that would shadow the fetched event base. Preserve shared worktrees; qualify in an independent checkout.
- E2E authentication setup gives factory reset a bounded 30-second request deadline for normalized-table cleanup. Other API requests retain their 10-second deadline; preserve status, mode-transition and schema assertions.

- `rv cargo-lock` remains offline by default. Use its explicit `--online` option when changed manifest requirements need missing registry metadata; preserve existing unrelated locked registry versions.
