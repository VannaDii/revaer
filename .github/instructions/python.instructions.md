---
applyTo:
  - "**/*.py"
  - "tools/**"
  - "pyproject.toml"
  - "uv.lock"
  - ".python-version"
  - ".uv-version"
---

`AGENTS.md` is the root contract. The accepted tooling migration is recorded in
`docs/adr/592-python-tooling.md`; this file specializes its Python implementation.
Workflow, release and installation policy remains in `devops.instructions.md`.

# Package management

- Use uv's official supported installer, project, dependency-group and tool
  interfaces. uv owns its environments and executables; do not alter them by
  hand or introduce parallel installation/resolution mechanisms.
- Ordinary invocations use the checked-in pins and `uv run --locked`. Dependency
  changes use uv's resolver and include the lockfile and written rationale.
  Select the appropriate group; keep core-only container commands independent
  of browser, development and release dependencies.
- Install the small worktree-aware launcher through `uv tool install`. It selects
  the checkout implementation; it must not carry another full dependency graph.

# Architecture and errors

- Keep the explicit CLI dictionary mapped to derived tasks' static
  `run(context)` methods. Internal steps stay ordinary calls unless useful as
  independent commands. Follow `tools/src/revaer_tooling/tasks/README.md`.
- Load environment settings and construct concrete external collaborators at
  the CLI boundary. Pass immutable typed settings/options and injected tools,
  filesystem operations and output functions to tasks.
- Native tools have classes that locate/verify the executable and accept typed
  operation arguments. Use the shared process runner; never evaluate a shell
  command string. Preserve cancellation, diagnostics, redaction and exact data
  bytes when the native protocol requires them.
- Use strict type checking. Narrow JSON and native output at their boundaries.
  Proof/provider evidence rejects duplicate keys and nonfinite numbers; missing
  or malformed input cannot become a default success.
- Propagate or translate failures explicitly. Runtime checks must not depend on
  Python assertions, which can be disabled. Do not swallow errors, suppress
  lints/types/warnings, retain dead paths or add future implementation stubs.
  Protocol declarations describe collaborators actually used by the code.

# Documentation and verification

- Native LLVM text reports use numbered source lines without macro expansion or
  region subviews for scanner compatibility. Preserve coverage measurements and
  expanded HTML diagnostics. Scanner debug logging must use the supported
  `--debug` option, without command-line criteria overrides.
- Route authored tracked Rust LCOV records to the Rust sensor; retain native,
  dependency and generated measurements in the complete JSON, text and HTML
  reports. Never modify counts or discard uncovered authored Rust lines.

- Browser startup regressions in `tools/tests/fixtures/browser_startup.py` exercise
  the shipped page helper in Chromium and WebKit. Keep delayed rendering and
  missing-control cases together; use the readiness contract in
  `revaer-ui.instructions.md` without changing application timeout defaults.

- Keep a structured README for each substantial tooling package: purpose,
  commands/inputs, module responsibilities, failure/cleanup behavior and evidence.
  Comment non-obvious invariants and native protocol details at their implementation.
- Validate migrated behavior against the existing native operation and retained
  artifacts. Test meaningful failures and ownership boundaries. Fixtures do not
  establish application, hosted workflow or publication acceptance.
- Run `rv tooling-check`, `rv tooling-cov` and `rv tooling-audit` as applicable
  to the change. Keep the full project acceptance gates and migration inventory
  current; Python checks alone do not complete a tooling migration.
- Distributed coverage consumes separately downloaded producer artifacts. Use
  the existing shard selection/completion validator before combining measurements;
  reject missing phases, inconsistent partitions, corrupt databases and absent
  Chromium captures. Keep the unsharded local/main scanner path functional.
- The internal, dev-dependency-only `revaer-test-support` crate retains complete
  validated coverage measurements without a production percentage threshold,
  under the operator-approved production-only coverage requirement. Other crate
  thresholds and all workspace instrumentation remain unchanged.
- Scanner extraction materializes upstream internal JRE license links from
  regular archive members. Reject escaping, missing or chained targets and retain
  expanded-byte limits, pinned archive hashes and signature verification.

- Keep distributed producer validation in focused functions, share repeated coverage
  paths and workflow step names as constants, and keep regression assertions and
  expected-failure calls individually scoped.
- Fixtures must supply their own browser phase selection and Sonar PR identity;
  operator qualification settings must not change the controlled fixture data.
- Composite qualification must enter through the same uv dotenv-loading boundary
  as standalone E2E before constructing its task context.
- Rust coverage routing normalizes paths for tracked-source membership and merges
  same-source compilation records using their measured counters. Retain separate
  compilation records in the complete engine reports; follow root operator consent.
- Sonar source preparation removes untracked Python bytecode caches beside authored
  Python files, including caches left by earlier runs; refuse tracked content and links.
- Coverage fixtures assert the actual Context type after copying options, and URL
  fixtures generate credentials at runtime while retaining percent-encoding cases.
- Exact SCM preparation must reject a stale local target branch that would shadow
  the fetched event base. Preserve shared worktrees; qualify in an independent checkout.
