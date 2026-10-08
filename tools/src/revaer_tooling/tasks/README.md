# Task implementation guide

Tasks coordinate Revaer operations through an injected `Context`. Every public
command maps to a static `Task.run(context)` method in the CLI's explicit
dictionary. Internal steps are ordinary calls; add a public command only when it
is useful on its own.

The [database transition guide](../database/README.md) covers immutable migration
inputs, candidate replay and initializer generation. The
[pristine catalog guide](../database/pristine/README.md) describes the independent
catalog reference and constrained-owner mutation suite. Both use recorded,
disposable resource ownership and preserve failures through cleanup.

For media operations, see the [fixture architecture and commands](../media/README.md).
The task classes own operation locks and report publication; catalog validation,
generation and probe comparison are reusable helpers. Native curl/FFmpeg/Cargo
arguments belong to their typed adapters. The conversion task uses Cargo metadata
to preserve the foundation's preparation-only contract and the media branch's
actual ignored integration suite.

## Ownership and failure handling

- Resolve environment settings and construct collaborators in `cli.py`.
- Put external command arguments in the corresponding typed tool adapter.
- Use the process runner for redaction, streaming, exit status, and cancellation.
- Name the resource owner before adding cleanup. Never terminate processes or
  remove data merely because their names resemble Revaer resources.
- Preserve errors from failed tools. A later cleanup or reporting step must not
  turn an incomplete operation into success.
- Verify generated artifacts and update the relevant README and migration row
  before calling a replacement equivalent to an existing recipe.

## Rust tests

`rv validate` prepares the selected database, holds its operation lock, and runs
the ordered gates in [`quality.py`](quality.py). It carries the normalized
connection to every gate and preserves an explicit test database override.
`rv ci` adds the release build after validation succeeds. Both stop on the first
failure. Their full repository acceptance is tracked in the migration inventory.

`rv lock` (also `rv release-lock`) invokes the official `uv lock` resolver.
Review the resulting manifest and lockfile diff together; ordinary command runs
continue to use `uv run --locked` and do not update the graph.

| Command | Scope |
| --- | --- |
| `rv test` | Every workspace package with all features. |
| `rv test-native` | `revaer-torrent-libt` with all features, `REVAER_NATIVE_IT=1`, and one test thread. |
| `rv test-media-service-recovery` | One explicit Linux service restart with active FFmpeg on an owned persistent mount. |
| `rv test-features-min` | `revaer-api`, then `revaer-app`, each with default features disabled. |

All variants use the locked Cargo graph and treat compiler warnings as errors.
An unsuccessful Cargo test is an unsuccessful `rv` command. Existing Cargo and
Rust test environment settings pass through. The native variant serializes its
tests as required by the media stack; other variants retain the caller's test
thread setting.

`REVAER_TEST_DATABASE_URL` selects the administrative test connection. An explicit
`DATABASE_URL` selects the application connection; otherwise it uses the test
connection. A supplied `DATABASE_URL` also serves as the test fallback. With
neither set, test commands stop before launching Cargo and ask for a disposable
database. This retains the active media stack's ownership requirement.

These targeted commands use an available database. Use `rv db-start` to prepare
a development database, or pass the disposable database supplied by CI. The
combined CI/E2E lifecycle is still being migrated; its status is tracked in the
[inventory](../../../migration.md).

## Runtime shutdown gates

`rv test-runtime-shutdown` runs `revaer-app` library tests selected by
`bootstrap::shutdown_tests`, first with all features and then with default
features disabled. It preserves shown test output and warnings-as-errors. Each
mode must execute at least one passing test, with no failures or ignored tests;
a missing module cannot become a successful zero-test run. These tests do not
provision a database.

`rv lint-runtime-shutdown` runs both Clippy passes (all targets and production
library/binary/example targets) in both feature modes. The typed Cargo adapter
owns package and feature arguments; the task only composes the four passes.
Both commands stop on the first failure. Native fixture regressions establish
selection, execution and failure behavior; the media application itself must
still pass in the disposable integration checkout.

## Runbook evidence

`rv runbook` runs the ordered E2E phases, checks operation and route coverage,
then archives logs, HTML reports, and test results under `artifacts/runbook`.
The E2E lock remains held until the archive finishes. Failures and interruptions
still archive the available evidence and preserve a failing exit status.

`summary.txt` is written last and reports `runbook=ok` or `runbook=failed`.
An archive failure removes the old summary, so a stale success cannot represent
an incomplete new archive. The archive excludes the legacy credential state file
and refuses symbolic links. See the [E2E guide](../e2e/README.md) for overrides
and the [scenario map](../../../../tests/MIGRATION.md) for preserved behavior.

## Local containers

`rv docker-build` uses Docker Buildx. One platform loads the image into Docker;
multiple platforms produce `artifacts/revaer-VERSION.oci`. Both retain Buildx's
digest metadata in `artifacts/image-build.json`. A failed build removes partial
outputs. The builder is named explicitly and never becomes the global selection.

| Override | Default |
| --- | --- |
| `PLATFORMS` | `linux/amd64,linux/arm64`; comma-separated OS/architecture[/variant] values. |
| `VERSION` | `dev.YYMMDD.COMMIT`, using the invocation's UTC date and current commit. |
| `BUILDX_BUILDER` | `revaer-builder`; Buildx creates it if absent. |
| `REVAER_LOCAL_IMAGE` | `revaer`; use a distinct name for parallel worktrees. |
| `REVAER_SCAN_IMAGE` | `revaer:ci`; selects the image for `rv docker-scan`. |

Builds retain the `latest` and version tags. A custom image name also names its
OCI archive. Docker owns builder caches and storage; rv does not prune them.

`rv docker-scan` invokes Trivy with the repository's `trivy.yaml`, the existing
HIGH/CRITICAL gate, and unfixed findings included. It retains
`artifacts/image-scan.json` on findings and propagates Trivy's nonzero exit.
An empty explicit ignore file prevents a local advisory-ignore file from silently
changing this gate. Image release signing and multi-architecture publication are
separate command families in the migration inventory.

## Native scanner inputs

`script-coverage` exercises the unchanged root bootstrap with the official uv
installer and native kcov. Its [guide](../bootstrap/README.md) documents the
platform prerequisites, descriptor ownership, raw evidence and pending legacy
acceptance-check correction.

Policy tasks share the validators described in the
[policy guide](../policy/README.md). Direct audit, deny and scanner entry points
enforce their relevant contracts as well as the full `rv policy` composition.
`js-coverage-merge` holds the E2E lock while binding the complete parsed source
inventory to actual Chromium execution. See the
[browser coverage guide](../e2e/README.md#javascript-source-coverage) before changing
source locations, counter conversion, or page cleanup.

`rv sonar-compile-db` builds the libtorrent bridge with native integration enabled
into `target/sonar-build`. The Rust build script supplies actual compiler flags
and definitions. rv validates the resulting Clang compilation database and
publishes `coverage/compile_commands.json` only when it includes the current
checkout's bridge. It does not synthesize compiler commands or change Sonar scope.

Each invocation gives the build script a fresh output filename, one of its declared
Cargo rebuild inputs. The parent remains `coverage/`: the build script stages its
headers under `coverage/cxxbridge/include`, and those paths must remain usable
after rv returns. Both generated headers and the compiler's include arguments are
verified before publication. Repeating the command regenerates a removed report
without deleting the build cache. Failure invalidates the old published file.
This command supplies analyzer inputs; it does not run or replace the Sonar gate.

## Rust and native coverage

`rv cov` cleans the coverage engine's workspace artifacts, runs all workspace
features with C++ instrumentation and native integration enabled, and calls
`rv cov-report`. It requires the
same explicit disposable database selection as Rust tests; database provisioning
is a separate lifecycle concern. The default is one build job and one test
thread, with `CARGO_BUILD_JOBS` and `RUST_TEST_THREADS` overrides preserved.
Cargo's negative CPU offsets and `default` job setting remain available.

The coverage adapter uses the pinned cargo-llvm-cov and the selected Rust
compiler's `llvm-tools-preview` and `rust-src`. Set `CC`, `CXX`, `LLVM_COV`, and
`LLVM_PROFDATA` when selecting other compatible native tools. Versioned
`clang-19`/`clang++-19` on PATH are preferred to unversioned Clang, matching the
media tooling. On macOS, a keg-only LLVM installation needs explicit `CC`/`CXX`
paths. Analysis never installs components. See the engine's
[native compatibility guidance](https://github.com/taiki-e/cargo-llvm-cov#get-coverage-of-cc-code-linked-to-rust-librarybinary).

| Output | Purpose |
| --- | --- |
| `coverage/toolchain.txt` | Rust host/LLVM details and the four selected native tools and versions. |
| `coverage/crates/*.json` | Complete per-package engine measurements; the existing 90% Rust line gate uses their exact counts. Cargo expands workspace member globs. |
| `coverage/rust-sources.rsp` | LLVM response file mapping compiler source paths to the matching installed `rust-src`, including paths with spaces/quotes. |
| `coverage/lcov.info` | Complete source/line records for downstream analysis. |
| `coverage/llvm-cov.txt` | Text evidence, including native sources. |
| `coverage/html/index.html` | Browsable complete report. |

The three complete reports disable the engine's default filename filtering and
include build scripts. No authored C++ file is removed from these reports.
Per-crate summaries retain C++ measurements as well. The pre-migration numeric
threshold measured Rust, so it still checks Rust counts at 90%; it does not
silently become a new combined Rust/C++ threshold. No files are removed from the
published reports. Sonar's independent requirements continue to apply.

Rust embeds its compiler commit in standard-library source paths. LLVM's supported
`-path-equivalence` option maps those paths to the matching source component.
The response file keeps quoting intact across cargo-llvm-cov's space-separated
flag interface. Missing sources fail with setup guidance instead of producing
an incomplete HTML report or hiding the source.

Threshold failures retain the reports and fail the command. A new collection
invalidates its old reports before compiling so a compiler/test failure cannot
leave stale success evidence. Other producers' files, such as `python.xml`, are
preserved. `rv cov-report` regenerates reports from existing engine profiles and
does not rerun tests; use it after additional instrumented runs against the same
source/build inputs.

## Database lifecycle

| Command | Behavior |
| --- | --- |
| `rv db-start` | Start or reuse the selected database, create the application database if absent, and apply migrations. |
| `rv db-migrate` | Apply pending migrations to an available database. |
| `rv db-reset` | Verify managed ownership, then ask SQLx to drop/recreate the application database and apply migrations. |
| `rv db-seed` | Run the start/migration sequence, then apply the development seed in one transaction. |

With no supplied connection URL, database commands use the local `revaer`
application database and `db-start` manages a PostgreSQL 16 container. With a
supplied `DATABASE_URL` or `REVAER_TEST_DATABASE_URL`, lifecycle ownership defaults
to the caller: `db-start` checks readiness and runs SQLx against that connection.
It does not create or replace a Docker container in that mode.

| Setting | Meaning |
| --- | --- |
| `DATABASE_URL` | Application connection, with precedence over the test URL. |
| `REVAER_TEST_DATABASE_URL` | Administrative test connection and application fallback. |
| `REVAER_DB_MANAGED` | `auto` by default; explicit `1`/`true` selects local managed ownership, `0`/`false` keeps caller ownership. |
| `REVAER_LOCAL_DB_USER`, `REVAER_LOCAL_DB_PASSWORD` | Credentials for constructing the default local URL. |
| `REVAER_LOCAL_DB_HOST`, `REVAER_LOCAL_DB_PORT` | Default local address and port; use a different port for simultaneous worktrees. |
| `PG_CONTAINER` | Container name override; the default includes a hash of the checkout path. |
| `REVAER_DB_DATA_DIR` | Persistent bind directory; defaults to `.server_root/postgres-data`. |
| `REVAER_DB_SHM_SIZE` | Docker shared-memory size, default `1g`; bytes and integer `k`/`m`/`g` suffixes are accepted. |
| `REVAER_DB_SHM_BYTES` | Minimum permitted shared-memory bytes, default 1 GiB. |
| `REVAER_DB_RESET` | `1` requests reset during `db-start`; caller-owned databases remain protected. |

Managed mode requires local host coordinates and explicit credentials/database
in its URI. It rejects query parameters that change the server identity. Docker
publishes only on loopback. For calls made inside Docker, readiness can try the
host's Docker alias; a server-side ownership check must still match the checkout
before SQLx runs. Caller-owned URLs are used as supplied.

### Ownership and persistence

Each managed bind directory contains a private `.rv-owner.json` record and a
local operation lock. PostgreSQL owns its `pgdata/` child through the official
image's `PGDATA` setting. The image runs with the host file owner's UID/GID and a
read-only root filesystem; its runtime scratch directories are temporary mounts.

The container label, directory record, and PostgreSQL server identity must match
the checkout. Matching names alone never authorize adoption, reset, or removal.
Changing an owned container's port/storage/shared-memory configuration can
recreate that container while preserving its data. A manually removed container
can be restored from the same recorded bind directory. Two operations on the
same directory cannot migrate or reset it concurrently.

Old unlabelled containers and old data directories are not adopted automatically.
During this migration, select a fresh `REVAER_DB_DATA_DIR` and an available local
port to use the new managed lifecycle alongside an existing database. The old
data remains available to its existing owner. Moving or sharing a directory
between checkouts requires an explicit ownership migration outside these commands.

### Migrations and failures

`rv db-migrate` invokes SQLx against `crates/revaer-data/migrations` using the
selected `DATABASE_URL`, falling back to the test URL. SQLx reads the existing
migration history and runs pending migrations.
It receives the selected URL through its environment and disables its separate
dotenv lookup so a nested invocation cannot silently choose a different database.
All application operations now share the media stack's local `revaer` default;
this removes the foundation recipe's inconsistent `postgres` migration default.
Credentials containing URL punctuation are encoded and omitted from settings
representations and process logs.

Replay is idempotent. A changed migration checksum, invalid SQL, or unavailable
database fails the command. The migration task never responds to an error by
resetting the database. Use `rv db-reset` explicitly when discarding managed
application data is intended. PostgreSQL's administrative/template databases
cannot be reset by these commands. Seed failures roll back their transaction.
This replaces the old recipes' automatic reset on migration failure with an
explicit operation, while retaining SQLx's existing reset implementation.

Database tests use actual Docker, PostgreSQL, SQLx, and libpq clients. They verify
ownership mismatches, idempotent start/seed, checksum rejection, transactional
rollback, explicit reset, and data preservation during container recreation.

## Chart compliance regression

`rv compliance-chart-test` runs the Python native Helm suite against this
checkout's chart. It requires the media chart's digest, architecture and compliance
binding API. `rv helm-lint` composes this check when that API is present, before
packaging. The standalone command does not silently fall back to a fixture.

The suite preserves independent schema-only and template-only rejection, empty
stdout for invalid inputs, immutable image/manifest storage bindings, retained
pod/security/health configuration, and ordinary config/data resources. It uses
private temporary values and never installs into Kubernetes. The full tooling
suite additionally checks the [recorded media chart](../../../tests/fixtures/media-chart/README.md).

## Other families

- [Database transition](../database/README.md): frozen corpus verification,
  native candidate replay, exact initializer deltas and owned proof storage.
- [Development](../development/README.md): native file notifications, API
  restarts, Trunk serving and checkout-scoped recovery through `rv zombies`.

The [workflow integration guide](../automation/README.md) describes metadata,
matrix selection, PR chart defaults, supply-chain aggregation, report files and
documentation deployment. These are ordinary static tasks with injected tools.
Their workflow callers contain single CLI invocations and declarative transport.

The Python E2E runner's architecture, configuration, and migration status are
documented in the [browser support guide](../e2e/README.md). Its transport and
service ownership checks are verified. The full foundation application E2E and
route gates pass; media integration remains pending.

`rv udeps` checks the exact cargo-udeps and nightly compiler pins from
`tools/versions.toml`, records `target/udeps-toolchain-evidence.txt`, and runs the
workspace/all-targets analysis. Conflicting `REVAER_UDEPS_VERSION` or
`REVAER_UDEPS_TOOLCHAIN` values are errors. Install the selected compiler through
`rv setup`; analysis does not install tools or silently fall back to another
compiler.

- [`build.py`](build.py): build outputs, assets, documentation, dependency audits,
  and the Python quality commands.
- [`policy.py`](policy.py): source/workflow policy and instruction drift.
- [`setup.py`](setup.py): official package-manager installation and diagnostics.
- [`automation.py`](automation.py), [`docs.py`](docs.py): workflow metadata,
  report handling and the documentation deployment payload.
- [`compliance.py`](compliance.py): independent [image evidence gates](../images/README.md).
- [`release.py`](release.py), [`charts.py`](charts.py): release coordination and
  chart operations; see the [release guide](../../../../release/README.md).

Supplying `REVAER_NATIVE_RECOVERY_ROOT` to `rv cov` selects an owned persistent Linux mount and additionally executes the existing native service-recovery scenario under coverage instrumentation. The ordinary workspace run and its profiles remain part of the resulting reports. An unsupported mount fails the scenario.
