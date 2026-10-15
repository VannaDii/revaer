# Revaer workflow setup

This composite action uses the official `astral-sh/setup-uv` action, synchronizes
the locked project, and invokes `rv setup`. Workflow steps then use
`uv run --locked -- rv COMMAND`. The action contains no authored installation
scripts, embedded Python programs, or JavaScript programs.

## Example

```yaml
- uses: ./.github/actions/setup-revaer
  timeout-minutes: 20
  with:
    apt-profile: db
    cargo-tools: sqlx-cli,trunk
    browsers: chromium
- name: Browser tests
  run: uv run --locked -- rv ui-e2e
```

Use `profile: python` for jobs that only need the Python tooling, including
workflow metadata, policy aggregation and chart operations. The default `ci`
profile installs the Rust toolchain and components from `rust-toolchain.toml`.
Unlike the full local setup default, this action explicitly starts with empty
Cargo and browser selections. Each job declares its additional requirements.

## Inputs

| Input | Meaning |
| --- | --- |
| `profile` | `ci` or `python`; passed to the same local setup command. |
| `cargo-tools` | Comma-separated package names from `tools/versions.toml`. Versions and features remain manifest-owned. |
| `browsers` | Comma-separated `chromium`, `firefox`, or `webkit`. CI also installs Playwright's supported OS dependencies. |
| `apt-profile` | `base`, `db`, `coverage`, or `media`; omitted means no apt installation. The coverage profile includes FFmpeg because the Python tooling suite executes native fixture recipes. |
| `apt-packages` | Whitespace-separated package names, overriding the predefined apt selection. Architecture/version-qualified names are supported. |
| `cargo-cache` | Enable the shared Cargo downloads, installed-tool receipts and sccache cache. |
| `compiler-family` | `stable` (default) or `nightly`; separates compiler snapshots while preserving cross-job reuse. |
| `compiler-cache` | Restore compiler results (default `true`); disable for binary-only audit jobs. Formatting disables all Cargo caches. |
| `coverage-cache` | Restore instrumented dependencies; enable only for Rust coverage producers. |
| `helm`, `oras`, `trivy` | Install the reviewed native CLI through its upstream setup action. |
| `sonar-scanner` | Install the pinned, signature-verified scanner and retain its analyzer cache. |

Coverage setup also installs the pinned kcov source build. All external actions
use full commit hashes. The reviewed Helm, ORAS and Trivy versions remain explicit
in the action; Cargo, kcov and scanner pins live in the tooling manifest.

## Ownership and failures

uv reads `tool.uv.required-version` from `pyproject.toml`, uses `.python-version`,
and owns dependency and Python caches. `rv setup` additionally checks `.uv-version`
and the lockfile. CI invokes the checkout's CLI through uv, so it does not need a
second installed launcher environment.

The typed apt adapter validates package tokens before invocation, preserves
repository signature verification, and stops if repository refresh fails. Root
containers invoke apt directly; hosted runners use noninteractive sudo. Installed
sccache settings go through GitHub's multiline environment-file protocol.
User input reaches CLI arguments through quoted environment variables.

Cargo owns installed-package receipts, version replacement and feature tracking.
Matching installations are retained. Kcov/scanner caches are verified before reuse;
a cache hit cannot substitute for archive identity or required analyzer inputs.

Cargo downloads are shared across jobs. Each pinned tool has its own installation
prefix and cache, including Cargo's native receipts. Cache keys include the version
manifest and Rust toolchain, but never depend on the other tools selected by a job.
Setup supplies sccache settings to the current installers and subsequent steps.
Validated Cargo and kcov installations are saved during setup, so later test
failures do not discard reusable tools. Cache hits retain their native receipts.
Build-cache consumers restore only. The PR check job and main release job publish
stable compiler snapshots; unused-dependency analysis publishes nightly snapshots.
Rust coverage producers publish instrumented dependencies. The shared publisher
action runs only after a successful producer and uses the setup action's returned
key and prefix, so a consumer cannot create another copy of the same snapshot.

Compatible keys include OS/architecture, Cargo lock and authored manifests,
`.cargo` target configuration, Rust/tool pins and native build inputs. Compiler
snapshots use a 512 MiB local sccache limit. Publication keeps at most two snapshots
and 1 GiB per family/ref using `rv workflow-cache-prune`; it lists every provider
page and deletes only exact matching IDs within the current workflow ref.
An older overlapping publisher does not prune a newer publisher's snapshots.
The three families therefore use at most 3 GiB per active ref after publication,
leaving room for tools, downloads and Python caches under the repository limit.
Several active refs still share that limit; GitHub may evict caches. No settings
or paid storage changes are required.

Instrumented dependency archives contain dependency libraries, build-script
outputs and Cargo fingerprints, excluding workspace crate outputs. Test binaries,
raw execution profiles, coverage results, databases and media fixtures are not
cached. Cache hits never replace tests or coverage; Cargo and sccache validate
the actual source, flags and compiler inputs before reusing compiled results.

The setup and output-file regression suite includes a real Debian container using
Astral's official uv image. Its apt changes belong only to that disposable
container. Hosted workflow execution and full application acceptance remain
separate checks in the [migration inventory](../../../tools/migration.md).

## Upstream interfaces

- [uv in GitHub Actions](https://docs.astral.sh/uv/guides/integration/github/)
- [setup-uv inputs](https://github.com/astral-sh/setup-uv/tree/v9.0.0)
- [uv in containers](https://docs.astral.sh/uv/guides/integration/docker/)
- [GitHub environment files](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#environment-files)

## Test concurrency

Pinned cargo-nextest supplies process-per-test isolation and resource groups;
this avoids an authored test executor or an in-process environment lock across
unrelated tests. `.config/nextest.toml` keeps native libtorrent sessions and shared bootstrap engine state serial,
bounds database-heavy work to two processes, and admits two independent recovery
scenarios. Each scenario keeps its own ordered restart/fault steps and private
database, engine paths/listener, media roots and workspace. Pure tests use the runner's available CPUs. Existing Cargo doctest and
feature gates remain required.

Coverage collections keep one workspace/all-features selection, including ignored
recovery tests, so feature unification does not force a second package build.
The engine clears profiles and workspace artifacts before collection; the rolling
instrumented-target cache retains compatible dependency builds. No execution data
from a cache can satisfy a test or coverage gate. Cargo determines compiler
parallelism unless the operator supplies `CARGO_BUILD_JOBS`. Python tooling tests
use two isolated workers with file-level scheduling and Coverage.py merging.
