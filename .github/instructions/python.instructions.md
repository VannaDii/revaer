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

- Keep a structured README for each substantial tooling package: purpose,
  commands/inputs, module responsibilities, failure/cleanup behavior and evidence.
  Comment non-obvious invariants and native protocol details at their implementation.
- Validate migrated behavior against the existing native operation and retained
  artifacts. Test meaningful failures and ownership boundaries. Fixtures do not
  establish application, hosted workflow or publication acceptance.
- Chart orchestration fixtures stage explicit media or empty chart values so
  the checkout's compliance settings cannot change the scenario being tested.
- Run `rv tooling-check`, `rv tooling-cov` and `rv tooling-audit` as applicable
  to the change. Keep the full project acceptance gates and migration inventory
  current; Python checks alone do not complete a tooling migration.
