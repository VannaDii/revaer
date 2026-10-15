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
sccache has a separate rolling cache keyed by the build inputs and producer, with
compatible cross-job restore keys. Cache hits never replace tests or coverage.

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
unrelated tests. `.config/nextest.toml` keeps native libtorrent sessions serial,
bounds database-heavy work to two processes, and admits two independent recovery
scenarios. Each scenario keeps its own ordered restart/fault steps and private
fixture. Pure tests use the runner's available CPUs. Existing Cargo doctest and
feature gates remain required.

Coverage collections keep one workspace/all-features selection, including ignored
recovery tests, so feature unification does not force a second package build.
The engine clears profiles and workspace artifacts before collection; the rolling
instrumented-target cache retains compatible dependency builds. No execution data
from a cache can satisfy a test or coverage gate. Cargo determines compiler
parallelism unless the operator supplies `CARGO_BUILD_JOBS`. Python tooling tests
use two isolated workers with file-level scheduling and Coverage.py merging.
