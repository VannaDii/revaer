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
- Install the small worktree-aware launcher through `uv tool install`. It selects the checkout implementation; it must not carry another full dependency graph.

# Architecture and errors

- Native text coverage sent to Sonar uses numbered source-line counts without macro expansion or sub-line region displays. Preserve measured line/branch counts, complete source scope and expanded HTML diagnostics.

- Bootstrap coverage uses a private disposable pytest base directory and removes it on success or failure, leaving other pytest sessions' directories intact.

- E2E's explicitly managed Linux roots use injected native mount/unmount tools. Rust tests and coverage reuse that fixture when explicitly selected, passing its mounted directory as `REVAER_NATIVE_RECOVERY_ROOT` for native root tests. Preserve the private owned directory after an unmount failure; never remove caller roots or substitute catalog declarations for bootstrap attestation. The disposable fixture has 4 GiB of capacity so normal media work can run above the production 1 GiB free-space reserve; production limits stay intact.

- The optional fixture-probe container is caller-selected at bootstrap. Setup provisions the existing approved Alpine build; cleanup verifies project, purpose and read-only checkout ownership before removing that container.

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
- Route authored tracked Rust LCOV records to the Rust sensor; retain native, dependency and generated measurements in the complete JSON, text and HTML reports. Never modify counts or discard uncovered authored Rust lines.

- Browser startup regressions in `tools/tests/fixtures/browser_startup.py` exercise the shipped page helper in Chromium and WebKit. Keep delayed rendering and missing-control cases together; use the readiness contract in `revaer-ui.instructions.md` without changing application timeout defaults.

- Keep a structured README for each substantial tooling package: purpose, commands/inputs, module responsibilities, failure/cleanup behavior and evidence. Comment non-obvious invariants and native protocol details at their implementation.
- Test task behavior through the supported native operation. Test meaningful failures and ownership boundaries. Fixtures do not establish application, hosted workflow or publication acceptance.
- Chart orchestration fixtures stage explicit media or empty chart values so the checkout's compliance settings cannot change the scenario being tested.
- Distributed coverage consumes separately downloaded producer artifacts. Use the existing shard selection/completion validator before combining measurements; reject missing phases, inconsistent partitions, corrupt databases and missing required execution data. Keep the unsharded local/main scanner path functional.
- Scanner extraction materializes upstream internal JRE license links from regular archive members. Reject escaping, missing or chained targets and retain expanded-byte limits, pinned archive hashes and signature verification.

- Keep distributed producer validation in focused functions, share repeated coverage paths and workflow step names as constants, and keep regression assertions and expected-failure calls individually scoped.
- Fixtures must supply their own browser phase selection, instruction-diff refs and native recovery roots; operator qualification settings must not change the controlled fixture data.
- Composite qualification must enter through the same uv dotenv-loading boundary as standalone E2E before constructing its task context.
- Rust coverage routing normalizes paths for tracked-source membership and merges same-source compilation records using their measured counters. Retain separate compilation records in the complete engine reports; follow root operator consent.
- Sonar source preparation removes untracked Python bytecode caches beside authored Python files, including caches left by earlier runs; refuse tracked content and links.
- Coverage fixtures construct Context with their existing injected collaborators and explicit Options; URL fixtures generate credentials at runtime while retaining percent-encoding cases.
- Exact SCM preparation must reject a stale local target branch that would shadow the fetched event base. Preserve shared worktrees; qualify in an independent checkout.
