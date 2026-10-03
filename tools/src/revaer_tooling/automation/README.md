# Workflow integration

Workflow behavior stays in the same Python package as local development tasks.
GitHub Actions handles triggers, dependency graphs, credentials and artifact
transport. `rv` handles executable project behavior.

## Commands

| Command | Inputs and result |
| --- | --- |
| `workflow-metadata` | Resolves actual Git HEAD and emits `sha` and `short_sha`. |
| `workflow-matrix` | Validates the committed image matrix and emits its compact JSON plus source identity. `--release-only` selects main or a stable version tag from `GITHUB_REF_TYPE` and `GITHUB_REF_NAME`. |
| `workflow-chart-versions` | Accepts `--chart-version`, `--app-version`, and `--pr-number`. Otherwise queries the selected branch's single open PR through `gh`. The default chart is `0.0.0-dev.pr<PR>.<GITHUB_RUN_NUMBER>`; the application tag includes the PR and actual short SHA. |
| `workflow-report PATH` | Displays an owned, nonempty report and appends it to `GITHUB_STEP_SUMMARY` when present. |
| `verify-supply-chain-results` | Requires `AUDIT_RESULT`, `DENY_RESULT`, and `UDEPS_RESULT` to each be exactly `success`. Missing, cancelled, skipped and failed jobs fail the aggregate gate. |
| `docs-guard` | Preserves the documentation secret-pattern and committed-file-type checks. Potential secret values are never echoed. |
| `docs-prepare` | Assembles the built book and three LLM manifests in `deploy/`, copies an optional `docs/CNAME`, and records UTC time, actual source commit and runner identity. |

Output commands print JSON locally and write the same named values to
`GITHUB_OUTPUT` when the runner supplies that file. They do not change releases,
registries, pull requests, or the Pages branch. Chart publication remains an
explicit `helm-package` / `helm-publish` sequence.

Manual chart overrides are validated before querying GitHub. Helm versions
follow [SemVer 2.0](https://semver.org/spec/v2.0.0.html), including numeric
identifier rules; application versions retain the existing ASCII character set.
A missing PR requires an explicit chart version. Multiple matching PRs and API
errors fail instead of selecting an arbitrary PR or treating an error as absence.
`gh api --method GET --raw-field` owns query encoding and authentication; see the
[official CLI interface](https://cli.github.com/manual/gh_api).

## Documentation deployment

Run `rv docs` to build the book and manifests, then `rv docs-prepare` to inspect
the complete deployment payload locally. Preparation validates producers before
replacing the previous payload and removes stale book files. It rejects linked
inputs, tracked deployment output, and an existing `deploy/` directory without a
deployment record. A copy failure preserves the previous payload.

The build and serve tasks verify the mdBook/mermaid versions in
`tools/versions.toml` before running the preprocessor. If another installation
shadows the Cargo-installed pin, correct PATH before retrying; the CLI does not
replace system packages or silence the protocol warning. For a normal Rustup
installation, its supported `~/.cargo/env` file supplies the Cargo binary path.

The docs workflow continues to publish through its pinned Pages action to
`gh-pages`, with the same main/tag/manual triggers. Metadata comes from Python's
injected invocation time and Git HEAD; no date expression is embedded in the
publication message. Building and preparing locally do not publish the site.

## Environment and output files

`files.write_values` writes GitHub's documented multiline protocol for both
environment variables and step outputs. It validates every entry before writing,
retains earlier values, and treats newlines, quotes and shell-looking text as
data. Command files must be regular files; symlinks and FIFOs fail promptly.

The CLI boundary loads runner-provided file paths into immutable settings.
Release completion outputs are written only after all expected remote artifacts
have been verified. Setup exports sccache settings only after installation and
the executable probe succeed.

## Setup

The [composite action](../../../../.github/actions/setup-revaer/README.md) documents
the supported inputs. It calls the normal setup command with explicit Cargo and
browser selections. The [bootstrap package](../bootstrap/README.md) resolves those
selections before running installers. Local `rv setup` retains the complete
development-tool default.

Tests use fixture-owned command files and simulated privilege boundaries. A real
Debian/uv container separately verifies apt installation, repeat execution,
invalid input, missing-package failure and subsequent recovery. No test installs
apt packages on the developer's host.
