# rv: Revaer tooling

`rv` is Revaer's command line interface for development and automation. uv manages
Python, dependencies, and installed executables. Revaer code coordinates the
project's tools and checks.

The migration is in progress. [The migration inventory](migration.md) records
which existing recipes still require implementation and parity checks. The
existing gates remain authoritative until their replacements are verified.

## Getting started

From the repository root:

```console
./setup.sh
rv --help
```

The bootstrap installs uv with its
[official versioned installer](https://docs.astral.sh/uv/getting-started/installation/)
if uv is missing from `PATH`. It then runs `uv sync --locked` and
`uv run --locked -- rv setup`. uv creates the project environment, downloads the
pinned Python when needed, and installs the locked dependencies. There is no
separate pip, virtualenv, wheel extraction, or executable-copying path.

An existing uv installation must satisfy `tool.uv.required-version` in
[`pyproject.toml`](../pyproject.toml). Setup does not replace a package-manager-owned
uv installation. Update it through the method that installed it if uv reports a
version mismatch. [`.uv-version`](../.uv-version) pins the first-install version;
keep both pins aligned when upgrading uv.

For Python-only development or initial bootstrap verification:

```console
./setup.sh --profile python
rv --help
```

`--no-launcher` skips the user-level executable installation. This is useful in
automation, where commands can run directly with `uv run --locked -- rv …`.
The full tooling test suite also uses native tools and Docker; see its
[prerequisites and fixture ownership](tests/README.md).

### Installation locations

The bootstrap uses the official `UV_INSTALL_DIR` option, defaulting to
`$HOME/.local/bin`, and adds that location to this invocation's `PATH`. The
upstream installer manages supported platforms, downloads, and shell integration.
Its `UV_NO_MODIFY_PATH` and `UV_UNMANAGED_INSTALL` options remain available; see
[uv's installer options](https://docs.astral.sh/uv/reference/installer/).

`rv setup` installs the small [launcher package](launcher/README.md) using
`uv tool install`. uv manages the launcher environment and entry point, including
refusing to overwrite an unrelated executable named `rv`. The launcher has no
runtime dependencies. Use `uv tool dir --bin` to find it, and
`uv tool update-shell` to make that directory available to future shells.

### Working in several checkouts

Run `rv` anywhere inside the intended Revaer checkout. The launcher stops at the
nearest Git repository boundary, including Git worktrees, then runs that
checkout's implementation through `uv run --locked`. It never falls back to the
checkout from which the launcher was installed.

Each checkout uses its own uv project environment and lockfile. Bootstrap and
the launcher discard inherited interpreter/project overrides that could redirect
execution into another checkout. uv cache, package-index, credential, and tool
storage settings remain available. No activation is needed.

## Architecture

```text
installed rv (stdlib-only launcher)
    -> uv run --locked in the selected worktree
        -> cli.py: parse arguments and construct dependencies
            -> COMMANDS[command](context)
                -> Task.run(context)
                    -> typed external-tool method
                        -> injected process runner
```

- **Dispatch:** [`cli.py`](src/revaer_tooling/cli.py) owns an explicit dictionary
  of command names and static `run` method references. There is no reflective
  discovery or plugin registry.
- **Tasks:** classes derive from [`Task`](src/revaer_tooling/tasks/base.py). Their
  static `run(context)` methods coordinate a complete Revaer operation. Larger
  operations call smaller task methods directly; a helper does not need a public
  CLI command.
- **Context:** [`context.py`](src/revaer_tooling/context.py) carries command inputs
  and collaborators. Environment reading and concrete dependency construction
  belong at the CLI boundary. Tasks receive what they need from their caller.
- **External tools:** adapters under [`external/`](src/revaer_tooling/external/)
  verify availability and version, accept typed inputs, and construct argument
  arrays. Tool options belong in the adapter, not in string templates in tasks.
- **Processes:** [`process.py`](src/revaer_tooling/process.py) handles execution,
  redaction, cancellation, and exit status. Commands do not use `shell=True`.
- **Packaging:** the checkout implementation uses the root `uv.lock`. The
  installed launcher only selects a checkout; it does not own its dependencies.

## Changing tooling

The [workflow integration guide](src/revaer_tooling/automation/README.md) covers
setup inputs, metadata outputs, chart version defaults, reports and documentation
deployment. CI uses the same locked task implementations as the local CLI.

The [media fixture guide](src/revaer_tooling/media/README.md) documents acquisition,
generation, snapshot review, conversion reports and cleanup for both the
foundation and ongoing media work.

The [development guide](src/revaer_tooling/development/README.md) covers watching,
restarts, database ownership, service logs and worktree-scoped process recovery.

1. Identify the existing command's inputs, defaults, output artifacts, failure
   behavior, and cleanup obligations in the migration inventory.
2. Put tool-specific argument construction in a typed adapter. Explain any
   non-obvious ordering, ownership, retry, or compatibility requirement in a
   comment beside the code.
3. Implement coordination in a task. Add a dictionary entry and argument parsing
   only if the operation makes sense as a public command.
4. Add tests for observable behavior and failure paths, then run the real tool
   where practical. A test that only repeats a method's arguments is insufficient
   evidence of parity.
5. Update the relevant README, instruction file, and
   [task ADR](../docs/adr/592-python-tooling.md). Describe dependencies before
   adding them. Preserve quality gates and existing behavior unless a change is
   explicitly approved.

Run the tooling checks with `rv tooling-check`, audit locked Python dependencies
with `rv tooling-audit`, and use `rv tooling-cov` to collect Python coverage. The
final migration also requires the full project CI and
browser suites on the foundation and a disposable checkout with the ongoing
media work integrated.

## Dependency updates

Use uv's project commands (`uv add`, `uv remove`, `uv lock`, `uv sync`) and review
the manifest and lockfile together. Never edit an installed environment or
manufacture a lockfile. Project tools run inside `uv run --locked`; `uvx` is not a
substitute for the project's locked dependencies.

### Dependency groups

The root project and lockfile own every Python dependency. The core package needs
only YAML parsing and JSON Schema validation. uv's dependency groups separate the
tools that a particular environment can use:

| Group | Purpose |
| --- | --- |
| `dev` | Formatting, static typing, dependency auditing, development watching/process inspection and PostgreSQL scanning. |
| `testing` | pytest, Playwright, coverage, and browser source parsing. |
| `release` | Python Semantic Release, the version and release-note engine. |

All three are default groups: ordinary `uv sync --locked`, `rv`, and CI setup
keep the complete development environment. A container stage that only runs core
tasks uses `uv sync --locked --no-default-groups` and
`uv run --locked --no-default-groups -- rv COMMAND`. uv still selects Python and
installs the same package from the same lockfile. A core environment cannot run
browser, test, or version-engine commands until their groups are installed.
Dependency auditing exports **all groups**, including in a core-only environment.
See [uv's dependency group documentation](https://docs.astral.sh/uv/concepts/projects/dependencies/#dependency-groups).

## Native prerequisites

`rv setup` uses Rustup and Cargo to install the configured compiler, components,
WebAssembly target, and tools from [`versions.toml`](versions.toml). Cargo keeps its
own installation receipts and skips matching versions/features. Setup installs
the Playwright browser builds associated with the locked Python package.

Install Rustup, Git, Docker, Helm, ORAS, GnuPG, and pkg-config through their
supported installation methods. The C++ bridge also needs the native libtorrent
development headers and libraries supported by the selected checkout. `rv doctor`
reports executable paths and versions and collects missing prerequisites into one
error. It verifies availability; the build gates establish native compatibility.
It does not promise that every executable earlier on PATH matches a Cargo pin.
The database tooling tests also require the PostgreSQL client commands `psql`
and `pg_isready`, supplied by PostgreSQL's client package.

The shared prerequisite baseline includes the media stack's libtorrent 2.1 ABI
support. The native build still verifies the selected headers, libraries, and
compiler together; see the [integration dependencies](migration.md#integration-dependencies)
for the source revision and acceptance status.

## Command families

- `setup`, `doctor`: installation and prerequisite diagnostics.
- `fmt`, `fmt-fix`, `tooling-check`, `tooling-cov`, `tooling-audit`: Python/Rust
  formatting and the Python quality checks.
- `policy`, `instruction-drift`, `lint`, `check`: source policy and compiler checks.
- `validate`, `ci`: ordered repository validation, followed by the release build
  for `ci`. `lock` and `release-lock` use uv's official project resolver.
- `test`, `test-native`, `test-features-min`, `cov`, `cov-report`,
  `db-start`, `db-migrate`, `db-reset`, `db-seed`:
  [Rust tests and database migrations](src/revaer_tooling/tasks/README.md).
- `build`, `build-release`, `sync-assets`, `check-assets`, `api-export`,
  `release-artifacts`, `sbom`, `licenses`: build outputs and artifact preparation.
  API export validates both the embedded input and generated output as OpenAPI 3
  with nonempty API paths. Resolve JSON merge conflicts before exporting; a
  successful generator process alone is insufficient artifact evidence.
- `db-rebaseline-freeze`, `db-rebaseline-candidate`, `db-init-prefix-check`,
  `db-init-finalize`: [reviewed database transition tooling](src/revaer_tooling/database/README.md).
- `ui-build`, `ui-serve`, `docs`, `docs-build`, `docs-serve`, `docs-index`,
  `docs-link-check`: UI and documentation workflows, with validation status
  recorded in the migration inventory.
- `ui-e2e`, `ui-e2e-coverage`, `ui-e2e-shard-coverage`: ordered application/browser
  tests, complete route checks, and verification of combined CI shard evidence;
  see the [browser guide](src/revaer_tooling/e2e/README.md).
- `runbook`: E2E execution, route gate, and artifact archival under one lock.
- `docker-build`, `docker-scan`, `sonar-compile-db`: local image output, scanning,
  and native analyzer inputs; see the [task guide](src/revaer_tooling/tasks/README.md).
- `script-coverage`: measured [root bootstrap behavior](src/revaer_tooling/bootstrap/README.md).
- `js-coverage-merge`, `python-coverage-merge`, `sonar-verify-inputs`, `sonar-prepare-sources`,
  `sonar-prepare-scm`, `sonar-scan`, `sonar-package-report`, `sonar-verify-result`:
  [Sonar coverage, installation, and evidence](src/revaer_tooling/sonar/README.md).
- `release preview`, `release publish`, `release resume`, and `helm-*`:
  [release and chart operations](../release/README.md).

Use `rv COMMAND --help` for arguments. The registry contains the current command
surface; the [recipe map](migration.md#recipe-mapping) also lists operations that
still need migration. [The testing README](tests/README.md) describes the evidence
and limits of each fixture family.

### Chart lint behavior

Chart packaging invokes Helm with `--strict`, preserving the media packaging
contract: a Helm warning fails before a package replaces the output directory.
The native regression fixture succeeds without strict mode and is rejected by
the packaging task, so malformed-chart failures cannot stand in for this check.

For the media chart, lint receives temporary image digest, architecture and
compliance-volume identities through a private values file outside the chart.
Those synthetic inputs never enter the package: deployment defaults still
require the real image and prepared compliance volume.

Release annotations replace exactly one standalone
`# __RELEASE_HELM_ANNOTATIONS__` comment. Missing, embedded or duplicate markers
fail before replacing existing packages. Neighboring authored annotations and
multiline image/signing metadata are preserved.

`rv helm-lint` runs `rv helm-annotation-test`, the media compliance checks when
applicable, and `rv helm-package-test` before packaging an unsigned chart. Each
prerequisite stops the process on failure. Package regression tests call the
package operation directly, so they cannot recursively invoke the lint command.


### Documentation tasks

`rv docs-install` installs the pinned mdBook and Mermaid tools through Cargo's
native installer, then asks Mermaid to install its book integration. `rv docs`
runs installation, book build and release-mode indexing in order. `rv docs-build`
and `rv docs-serve` verify the installed pins; serve retains browser opening.
`rv docs-link-check` installs the pinned Lychee checker and fails on broken links.
Versions have one source in `tools/versions.toml`.

### License reports

`rv licenses` first ensures the repository-pinned `cargo-deny` installation, then
uses its native JSON license report. Standalone reporting does not depend on an
earlier full setup. The report is inventory evidence; dependency audit and deny
remain separate required gates.

### Disposable single-init databases

`rv db-test-init NAME` and `rv db-test-drop NAME` support the media checkout's
single initializer. Select an existing disposable PostgreSQL service with
`PG_CONTAINER`, `REVAER_LOCAL_DB_USER`, and `REVAER_LOCAL_DB_PASSWORD`.
Initialization also requires `REVAER_TEST_RUNTIME_PASSWORD` containing 64 lowercase
hexadecimal characters. Credentials travel through process environment inputs.

Names must match `revaer_test_NUMBER_NUMBER` and contain at most 50 characters.
The container must match `POSTGRES_REBASELINE_IMAGE` in the checkout's build
manifest. Initialization refuses database, role, and staging-directory collisions;
it applies and seals the exact initializer transactionally, then disables the
owner login. Tests connect through the restricted `NAME_runtime` role. Drop
verifies database ownership before removing the database and its two roles.
Failures clean up only resources created by that invocation. These commands do
not initialize or reset a caller-owned development database.

### Application E2E bootstrap regressions

`rv ui-e2e-app-test` runs the existing launch-guard library tests, compliance
library tests, and `bootstrap` integration binary in that order. It preserves
Cargo's default features and the two library groups' single-threaded execution.
Supply `REVAER_TEST_DATABASE_URL` (or `DATABASE_URL`) for a disposable PostgreSQL
service; the command never resets a developer database. This command exercises
app bootstrap regressions; `rv ui-e2e` remains the real browser/API suite.

Each group must execute at least one passing test with no failures or ignored
tests. A renamed filter that matches zero tests fails closed. Native Cargo
fixtures verify selection, feature flags, environment propagation, failures,
and missing groups. Actual application qualification is recorded separately.

### Media filesystem tests

Run media root-catalog checks from a checkout whose ancestors are owned by the
current user or root and are not group/world writable. The descriptor fixtures
use the checkout directory and intentionally enforce the production ancestry
checks. A checkout under `/tmp` or `/private/tmp` fails these checks even when
the checkout itself has restrictive permissions. Use a normal checkout beneath
your protected source directory; do not change shared temporary-directory
permissions or relax the ancestry validator. Linux-only mount and filesystem
cases still need a Linux validation run.

### Database test scheduling

Single-init tests create complete schemas in isolated databases. Many simultaneous
initializations can exhaust a default PostgreSQL server's shared lock memory.
For a constrained local test service, select serial Rust test scheduling with
`RUST_TEST_THREADS=1 uv run --locked -- rv test`, using your explicit disposable
test database configuration. This runs the same complete test selection and
retains concurrency exercised inside individual tests. It does not disable
statement timeouts or replace production baseline checks.
